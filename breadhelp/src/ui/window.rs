use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GBox, Orientation, Stack, StackTransitionType};

use crate::cli::Action;
use crate::config::State;
use crate::content::{keybinds, ContentStore};

use super::home::Home;
use super::{ask, learn, modes, tabs, tour};

const DEFAULT_TAB: &str = "home";

struct Handle {
    window: ApplicationWindow,
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
        let display = WidgetExt::display(&handle.window);

        if action.force_onboard {
            tour::restart(&display);
            return;
        }

        if let Some(id) = &action.tour_event {
            tour::on_tour_event(id);
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
        // respond to SUPER+/ instantly) but only starts the tour / pops the
        // window open on a genuine first run — never on later logins.
        if action.autostart && !State::load().onboarding_completed() {
            tour::start(&display);
            return;
        }
        let silent_autostart = action.autostart && State::load().onboarding_completed();
        if !silent_autostart {
            handle.window.present();
        }
    });
}

fn build(app: &Application) -> Handle {
    // First thing on every cold start: revert any keybind a previous,
    // crashed run may have left temporarily rebound mid tour-step (see
    // `ui::tour`'s crash-safety notes) before the user could be surprised
    // by it firing again.
    tour::self_heal();

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

    let home = super::home::build(&binds, &window, state.mode(), {
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

    window.set_child(Some(&content_vbox));
    // Deliberately not presented here — `present()` (the caller) decides
    // whether this initial build should actually be shown (see
    // `silent_autostart` above), so a from-cold every-login autostart with
    // onboarding already complete builds a ready-but-hidden window instead
    // of flashing it open. On a genuine first run, the tour overlay runs
    // independently of this window (see `tour::start`) — it never needs to
    // be shown at all until the user explicitly opens it later.

    Handle { window, home }
}
