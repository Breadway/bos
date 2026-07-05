mod steps;

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box as GBox, Button, Label, Orientation, Stack, StackTransitionType};

use crate::config::State;
use steps::Step;

/// The onboarding tour: a `Stack` of step pages (plain `gtk4::Stack` with a
/// slide transition, not `Adw.Carousel` — no libadwaita dependency exists
/// anywhere in this ecosystem, and introducing one for a 7-step tour isn't
/// justified) with a Back/Skip/Next nav bar and a step-dots progress
/// indicator. The caller places `root` in an `Overlay` above the normal
/// tabbed content and toggles its visibility; `goto` lets a forced restart
/// (`breadhelp --onboard` while already running) jump back to step 0 without
/// rebuilding the widget tree.
pub struct Onboarding {
    pub root: GBox,
    stack: Stack,
    dots: Vec<Label>,
    current: Rc<Cell<usize>>,
}

impl Onboarding {
    pub fn goto(&self, step: usize) {
        let steps = steps::steps();
        let step = step.min(steps.len() - 1);
        self.current.set(step);
        self.stack.set_visible_child_name(&step.to_string());
        update_dots(&self.dots, step);
    }
}

pub fn build(on_finish: impl Fn() + 'static) -> Onboarding {
    let root = GBox::new(Orientation::Vertical, 16);
    root.add_css_class("view-content");
    // GTK4's built-in "background" class paints the theme's opaque default
    // background — needed since this sits in an `Overlay` above the tabs.
    root.add_css_class("background");
    root.set_vexpand(true);
    root.set_hexpand(true);

    let all_steps = steps::steps();

    let stack = Stack::new();
    stack.set_transition_type(StackTransitionType::SlideLeftRight);
    stack.set_vexpand(true);
    stack.set_valign(Align::Center);
    for (i, step) in all_steps.iter().enumerate() {
        stack.add_named(&build_step_page(step), Some(&i.to_string()));
    }

    let dots_row = GBox::new(Orientation::Horizontal, 6);
    dots_row.set_halign(Align::Center);
    let dots: Vec<Label> = all_steps
        .iter()
        .map(|_| {
            let dot = Label::new(Some("\u{25CF}"));
            dot.add_css_class("dim-label");
            dots_row.append(&dot);
            dot
        })
        .collect();

    let start_step = (State::load().onboarding_step() as usize).min(all_steps.len() - 1);
    let current = Rc::new(Cell::new(start_step));
    stack.set_visible_child_name(&start_step.to_string());
    update_dots(&dots, start_step);

    let nav = GBox::new(Orientation::Horizontal, 8);
    nav.set_halign(Align::Fill);

    let skip_btn = Button::with_label("Skip");
    let back_btn = Button::with_label("Back");
    back_btn.set_sensitive(start_step > 0);
    let next_btn = Button::with_label(if start_step + 1 == all_steps.len() { "Finish" } else { "Next" });
    next_btn.add_css_class("suggested-action");

    let spacer = GBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    nav.append(&skip_btn);
    nav.append(&spacer);
    nav.append(&back_btn);
    nav.append(&next_btn);

    let on_finish = Rc::new(on_finish);

    back_btn.connect_clicked({
        let current = current.clone();
        let stack = stack.clone();
        let dots = dots.clone();
        let next_btn = next_btn.clone();
        let back_btn = back_btn.clone();
        let total = all_steps.len();
        move |_| {
            let i = current.get();
            if i == 0 {
                return;
            }
            let new = i - 1;
            current.set(new);
            stack.set_visible_child_name(&new.to_string());
            update_dots(&dots, new);
            back_btn.set_sensitive(new > 0);
            next_btn.set_label(if new + 1 == total { "Finish" } else { "Next" });
            let mut state = State::load();
            state.set_onboarding_step(new as i64);
        }
    });

    next_btn.connect_clicked({
        let current = current.clone();
        let stack = stack.clone();
        let dots = dots.clone();
        let back_btn = back_btn.clone();
        let next_btn = next_btn.clone();
        let total = all_steps.len();
        let on_finish = on_finish.clone();
        move |_| {
            let i = current.get();
            if i + 1 >= total {
                let mut state = State::load();
                state.set_onboarding_completed(true);
                on_finish();
                return;
            }
            let new = i + 1;
            current.set(new);
            stack.set_visible_child_name(&new.to_string());
            update_dots(&dots, new);
            back_btn.set_sensitive(true);
            next_btn.set_label(if new + 1 == total { "Finish" } else { "Next" });
            let mut state = State::load();
            state.set_onboarding_step(new as i64);
        }
    });

    skip_btn.connect_clicked({
        let on_finish = on_finish.clone();
        move |_| {
            let mut state = State::load();
            state.set_onboarding_completed(true);
            on_finish();
        }
    });

    root.append(&stack);
    root.append(&dots_row);
    root.append(&nav);

    Onboarding { root, stack, dots, current }
}

fn build_step_page(step: &Step) -> gtk4::Widget {
    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.set_valign(Align::Center);
    vbox.set_halign(Align::Center);
    vbox.set_margin_start(40);
    vbox.set_margin_end(40);

    let title = Label::new(Some(step.title));
    title.add_css_class("title");
    title.set_wrap(true);
    title.set_justify(gtk4::Justification::Center);
    vbox.append(&title);

    let body = Label::new(Some(step.body));
    body.set_wrap(true);
    body.set_justify(gtk4::Justification::Center);
    body.set_max_width_chars(56);
    vbox.append(&body);

    if let Some(cmd) = step.demo_cmd {
        let try_btn = Button::with_label("Try it");
        let cmd = cmd.to_string();
        try_btn.connect_clicked(move |_| crate::services::exec::run(&cmd));
        try_btn.set_halign(Align::Center);
        vbox.append(&try_btn);
    }

    vbox.upcast()
}

fn update_dots(dots: &[Label], active: usize) {
    for (i, dot) in dots.iter().enumerate() {
        if i == active {
            dot.remove_css_class("dim-label");
        } else {
            dot.add_css_class("dim-label");
        }
    }
}
