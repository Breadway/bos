use gtk4::prelude::*;
use gtk4::{
    Align, ApplicationWindow, Box as GBox, Button, DropDown, FlowBox, Label, Orientation, Revealer,
    RevealerTransitionType, ScrolledWindow, StringList, Widget,
};

use crate::config::Mode;
use crate::content::{keybinds::Keybind, ContentStore};

use super::{keybind_viewer, troubleshoot_wizard};

const FIX_SOUND: &str = "systemctl --user restart wireplumber pipewire pipewire.socket pipewire-pulse pipewire-pulse.socket";
const FIX_WIFI: &str = "nmcli radio wifi off && sleep 1 && nmcli radio wifi on";
const FIX_UPDATE: &str = "bos-update";
const FIX_BACKUP: &str = "snapper -c root create --description 'breadhelp manual backup'";

/// Handle back to the parts of Home that get updated after construction —
/// the suggestion banner (from breadd, see `services::breadd`) and the
/// one-click-fix toast, neither of which exist until the widget tree is
/// built once.
pub struct Home {
    pub root: Widget,
    banner_revealer: Revealer,
    banner_label: Label,
}

impl Home {
    pub fn set_suggestion(&self, text: Option<&str>) {
        match text {
            Some(t) => {
                self.banner_label.set_label(t);
                self.banner_revealer.set_reveal_child(true);
            }
            None => self.banner_revealer.set_reveal_child(false),
        }
    }
}

pub fn build(
    store: &ContentStore,
    binds: &[Keybind],
    parent_window: &ApplicationWindow,
    initial_mode: Mode,
    on_mode_change: impl Fn(Mode) + 'static,
) -> Home {
    let vbox = GBox::new(Orientation::Vertical, 16);
    vbox.add_css_class("view-content");

    let header = GBox::new(Orientation::Horizontal, 12);
    let title = Label::new(Some("Home"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    title.set_hexpand(true);
    header.append(&title);
    header.append(&mode_selector(initial_mode, on_mode_change));
    vbox.append(&header);

    let banner_label = Label::new(None);
    banner_label.set_wrap(true);
    banner_label.set_xalign(0.0);
    banner_label.set_hexpand(true);
    let banner_dismiss = Button::from_icon_name("window-close-symbolic");
    banner_dismiss.set_tooltip_text(Some("Dismiss"));
    banner_dismiss.update_property(&[gtk4::accessible::Property::Label("Dismiss")]);
    let banner_row = GBox::new(Orientation::Horizontal, 8);
    banner_row.add_css_class("card");
    banner_row.set_margin_top(4);
    banner_row.set_margin_bottom(4);
    banner_row.append(&banner_label);
    banner_row.append(&banner_dismiss);
    let banner_revealer = Revealer::new();
    banner_revealer.set_transition_type(RevealerTransitionType::SlideDown);
    banner_revealer.set_child(Some(&banner_row));
    banner_revealer.set_reveal_child(false);
    {
        let banner_revealer = banner_revealer.clone();
        banner_dismiss.connect_clicked(move |_| banner_revealer.set_reveal_child(false));
    }
    vbox.append(&banner_revealer);

    vbox.append(&quick_fixes());

    let troubleshoot_btn = Button::with_label("Something's wrong?");
    troubleshoot_btn.set_halign(Align::Start);
    {
        let parent = parent_window.clone();
        troubleshoot_btn.connect_clicked(move |_| troubleshoot_wizard::open(&parent));
    }
    vbox.append(&troubleshoot_btn);

    let getting_started = store.guides_in("getting-started");
    if !getting_started.is_empty() {
        let heading = Label::new(Some("Getting Started"));
        heading.add_css_class("section-header");
        heading.set_xalign(0.0);
        vbox.append(&heading);

        for guide in getting_started.iter().take(3) {
            let lbl = Label::new(Some(&format!("\u{2022} {}", guide.meta.title)));
            lbl.set_xalign(0.0);
            vbox.append(&lbl);
        }
    }

    vbox.append(&keybind_viewer::build(binds));

    let root = ScrolledWindow::builder().child(&vbox).vexpand(true).hexpand(true).build().upcast();

    Home { root, banner_revealer, banner_label }
}

fn quick_fixes() -> Widget {
    let heading_box = GBox::new(Orientation::Vertical, 8);

    let heading = Label::new(Some("Quick fixes"));
    heading.add_css_class("section-header");
    heading.set_xalign(0.0);
    heading_box.append(&heading);

    let flow = FlowBox::new();
    flow.set_selection_mode(gtk4::SelectionMode::None);
    flow.set_max_children_per_line(4);
    flow.set_column_spacing(8);
    flow.set_row_spacing(8);

    let status = Label::new(None);
    status.set_xalign(0.0);
    status.add_css_class("dim-label");

    for (label, command) in [
        ("Fix sound", FIX_SOUND),
        ("Reconnect Wi-Fi", FIX_WIFI),
        ("Update everything", FIX_UPDATE),
        ("Create backup", FIX_BACKUP),
    ] {
        let btn = Button::with_label(label);
        let command = command.to_string();
        let status = status.clone();
        let label = label.to_string();
        btn.connect_clicked(move |_| {
            status.set_label(&format!("{label}\u{2026}"));
            let status = status.clone();
            let label = label.clone();
            crate::services::exec::run_reporting(&command, move |success| {
                status.set_label(&if success { format!("{label}: done.") } else { format!("{label}: something went wrong.") });
            });
        });
        flow.insert(&btn, -1);
    }

    heading_box.append(&flow);
    heading_box.append(&status);
    heading_box.upcast()
}

fn mode_selector(initial: Mode, on_change: impl Fn(Mode) + 'static) -> DropDown {
    let labels = StringList::new(&["Normal", "Beginner", "Dad", "Compact"]);
    let dropdown = DropDown::new(Some(labels), gtk4::Expression::NONE);
    dropdown.set_selected(mode_index(initial));
    dropdown.set_tooltip_text(Some("Display mode"));
    dropdown.update_property(&[gtk4::accessible::Property::Label("Display mode")]);
    dropdown.connect_selected_notify(move |dd| {
        let mode = match dd.selected() {
            1 => Mode::Beginner,
            2 => Mode::Dad,
            3 => Mode::Compact,
            _ => Mode::Normal,
        };
        on_change(mode);
    });
    dropdown
}

fn mode_index(mode: Mode) -> u32 {
    match mode {
        Mode::Normal => 0,
        Mode::Beginner => 1,
        Mode::Dad => 2,
        Mode::Compact => 3,
    }
}
