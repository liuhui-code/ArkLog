use std::io;

use regex::Regex;

use crate::SessionLogStore;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogTab {
    HiLog,
    FaultLog,
}

pub struct ArkLogState {
    store: SessionLogStore,
    scroll_start: Option<u64>,
    filter_query: String,
    filter_error: Option<String>,
    find_query: String,
    current_find: u64,
    current_find_visible_index: Option<u64>,
    tab: LogTab,
}

impl ArkLogState {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            store: SessionLogStore::new()?,
            scroll_start: None,
            filter_query: String::new(),
            filter_error: None,
            find_query: String::new(),
            current_find: 0,
            current_find_visible_index: None,
            tab: LogTab::HiLog,
        })
    }

    pub fn append_lines<I, S>(&mut self, lines: I) -> io::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.store.append_lines(lines)
    }

    pub fn visible_window(&mut self, height: usize) -> io::Result<Vec<String>> {
        let start = self.window_start(height);
        self.store.visible_window(start, height)
    }

    pub fn apply_filter(&mut self, query: impl Into<String>) {
        self.filter_query = query.into();
        self.filter_error = self.store.set_filter(&self.filter_query).err();
        if self.filter_error.is_none() {
            self.normalize_current_find();
            if let Err(error) = self.reveal_current_find() {
                self.filter_error = Some(error.to_string());
            }
        }
        self.follow_latest();
    }

    pub fn filter_query(&self) -> &str {
        &self.filter_query
    }

    pub fn filter_error(&self) -> Option<&str> {
        self.filter_error.as_deref()
    }

    pub fn active_filter(&self) -> Option<&Regex> {
        self.store.active_filter()
    }

    pub fn raw_count(&self) -> u64 {
        self.store.raw_count()
    }

    pub fn visible_count(&self) -> u64 {
        self.store.visible_count()
    }

    pub fn clear(&mut self) -> io::Result<()> {
        self.store.clear()?;
        self.scroll_start = None;
        self.current_find = 0;
        self.current_find_visible_index = None;
        Ok(())
    }

    pub fn set_find(&mut self, query: impl Into<String>) -> io::Result<()> {
        self.find_query = query.into();
        self.store.set_find(&self.find_query)?;
        self.current_find = 0;
        self.reveal_current_find()
    }

    pub fn find_query(&self) -> &str {
        &self.find_query
    }

    pub fn find_status(&self) -> (u64, u64) {
        let count = self.store.find_count();
        if count == 0 {
            (0, 0)
        } else {
            (self.current_find.min(count - 1) + 1, count)
        }
    }

    pub fn current_find_visible_index(&self) -> Option<u64> {
        self.current_find_visible_index
    }

    pub fn tab(&self) -> LogTab {
        self.tab
    }

    pub fn next_tab(&mut self) {
        self.tab = match self.tab {
            LogTab::HiLog => LogTab::FaultLog,
            LogTab::FaultLog => LogTab::HiLog,
        };
    }

    pub fn next_find(&mut self) -> io::Result<()> {
        let count = self.store.find_count();
        if count > 0 {
            self.current_find = (self.current_find + 1) % count;
        }
        self.reveal_current_find()
    }

    pub fn previous_find(&mut self) -> io::Result<()> {
        let count = self.store.find_count();
        if count > 0 {
            self.current_find = if self.current_find == 0 {
                count - 1
            } else {
                self.current_find - 1
            };
        }
        self.reveal_current_find()
    }

    pub fn scroll_up(&mut self, rows: u64, height: usize) {
        let current = self.window_start(height);
        self.scroll_start = Some(current.saturating_sub(rows));
    }

    pub fn scroll_down(&mut self, rows: u64, height: usize) {
        let latest = self.store.visible_count().saturating_sub(height as u64);
        let next = self.window_start(height).saturating_add(rows);
        self.scroll_start = (next < latest).then_some(next);
    }

    pub fn visible_window_start(&self, height: usize) -> u64 {
        self.window_start(height)
    }

    pub fn follow_latest(&mut self) {
        self.scroll_start = None;
    }

    pub fn is_following_latest(&self) -> bool {
        self.scroll_start.is_none()
    }

    fn window_start(&self, height: usize) -> u64 {
        let latest = self.store.visible_count().saturating_sub(height as u64);
        self.scroll_start.unwrap_or(latest).min(latest)
    }

    fn normalize_current_find(&mut self) {
        let count = self.store.find_count();
        self.current_find = self.current_find.min(count.saturating_sub(1));
    }

    fn reveal_current_find(&mut self) -> io::Result<()> {
        self.normalize_current_find();
        self.current_find_visible_index = self.store.find_visible_index(self.current_find)?;
        if let Some(index) = self.current_find_visible_index {
            self.scroll_start = Some(index);
        }
        Ok(())
    }
}
