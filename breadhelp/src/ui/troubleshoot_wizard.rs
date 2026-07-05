//! Troubleshooting wizard: a small modal window that walks a symptom tree
//! (`content::troubleshoot`) — prompt as a heading, `options` as buttons,
//! a leaf's `fix` as a "Run fix" button + result status.
//!
//! Uses the `Rc<Self>` method-receiver pattern (rather than a boxed
//! recursive closure) so each button can re-render the same widget tree at
//! a different node — `self.clone()` (an `Rc` clone) moves cleanly into
//! GTK's `'static` signal closures.

use std::collections::HashMap;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box as GBox, Button, Label, Orientation, Window};

use crate::content::troubleshoot::{self, SymptomNode};

/// Curated symptom files with a human label — file stems aren't user-facing
/// strings, and not every `_symptoms/*.toml` necessarily belongs on the
/// top-level picker.
const SYMPTOMS: &[(&str, &str)] = &[("no-sound", "No sound"), ("wifi-wont-connect", "Wi-Fi won't connect")];

struct Wizard {
    content: GBox,
    trees: HashMap<String, Vec<SymptomNode>>,
}

pub fn open(parent: &impl IsA<gtk4::Window>) {
    let window = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Something's wrong?")
        .default_width(480)
        .default_height(360)
        .build();

    let content = GBox::new(Orientation::Vertical, 12);
    content.add_css_class("view-content");

    let wizard = Rc::new(Wizard { content: content.clone(), trees: troubleshoot::load_all() });
    wizard.render_picker();

    window.set_child(Some(&content));
    window.present();
}

impl Wizard {
    fn clear(&self) {
        while let Some(child) = self.content.first_child() {
            self.content.remove(&child);
        }
    }

    fn render_picker(self: &Rc<Self>) {
        self.clear();

        let title = Label::new(Some("What's wrong?"));
        title.add_css_class("title");
        title.set_xalign(0.0);
        self.content.append(&title);

        let mut any = false;
        for (file, label) in SYMPTOMS {
            if !self.trees.contains_key(*file) {
                continue;
            }
            any = true;
            let btn = Button::with_label(label);
            btn.set_halign(Align::Start);
            let this = self.clone();
            let file = file.to_string();
            btn.connect_clicked(move |_| this.render_node(&file, "start"));
            self.content.append(&btn);
        }
        if !any {
            let empty = Label::new(Some("No troubleshooting guides installed."));
            empty.add_css_class("dim-label");
            self.content.append(&empty);
        }
    }

    fn render_node(self: &Rc<Self>, file: &str, id: &str) {
        let Some(nodes) = self.trees.get(file) else {
            self.render_picker();
            return;
        };
        let Some(node) = nodes.iter().find(|n| n.id == id) else {
            self.render_picker();
            return;
        };

        self.clear();

        let prompt = Label::new(Some(&node.prompt));
        prompt.set_wrap(true);
        prompt.set_xalign(0.0);
        self.content.append(&prompt);

        for opt in &node.options {
            let btn = Button::with_label(&opt.label);
            btn.set_halign(Align::Start);
            let this = self.clone();
            // `goto` is either a bare node id (same file) or
            // "<file-stem>:<id>" to jump into a different symptom file.
            let (goto_file, goto_id) = match opt.goto.split_once(':') {
                Some((f, i)) => (f.to_string(), i.to_string()),
                None => (file.to_string(), opt.goto.clone()),
            };
            btn.connect_clicked(move |_| this.render_node(&goto_file, &goto_id));
            self.content.append(&btn);
        }

        if let Some(fix) = &node.fix {
            let desc = Label::new(Some(&fix.description));
            desc.set_xalign(0.0);
            self.content.append(&desc);

            let status = Label::new(None);
            status.set_xalign(0.0);
            status.add_css_class("dim-label");

            let run_btn = Button::with_label("Run fix");
            run_btn.add_css_class("suggested-action");
            run_btn.set_halign(Align::Start);
            let command = fix.command.clone();
            let status_for_click = status.clone();
            run_btn.connect_clicked(move |_| {
                status_for_click.set_label("Running\u{2026}");
                let status = status_for_click.clone();
                crate::services::exec::run_reporting(&command, move |success| {
                    status.set_label(if success { "Done." } else { "Something went wrong — see the terminal for details." });
                });
            });
            self.content.append(&run_btn);
            self.content.append(&status);
        }

        let back_btn = Button::with_label("Start over");
        back_btn.set_halign(Align::Start);
        let this = self.clone();
        back_btn.connect_clicked(move |_| this.render_picker());
        self.content.append(&back_btn);
    }
}
