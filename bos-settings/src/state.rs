pub struct AppState {
    pub current_view: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_view: "snapshots".to_string(),
        }
    }
}
