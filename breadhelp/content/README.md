# breadhelp content

Guides and troubleshooting data, installed to `/usr/share/breadhelp/content/`.
User overrides/additions live at `~/.local/share/breadhelp/content/` (same
shape; a colliding guide directory name wins wholesale over the system copy).

Categories are just top-level directories here — the Learn tab lists whatever
exists, so adding a category is a content change, not a code change:

```
getting-started/
daily-use/
customization/
apps/
troubleshooting/
  _symptoms/          # troubleshooting wizard's symptom-tree *.toml files
advanced/
```

Each guide is a directory with:

- `meta.toml` — title, difficulty, tags, related guides, estimated time
- `content.md` — the guide body (constrained Markdown subset + `[Run]`/
  `[Show Keybind]` tags — see `src/content/markdown.rs`)
- `content.beginner.md` — optional, used in Beginner/Dad mode instead of
  `content.md` when present

Content lands starting at M2.
