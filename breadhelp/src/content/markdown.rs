//! Constrained-subset markdown parser + renderer for guide `content.md`.
//!
//! Deliberately not a full CommonMark parser (`pulldown-cmark`/`comrak`) and
//! deliberately not rendered as HTML in a WebKitGTK view: no sibling app in
//! this ecosystem embeds a browser engine, a full parser still wouldn't
//! understand the `[Run]`/`[Show Keybind]` tags so it buys nothing, and this
//! content is authored in-repo by BOS maintainers, not arbitrary user input —
//! a small subset is an acceptable authoring limit, not a security compromise.
//!
//! Supported: `#`/`##` headings, one-block-per-line paragraphs, `- ` bullets,
//! inline `**bold**`/`` `code` `` (rendered via GTK's own Pango markup), and
//! two custom tags: `[Run]<command>[/Run]` (optional `|Label` suffix) and
//! `[Show Keybind]<combo>[/Show Keybind]`.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Label, Orientation};

use super::keybinds::Keybind;

#[derive(Debug, Clone, PartialEq)]
pub enum Span {
    Text(String),
    Run { command: String, label: String },
    Keybind(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading(u8, String),
    Paragraph(Vec<Span>),
    Bullet(Vec<Span>),
}

pub fn parse(src: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    for raw_line in src.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            blocks.push(Block::Heading(2, rest.trim().to_string()));
        } else if let Some(rest) = line.strip_prefix("# ") {
            blocks.push(Block::Heading(1, rest.trim().to_string()));
        } else if let Some(rest) = line.strip_prefix("- ") {
            blocks.push(Block::Bullet(parse_spans(rest)));
        } else {
            blocks.push(Block::Paragraph(parse_spans(line)));
        }
    }
    blocks
}

fn parse_spans(text: &str) -> Vec<Span> {
    const RUN_OPEN: &str = "[Run]";
    const RUN_CLOSE: &str = "[/Run]";
    const KB_OPEN: &str = "[Show Keybind]";
    const KB_CLOSE: &str = "[/Show Keybind]";

    let mut spans = Vec::new();
    let mut rest = text;
    loop {
        let run_idx = rest.find(RUN_OPEN);
        let kb_idx = rest.find(KB_OPEN);
        let next = match (run_idx, kb_idx) {
            (Some(r), Some(k)) if r <= k => Some((r, true)),
            (Some(_), Some(k)) => Some((k, false)),
            (Some(r), None) => Some((r, true)),
            (None, Some(k)) => Some((k, false)),
            (None, None) => None,
        };
        let Some((idx, is_run)) = next else {
            if !rest.is_empty() {
                spans.push(Span::Text(rest.to_string()));
            }
            break;
        };
        if idx > 0 {
            spans.push(Span::Text(rest[..idx].to_string()));
        }
        let (open_len, close_tag) = if is_run { (RUN_OPEN.len(), RUN_CLOSE) } else { (KB_OPEN.len(), KB_CLOSE) };
        let after = &rest[idx + open_len..];
        let Some(end) = after.find(close_tag) else {
            // Unterminated tag — keep the literal text rather than dropping it.
            spans.push(Span::Text(rest[idx..].to_string()));
            break;
        };
        let inner = &after[..end];
        if is_run {
            let (command, label) = match inner.split_once('|') {
                Some((c, l)) => (c.trim().to_string(), l.trim().to_string()),
                None => (inner.trim().to_string(), inner.trim().to_string()),
            };
            spans.push(Span::Run { command, label });
        } else {
            spans.push(Span::Keybind(inner.trim().to_string()));
        }
        rest = &after[end + close_tag.len()..];
    }
    spans
}

pub fn render(blocks: &[Block], binds: &[Keybind]) -> gtk4::Widget {
    let vbox = GBox::new(Orientation::Vertical, 10);
    for block in blocks {
        match block {
            Block::Heading(level, text) => {
                let lbl = Label::new(None);
                let size = if *level == 1 { "large" } else { "medium" };
                lbl.set_markup(&format!("<b><span size='{size}'>{}</span></b>", glib::markup_escape_text(text)));
                lbl.set_xalign(0.0);
                lbl.set_wrap(true);
                vbox.append(&lbl);
            }
            Block::Paragraph(spans) => vbox.append(&render_spans(spans, binds)),
            Block::Bullet(spans) => {
                let row = GBox::new(Orientation::Horizontal, 6);
                row.append(&Label::new(Some("\u{2022}")));
                row.append(&render_spans(spans, binds));
                vbox.append(&row);
            }
        }
    }
    vbox.upcast()
}

