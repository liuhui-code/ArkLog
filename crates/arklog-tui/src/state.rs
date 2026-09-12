use std::io;

use regex::Regex;

use crate::{
    log_geometry::{
        cell_range_for_bytes, display_width, first_literal_match_range, next_wrapped_row_start,
        previous_wrapped_row_start, wrapped_row_starts_from, wrapped_row_starts_tail,
    },
    theme, QueryHistory, SessionLogStore,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogTab {
    HiLog,
    FaultLog,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct ViewportAnchor {
    record: u64,
    cell: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DisplayWindow {
    pub start_record: u64,
    pub start_cell: u64,
    pub records: Vec<String>,
}

pub struct ArkLogState {
    store: SessionLogStore,
    viewport_anchor: Option<ViewportAnchor>,
    filter_query: String,
    filter_error: Option<String>,
    find_query: String,
    current_find: u64,
    current_find_visible_index: Option<u64>,
    horizontal_offset: u64,
    viewport_width: usize,
    viewport_height: usize,
    filter_history: QueryHistory,
    soft_wrap: bool,
    tab: LogTab,
}

impl ArkLogState {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            store: SessionLogStore::new()?,
            viewport_anchor: None,
            filter_query: String::new(),
            filter_error: None,
            find_query: String::new(),
            current_find: 0,
            current_find_visible_index: None,
            horizontal_offset: 0,
            viewport_width: 1,
            viewport_height: 1,
            filter_history: QueryHistory::new(),
            soft_wrap: false,
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

    pub fn apply_filter(&mut self, query: impl Into<String>) -> Result<(), String> {
        self.filter_query = query.into();
        self.filter_error = self.store.set_filter(&self.filter_query).err();
        if self.filter_error.is_none() {
            self.normalize_current_find();
            if let Err(error) = self.reveal_current_find(false) {
                self.filter_error = Some(error.to_string());
            }
        }
        self.follow_latest();
        let result = self.filter_error.clone().map_or(Ok(()), Err);
        if result.is_ok() {
            self.filter_history.record_applied(&self.filter_query);
        }
        result
    }

    pub fn filter_query(&self) -> &str {
        &self.filter_query
    }

    pub fn filter_error(&self) -> Option<&str> {
        self.filter_error.as_deref()
    }

    pub fn older_filter_query(&mut self, current: &str) -> Option<String> {
        self.filter_history.older(current)
    }

    pub fn newer_filter_query(&mut self, current: &str) -> Option<String> {
        self.filter_history.newer(current)
    }

    pub fn cancel_filter_history_navigation(&mut self) {
        self.filter_history.cancel_navigation();
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
        self.viewport_anchor = None;
        self.current_find = 0;
        self.current_find_visible_index = None;
        Ok(())
    }

    pub fn set_find(&mut self, query: impl Into<String>) -> io::Result<()> {
        self.find_query = query.into();
        self.store.set_find(&self.find_query)?;
        self.current_find = 0;
        self.reveal_current_find(true)
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
        self.reveal_current_find(true)
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
        self.reveal_current_find(true)
    }

    pub fn scroll_up(&mut self, rows: u64, height: usize) -> io::Result<()> {
        if !self.soft_wrap {
            let current = self.window_start(height);
            self.viewport_anchor = Some(ViewportAnchor {
                record: current.saturating_sub(rows),
                cell: 0,
            });
            return Ok(());
        }
        let mut anchor = self.current_display_anchor()?;
        for _ in 0..rows {
            let line = self.visible_record(anchor.record)?.unwrap_or_default();
            if let Some(cell) = previous_wrapped_row_start(&line, anchor.cell, self.viewport_width)
            {
                anchor.cell = cell;
            } else if anchor.record > 0 {
                anchor.record -= 1;
                let previous = self.visible_record(anchor.record)?.unwrap_or_default();
                anchor.cell = wrapped_row_starts_tail(&previous, self.viewport_width, 1)
                    .first()
                    .copied()
                    .unwrap_or(0);
            } else {
                break;
            }
        }
        self.viewport_anchor = Some(anchor);
        Ok(())
    }

    pub fn scroll_down(&mut self, rows: u64, height: usize) -> io::Result<()> {
        if !self.soft_wrap {
            let latest = self.store.visible_count().saturating_sub(height as u64);
            let next = self.window_start(height).saturating_add(rows);
            self.viewport_anchor = (next < latest).then_some(ViewportAnchor {
                record: next,
                cell: 0,
            });
            return Ok(());
        }
        let latest = self.latest_display_anchor()?;
        let mut anchor = self.current_display_anchor()?;
        for _ in 0..rows {
            let line = self.visible_record(anchor.record)?.unwrap_or_default();
            if let Some(cell) = next_wrapped_row_start(&line, anchor.cell, self.viewport_width) {
                anchor.cell = cell;
            } else if anchor.record + 1 < self.store.visible_count() {
                anchor.record += 1;
                anchor.cell = 0;
            } else {
                break;
            }
        }
        self.viewport_anchor = (anchor < latest).then_some(anchor);
        Ok(())
    }

    pub fn visible_window_start(&self, height: usize) -> u64 {
        self.window_start(height)
    }

    pub fn set_viewport_size(&mut self, width: usize, height: usize) {
        self.viewport_width = width.max(1);
        self.viewport_height = height.max(1);
    }

    pub fn horizontal_offset(&self) -> u64 {
        self.horizontal_offset
    }

    pub fn scroll_horizontal_right(&mut self) -> io::Result<()> {
        let start = self.window_start(self.viewport_height);
        let max_width = self
            .store
            .visible_window(start, self.viewport_height)?
            .iter()
            .map(|line| display_width(line))
            .max()
            .unwrap_or(0);
        let end_payload = (self.viewport_width as u64)
            .saturating_sub(theme::OVERFLOW_MARKER_WIDTH_CELLS)
            .max(1);
        let latest = max_width.saturating_sub(end_payload);
        self.horizontal_offset = self
            .horizontal_offset
            .saturating_add(theme::HORIZONTAL_SCROLL_STEP_CELLS)
            .min(latest);
        Ok(())
    }

    pub fn scroll_horizontal_left(&mut self) {
        self.horizontal_offset = self
            .horizontal_offset
            .saturating_sub(theme::HORIZONTAL_SCROLL_STEP_CELLS);
    }

    pub fn reset_horizontal(&mut self) {
        self.horizontal_offset = 0;
    }

    pub fn toggle_soft_wrap(&mut self) {
        self.soft_wrap = !self.soft_wrap;
    }

    pub fn soft_wrap(&self) -> bool {
        self.soft_wrap
    }

    pub fn display_window(&mut self) -> io::Result<DisplayWindow> {
        if self.store.visible_count() == 0 {
            return Ok(DisplayWindow {
                start_record: 0,
                start_cell: 0,
                records: Vec::new(),
            });
        }
        if !self.soft_wrap {
            let start_record = self.window_start(self.viewport_height);
            return Ok(DisplayWindow {
                start_record,
                start_cell: 0,
                records: self
                    .store
                    .visible_window(start_record, self.viewport_height)?,
            });
        }
        let start = self.current_display_anchor()?;
        let mut records = Vec::new();
        let mut remaining = self.viewport_height;
        let mut record = start.record;
        let mut cell = start.cell;
        while remaining > 0 && record < self.store.visible_count() {
            let Some(line) = self.visible_record(record)? else {
                break;
            };
            let row_count =
                wrapped_row_starts_from(&line, cell, self.viewport_width, remaining).len();
            records.push(line);
            remaining = remaining.saturating_sub(row_count.max(1));
            record += 1;
            cell = 0;
        }
        Ok(DisplayWindow {
            start_record: start.record,
            start_cell: start.cell,
            records,
        })
    }

    pub fn follow_latest(&mut self) {
        self.viewport_anchor = None;
    }

    pub fn is_following_latest(&self) -> bool {
        self.viewport_anchor.is_none()
    }

    fn window_start(&self, height: usize) -> u64 {
        let latest = self.store.visible_count().saturating_sub(height as u64);
        self.viewport_anchor
            .map_or(latest, |anchor| anchor.record)
            .min(latest)
    }

    fn normalize_current_find(&mut self) {
        let count = self.store.find_count();
        self.current_find = self.current_find.min(count.saturating_sub(1));
    }

    fn reveal_current_find(&mut self, reveal_content: bool) -> io::Result<()> {
        self.normalize_current_find();
        self.current_find_visible_index = self.store.find_visible_index(self.current_find)?;
        if let Some(index) = self.current_find_visible_index {
            self.viewport_anchor = Some(ViewportAnchor {
                record: index,
                cell: 0,
            });
            if reveal_content {
                if let Some(line) = self.store.visible_window(index, 1)?.first() {
                    if let Some(bytes) = first_literal_match_range(line, &self.find_query) {
                        if let Some(cells) = cell_range_for_bytes(line, &bytes) {
                            if self.soft_wrap {
                                self.viewport_anchor = Some(ViewportAnchor {
                                    record: index,
                                    cell: cells.start,
                                });
                            } else {
                                let available = (self.viewport_width as u64)
                                    .saturating_sub(theme::HORIZONTAL_MATCH_MARKER_RESERVE_CELLS)
                                    .max(1);
                                if cells.start < self.horizontal_offset {
                                    self.horizontal_offset = cells.start;
                                } else if cells.end
                                    > self.horizontal_offset.saturating_add(available)
                                {
                                    self.horizontal_offset = if cells.end - cells.start >= available
                                    {
                                        cells.start
                                    } else {
                                        cells.end - available
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn current_display_anchor(&mut self) -> io::Result<ViewportAnchor> {
        match self.viewport_anchor {
            Some(anchor) => Ok(anchor),
            None => self.latest_display_anchor(),
        }
    }

    fn latest_display_anchor(&mut self) -> io::Result<ViewportAnchor> {
        let count = self.store.visible_count();
        if count == 0 {
            return Ok(ViewportAnchor::default());
        }
        if !self.soft_wrap {
            return Ok(ViewportAnchor {
                record: count.saturating_sub(self.viewport_height as u64),
                cell: 0,
            });
        }
        let mut remaining = self.viewport_height.max(1);
        let mut record = count;
        while record > 0 {
            record -= 1;
            let line = self.visible_record(record)?.unwrap_or_default();
            let starts = wrapped_row_starts_tail(&line, self.viewport_width, remaining);
            if starts.len() >= remaining {
                return Ok(ViewportAnchor {
                    record,
                    cell: starts[0],
                });
            }
            remaining -= starts.len();
        }
        Ok(ViewportAnchor::default())
    }

    fn visible_record(&mut self, index: u64) -> io::Result<Option<String>> {
        Ok(self.store.visible_window(index, 1)?.into_iter().next())
    }
}
