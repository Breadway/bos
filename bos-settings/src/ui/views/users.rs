//! User account management — add/remove users, change passwords. Everything
//! here needs root (useradd/userdel/chpasswd), so every action goes through
//! `pkexec` on a background thread (GTK widgets aren't Send), matching the
//! async_channel + glib::spawn_future_local handoff used elsewhere for
//! destructive/root actions (see snapshots.rs).

use gtk4::prelude::*;
use gtk4::{AlertDialog, Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::ui::widgets as w;

#[derive(Clone)]
struct Account {
    username: String,
    full_name: String,
}

fn list_accounts() -> Vec<Account> {
    let Ok(text) = std::fs::read_to_string("/etc/passwd") else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() < 7 {
                return None;
            }
            let uid: u32 = f[2].parse().ok()?;
            let shell = f[6];
            // Real human accounts: normal UID range, a real login shell (not
            // nologin/false — excludes system/service accounts like
            // greeter, avahi, etc).
            if !(1000..60000).contains(&uid) || shell.ends_with("nologin") || shell.ends_with("/false") {
                return None;
            }
            Some(Account {
                username: f[0].to_string(),
                full_name: f[4].split(',').next().unwrap_or("").to_string(),
            })
        })
        .collect()
}

fn current_user() -> String {
    std::env::var("USER").unwrap_or_default()
}

/// Run a root command that needs a line of input on stdin (chpasswd's own
/// "user:password" format) on a background thread, reporting success back
/// via a channel. `pkexec` inherits the spawning process's stdin only when
/// explicitly piped, so this pipes it through rather than relying on that.
fn run_with_stdin(args: &[&str], input: String, on_done: impl FnOnce(bool) + 'static) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (tx, rx) = async_channel::bounded::<bool>(1);
    std::thread::spawn(move || {
        let ok = (|| -> std::io::Result<bool> {
            let mut child = Command::new(&args[0])
                .args(&args[1..])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input.as_bytes())?;
            }
            Ok(child.wait()?.success())
        })()
        .unwrap_or(false);
        let _ = tx.send_blocking(ok);
    });
    glib::spawn_future_local(async move {
        let ok = rx.recv().await.unwrap_or(false);
        on_done(ok);
    });
}

