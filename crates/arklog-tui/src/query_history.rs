use crate::text_input::MAX_INPUT_BYTES;

const QUERY_HISTORY_LIMIT: usize = 50;

pub struct QueryHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
    draft: Option<String>,
    browse_values: Vec<String>,
}

impl QueryHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cursor: None,
            draft: None,
            browse_values: Vec::new(),
        }
    }

    pub fn record_applied(&mut self, query: &str) -> bool {
        if query.is_empty() || query.len() > MAX_INPUT_BYTES {
            return false;
        }
        if let Some(index) = self.entries.iter().position(|entry| entry == query) {
            self.entries.remove(index);
        }
        if self.entries.len() == QUERY_HISTORY_LIMIT {
            self.entries.remove(0);
        }
        self.entries.push(query.to_string());
        self.cancel_navigation();
        true
    }

    pub fn older(&mut self, current: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        match self.cursor {
            None => {
                self.draft = Some(current.to_string());
                self.browse_values.clone_from(&self.entries);
                let index = self.browse_values.len() - 1;
                self.cursor = Some(index);
                Some(self.browse_values[index].clone())
            }
            Some(index) => {
                self.browse_values[index] = current.to_string();
                if index == 0 {
                    None
                } else {
                    self.cursor = Some(index - 1);
                    Some(self.browse_values[index - 1].clone())
                }
            }
        }
    }

    pub fn newer(&mut self, current: &str) -> Option<String> {
        let index = self.cursor?;
        self.browse_values[index] = current.to_string();
        if index + 1 < self.browse_values.len() {
            self.cursor = Some(index + 1);
            Some(self.browse_values[index + 1].clone())
        } else {
            let draft = self.draft.take().unwrap_or_default();
            self.cursor = None;
            self.browse_values.clear();
            Some(draft)
        }
    }

    pub fn cancel_navigation(&mut self) {
        self.cursor = None;
        self.draft = None;
        self.browse_values.clear();
    }
}

impl Default for QueryHistory {
    fn default() -> Self {
        Self::new()
    }
}
