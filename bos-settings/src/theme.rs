use gtk4::prelude::*;
use gtk4::CssProvider;

const CSS: &str = r#"
window {
    background-color: #2e3440;
    color: #eceff4;
}

.sidebar {
    background-color: #3b4252;
    border-right: 1px solid #434c5e;
}

.sidebar row {
    padding: 8px 12px;
    color: #d8dee9;
}

.sidebar row:selected {
    background-color: #5e81ac;
    color: #eceff4;
}

.sidebar .section-header {
    padding: 12px 12px 4px 12px;
    font-size: 0.75em;
    font-weight: bold;
    color: #616e88;
    text-transform: uppercase;
    letter-spacing: 1px;
}

.view-content {
    padding: 24px;
}

.view-content label.title {
    font-size: 1.4em;
    font-weight: bold;
    color: #eceff4;
    margin-bottom: 16px;
}

button {
    background-color: #5e81ac;
    color: #eceff4;
    border: none;
    border-radius: 4px;
    padding: 6px 16px;
}

button:hover {
    background-color: #81a1c1;
}

button.destructive-action {
    background-color: #bf616a;
}

button.destructive-action:hover {
    background-color: #d08770;
}

entry {
    background-color: #434c5e;
    color: #eceff4;
    border: 1px solid #4c566a;
    border-radius: 4px;
}

textview {
    background-color: #272c36;
    color: #a3be8c;
    font-family: monospace;
    padding: 8px;
}
"#;

pub fn load(display: &gtk4::gdk::Display) {
    let provider = CssProvider::new();
    provider.load_from_string(CSS);
    gtk4::style_context_add_provider_for_display(
        display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