fn populate(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let me = current_user();
    for acc in list_accounts() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let card = GBox::new(Orientation::Vertical, 6);
        card.add_css_class("card");
        card.set_margin_top(3);
        card.set_margin_bottom(3);

        let top = GBox::new(Orientation::Horizontal, 12);
        let label_text = if acc.full_name.is_empty() {
            acc.username.clone()
        } else {
            format!("{}  ({})", acc.username, acc.full_name)
        };
        let name_lbl = Label::new(Some(&label_text));
        name_lbl.set_hexpand(true);
        name_lbl.set_xalign(0.0);

        let change_pw_btn = Button::with_label("Change password");
        let remove_btn = Button::with_label("Remove");
        remove_btn.add_css_class("destructive-action");
        // Don't let the panel remove the account it's currently running as
        // — that's a self-lockout, not a normal account-management action.
        remove_btn.set_sensitive(acc.username != me);

        top.append(&name_lbl);
        top.append(&change_pw_btn);
        top.append(&remove_btn);
        card.append(&top);

        // Inline password row, hidden until "Change password" is clicked —
        // same pattern as Network's secured-connection password prompt.
        let pw_row = GBox::new(Orientation::Horizontal, 8);
        let pw_entry = Entry::new();
        pw_entry.set_visibility(false);
        pw_entry.set_hexpand(true);
        pw_entry.set_placeholder_text(Some("New password"));
        let pw_apply_btn = Button::with_label("Apply");
        pw_row.append(&pw_entry);
        pw_row.append(&pw_apply_btn);
        pw_row.set_visible(false);
        card.append(&pw_row);

        let status = Label::new(None);
        status.add_css_class("dim-label");
        status.set_xalign(0.0);
        card.append(&status);

        {
            let pw_row = pw_row.clone();
            change_pw_btn.connect_clicked(move |_| {
                pw_row.set_visible(!pw_row.is_visible());
            });
        }
        {
            let username = acc.username.clone();
            let pw_entry = pw_entry.clone();
            let pw_row = pw_row.clone();
            let status = status.clone();
            pw_apply_btn.connect_clicked(move |_| {
                let password = pw_entry.text().to_string();
                if password.is_empty() {
                    return;
                }
                let input = format!("{username}:{password}\n");
                let status2 = status.clone();
                let pw_entry2 = pw_entry.clone();
                let pw_row2 = pw_row.clone();
                status.set_text("Applying…");
                run_with_stdin(&["pkexec", "chpasswd"], input, move |ok| {
                    if ok {
                        status2.set_text("Password changed");
                        pw_entry2.set_text("");
                        pw_row2.set_visible(false);
                    } else {
                        status2.set_text("Failed to change password");
                    }
                });
            });
        }
        {
            let list = list.clone();
            let username = acc.username.clone();
            remove_btn.connect_clicked(move |btn| {
                let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
                let dialog = AlertDialog::builder()
                    .message(format!("Remove user {username}?"))
                    .detail("Deletes the account and its home directory. This cannot be undone.")
                    .buttons(["Cancel", "Remove"])
                    .cancel_button(0)
                    .default_button(0)
                    .build();
                let list2 = list.clone();
                let username2 = username.clone();
                dialog.choose(window.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                    if result != Ok(1) {
                        return;
                    }
                    let log_buf = gtk4::TextBuffer::new(None);
                    let list3 = list2.clone();
                    w::stream_command_then(
                        &["pkexec", "userdel", "-r", &username2],
                        log_buf,
                        move || populate(&list3),
                    );
                });
            });
        }

        row.set_child(Some(&card));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Users");
    content.append(&w::hint(
        "Real login accounts on this machine (system/service accounts \
         aren't shown). Your own account can't be removed from here.",
    ));

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    populate(&list);
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_min_content_height(260);
    scroll.set_child(Some(&list));
    content.append(&scroll);

    content.append(&w::section("Add user"));
    let username_entry = Entry::new();
    username_entry.set_placeholder_text(Some("username"));
    content.append(&w::row("Username", &username_entry));

    let fullname_entry = Entry::new();
    fullname_entry.set_placeholder_text(Some("Full name (optional)"));
    content.append(&w::row("Full name", &fullname_entry));

    let password_entry = Entry::new();
    password_entry.set_visibility(false);
    password_entry.set_placeholder_text(Some("password"));
    content.append(&w::row("Password", &password_entry));

    let add_status = Label::new(None);
    add_status.add_css_class("dim-label");
    add_status.set_xalign(0.0);

    let add_btn = Button::with_label("Add user");
    add_btn.add_css_class("suggested-action");
    add_btn.set_halign(gtk4::Align::Start);
    add_btn.set_margin_top(8);
    {
        let list = list.clone();
        let username_entry = username_entry.clone();
        let fullname_entry = fullname_entry.clone();
        let password_entry = password_entry.clone();
        let add_status = add_status.clone();
        add_btn.connect_clicked(move |_| {
            let username = username_entry.text().to_string();
            let full_name = fullname_entry.text().to_string();
            let password = password_entry.text().to_string();
            if username.trim().is_empty() || password.is_empty() {
                add_status.set_text("Username and password are required.");
                return;
            }
            add_status.set_text("Adding…");

            let mut useradd_args = vec!["pkexec", "useradd", "-m", "-s", "/bin/bash"];
            if !full_name.trim().is_empty() {
                useradd_args.push("-c");
                useradd_args.push(full_name.trim());
            }
            useradd_args.push(username.trim());

            let list2 = list.clone();
            let username2 = username.trim().to_string();
            let password2 = password.clone();
            let add_status2 = add_status.clone();
            let username_entry2 = username_entry.clone();
            let fullname_entry2 = fullname_entry.clone();
            let password_entry2 = password_entry.clone();
            let log_buf = gtk4::TextBuffer::new(None);
            w::stream_command_then(&useradd_args, log_buf, move || {
                let input = format!("{username2}:{password2}\n");
                let list3 = list2.clone();
                let add_status3 = add_status2.clone();
                let username_entry3 = username_entry2.clone();
                let fullname_entry3 = fullname_entry2.clone();
                let password_entry3 = password_entry2.clone();
                run_with_stdin(&["pkexec", "chpasswd"], input, move |ok| {
                    if ok {
                        add_status3.set_text("User added.");
                        username_entry3.set_text("");
                        fullname_entry3.set_text("");
                        password_entry3.set_text("");
                        populate(&list3);
                    } else {
                        add_status3.set_text("User created, but setting the password failed.");
                        populate(&list3);
                    }
                });
            });
        });
    }
    content.append(&add_btn);
    content.append(&add_status);

    outer
}
