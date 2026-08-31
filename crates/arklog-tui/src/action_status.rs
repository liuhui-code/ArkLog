pub struct ActionStatus {
    error: Option<String>,
}

impl ActionStatus {
    pub fn new(error: Option<String>) -> Self {
        Self { error }
    }

    pub fn record_action(&mut self, result: Result<(), String>) {
        self.error = result.err();
    }

    pub fn record_background(&mut self, result: Result<(), String>) {
        if let Err(error) = result {
            self.error = Some(error);
        }
    }

    pub fn message_or<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.error.as_deref().unwrap_or(fallback)
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