fn render_spans(spans: &[Span], binds: &[Keybind]) -> gtk4::Widget {
    let row = GBox::new(Orientation::Horizontal, 6);
    for span in spans {
        match span {
            Span::Text(t) => {
                if t.trim().is_empty() {
                    continue;
                }
                let lbl = Label::new(None);
                lbl.set_markup(&inline_markup(t));
                lbl.set_wrap(true);
                // `set_wrap` alone only wraps once something else constrains
                // the label's allocated width (true inside guide_view's fixed-
                // width scrolled pane) — a freestanding top-level window (the
                // tour callout) has nothing to allocate against, so without
                // this a long paragraph reports its unwrapped natural width
                // and the whole window balloons to fit it.
                lbl.set_max_width_chars(48);
                lbl.set_xalign(0.0);
                row.append(&lbl);
            }
            Span::Run { command, label } => {
                let btn = Button::with_label(label);
                let cmd = command.clone();
                btn.connect_clicked(move |_| crate::services::exec::run(&cmd));
                row.append(&btn);
            }
            Span::Keybind(combo) => {
                let text = super::keybinds::find(binds, combo).map(|k| k.combo.clone()).unwrap_or_else(|| combo.clone());
                let chip = Label::new(Some(&text));
                chip.add_css_class("keybind-chip");
                row.append(&chip);
            }
        }
    }
    row.upcast()
}

fn inline_markup(text: &str) -> String {
    let escaped = glib::markup_escape_text(text).to_string();
    let bolded = replace_delim(&escaped, "**", "<b>", "</b>");
    replace_delim(&bolded, "`", "<tt>", "</tt>")
}

/// Alternates `open`/`close` between successive `delim`-separated segments —
/// an odd number of delimiters leaves a dangling tag, which GTK's markup
/// parser reports as a warning rather than crashing (acceptable: this only
/// runs over content authored in-repo, per the module's rationale above).
fn replace_delim(s: &str, delim: &str, open: &str, close: &str) -> String {
    let mut parts = s.split(delim);
    let mut out = parts.next().unwrap_or_default().to_string();
    for (i, part) in parts.enumerate() {
        out.push_str(if i % 2 == 0 { open } else { close });
        out.push_str(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heading_bullet_paragraph() {
        let blocks = parse("# Title\n\nSome text.\n\n- a bullet\n");
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], Block::Heading(1, t) if t == "Title"));
        assert!(matches!(&blocks[1], Block::Paragraph(_)));
        assert!(matches!(&blocks[2], Block::Bullet(_)));
    }

    #[test]
    fn parses_run_tag_with_label() {
        let spans = parse_spans("Click [Run]bos-update|Update everything[/Run] to update.");
        assert_eq!(spans.len(), 3);
        assert!(matches!(&spans[0], Span::Text(t) if t == "Click "));
        assert_eq!(
            spans[1],
            Span::Run { command: "bos-update".into(), label: "Update everything".into() }
        );
        assert!(matches!(&spans[2], Span::Text(t) if t == " to update."));
    }

    #[test]
    fn parses_run_tag_without_label() {
        let spans = parse_spans("[Run]breadpad[/Run]");
        assert_eq!(spans, vec![Span::Run { command: "breadpad".into(), label: "breadpad".into() }]);
    }

    #[test]
    fn parses_show_keybind_tag() {
        let spans = parse_spans("Press [Show Keybind]super + space[/Show Keybind] to launch.");
        assert_eq!(spans[1], Span::Keybind("super + space".into()));
    }

    #[test]
    fn counts_multiple_tags_in_one_line() {
        let spans =
            parse_spans("[Run]a[/Run] then [Show Keybind]super+/[/Show Keybind] then [Run]b|B[/Run]");
        let run_count = spans.iter().filter(|s| matches!(s, Span::Run { .. })).count();
        let kb_count = spans.iter().filter(|s| matches!(s, Span::Keybind(_))).count();
        assert_eq!(run_count, 2);
        assert_eq!(kb_count, 1);
    }

    #[test]
    fn unterminated_tag_falls_back_to_text() {
        let spans = parse_spans("oops [Run]no closing tag");
        assert_eq!(spans, vec![Span::Text("oops ".into()), Span::Text("[Run]no closing tag".into())]);
    }
}
