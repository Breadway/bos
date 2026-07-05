use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GBox, Orientation, Overlay, Stack, StackTransitionType};

use crate::cli::Action;
use crate::config::State;
use crate::content::{keybinds, ContentStore};

use super::home::Home;
use super::onboarding::{self, Onboarding};
use super::{ask, learn, modes, tabs};

const DEFAULT_TAB: &str = "home";

struct Handle {
    window: ApplicationWindow,
    onboarding: Onboarding,
    home: Home,
}

thread_local! {
    static HANDLE: RefCell<Option<Handle>> = const { RefCell::new(None) };
}

/// Called from every `command_line` invocation — including ones forwarded
/// over D-Bus to this primary instance by a second `breadhelp` launch — so
/// it must build the window at most once and reuse it thereafter.
pub fn present(app: &Application, action: Action) {
    HANDLE.with(|cell| {
        let mut just_built = false;
        {
            let mut cell_ref = cell.borrow_mut();
            if cell_ref.is_none() {
                *cell_ref = Some(build(app));
                just_built = true;
            }
        }
        let cell_ref = cell.borrow();
        let handle = cell_ref.as_ref().unwrap();

        if action.force_onboard {
            let mut state = State::load();
            state.set_onboarding_completed(false);
            state.set_onboarding_step(0);
            handle.onboarding.goto(0);
            handle.onboarding.root.set_visible(true);
            handle.window.present();
            return;
        }

        if let Some(id) = &action.suggest {
            if let Some(s) = crate::services::breadd::resolve(id) {
                handle.home.set_suggestion(Some(&s.text));
            }
            // Only focus the window the first time this process builds it
            // (i.e. breadhelp wasn't already running) — a background daemon
            // event shouldn't steal focus from whatever the user is doing.
            if just_built {
                handle.window.present();
            }
            return;
        }

        // Every-login autostart builds the window (so the app is ready to
        // respond to SUPER+/ instantly) but must only pop it open on a
        // genuine first run — not on every subsequent login.
        let silent_autostart = action.autostart && State::load().onboarding_completed();
        if !silent_autostart {
            handle.window.present();
        }
    });
}

fn build(app: &Application) -> Handle {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("BOS Help")
        .default_width(880)
        .default_height(600)
        .build();

    crate::theme::load(&WidgetExt::display(&window));

    let store = Rc::new(ContentStore::load());
    let binds = Rc::new(keybinds::load());
    let state = State::load();

    modes::apply(&window, state.mode());

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(StackTransitionType::SlideLeftRight);

    let home = super::home::build(&store, &binds, &window, state.mode(), {
        let window = window.clone();
        move |mode| {
            modes::apply(&window, mode);
            let mut state = State::load();
            state.set_mode(mode);
        }
    });
    stack.add_titled(&home.root, Some("home"), "Home");
    stack.add_titled(&learn::build(&store, &binds, state.mode()), Some("learn"), "Learn");
    stack.add_titled(&ask::build(store.clone(), binds.clone(), state.mode()), Some("ask"), "Ask");
    stack.set_visible_child_name(DEFAULT_TAB);

    let switcher = tabs::build(&stack);

    let content_vbox = GBox::new(Orientation::Vertical, 0);
    content_vbox.append(&switcher);
    content_vbox.append(&stack);

    let overlay = Overlay::new();
    overlay.set_child(Some(&content_vbox));

    // The on_finish closure needs to hide the onboarding widget once it's
    // built, but the widget doesn't exist until `onboarding::build` returns —
    // so it closes over a cell filled in right after.
    let hide_target: Rc<RefCell<Option<GBox>>> = Rc::new(RefCell::new(None));
    let hide_target_for_closure = hide_target.clone();
    let ob = onboarding::build(move || {
        if let Some(w) = hide_target_for_closure.borrow().as_ref() {
            w.set_visible(false);
        }
    });
    *hide_target.borrow_mut() = Some(ob.root.clone());

    ob.root.set_visible(!state.onboarding_completed());
    overlay.add_overlay(&ob.root);

    window.set_child(Some(&overlay));
    // Deliberately not presented here — `present()` (the caller) decides
    // whether this initial build should actually be shown (see
    // `silent_autostart` above), so a from-cold every-login autostart with
    // onboarding already complete builds a ready-but-hidden window instead
    // of flashing it open.

    Handle { window, onboarding: ob, home }
}
