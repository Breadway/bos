//! Maps a `--suggest <id>` payload (sent by a breadd Lua module, e.g.
//! `breadhelp-suggest.lua` reacting to `bread.monitor.connected`) to Home
//! tab banner text.

pub struct Suggestion {
    pub text: String,
}

pub fn resolve(id: &str) -> Option<Suggestion> {
    match id {
        "monitor-setup" => Some(Suggestion {
            text: "New monitor detected — want to set up your display layout?".to_string(),
        }),
        _ => None,
    }
}
