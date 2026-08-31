use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const MAX_INPUT_BYTES: usize = 4 * 1024;

pub trait Clipboard {
    fn get_text(&mut self) -> Result<String, String>;
    fn set_text(&mut self, text: &str) -> Result<(), String>;
}

#[derive(Clone, Copy)]
pub struct TextInputView<'a> {
    pub text: &'a str,
    pub cursor: usize,
    pub selection: Option<(usize, usize)>,
}

impl<'a> TextInputView<'a> {
    pub fn at_end(text: &'a str) -> Self {
        Self {
            text,
            cursor: text.len(),
            selection: None,
        }
    }
}

pub struct TextInput {
    text: String,
    cursor: usize,
    anchor: Option<usize>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

#[derive(Clone)]
struct Snapshot {
    text: String,
    cursor: usize,
    anchor: Option<usize>,
}

impl TextInput {
    pub fn new(text: impl Into<String>) -> Self {
        let text = bounded_single_line(&text.into(), MAX_INPUT_BYTES);
        let cursor = text.len();
        Self {
            text,
            cursor,
            anchor: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor_chars(&self) -> usize {
        self.text[..self.cursor].chars().count()
    }

    pub fn view(&self) -> TextInputView<'_> {
        TextInputView {
            text: &self.text,
            cursor: self.cursor,
            selection: self.selection(),
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.replace_selection(text);
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        clipboard: &mut impl Clipboard,
    ) -> Result<bool, String> {
        if has_primary(key.modifiers) {
            match normalized_code(key.code) {
                KeyCode::Char('a') => {
                    self.anchor = Some(0);
                    self.cursor = self.text.len();
                    return Ok(true);
                }
                KeyCode::Char('c') => {
                    if let Some((start, end)) = self.selection() {
                        clipboard.set_text(&self.text[start..end])?;
                    }
                    return Ok(true);
                }
                KeyCode::Char('x') => {
                    if let Some((start, end)) = self.selection() {
                        clipboard.set_text(&self.text[start..end])?;
                        self.replace_selection("");
                    }
                    return Ok(true);
                }
                KeyCode::Char('v') => {
                    let pasted = clipboard.get_text()?;
                    self.replace_selection(&pasted);
                    return Ok(true);
                }
                KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.redo();
                    return Ok(true);
                }
                KeyCode::Char('z') => {
                    self.undo();
                    return Ok(true);
                }
                KeyCode::Char('y') => {
                    self.redo();
                    return Ok(true);
                }
                _ => {}
            }
        }
        let selecting = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Backspace => {
                if self.selection().is_some() {
                    self.replace_selection("");
                } else {
                    let start = if has_line_modifier(key.modifiers) {
                        0
                    } else if has_word_modifier(key.modifiers) {
                        previous_word_boundary(&self.text, self.cursor)
                    } else {
                        previous_char_boundary(&self.text, self.cursor)
                    };
                    self.anchor = Some(start);
                    self.replace_selection("");
                }
                return Ok(true);
            }
            KeyCode::Delete => {
                if self.selection().is_some() {
                    self.replace_selection("");
                } else {
                    let end = if has_line_modifier(key.modifiers) {
                        self.text.len()
                    } else if has_word_modifier(key.modifiers) {
                        next_word_boundary(&self.text, self.cursor)
                    } else {
                        next_char_boundary(&self.text, self.cursor)
                    };
                    self.anchor = Some(end);
                    self.replace_selection("");
                }
                return Ok(true);
            }
            KeyCode::Left => {
                let position = if has_line_modifier(key.modifiers) {
                    0
                } else if has_word_modifier(key.modifiers) {
                    previous_word_boundary(&self.text, self.cursor)
                } else {
                    previous_char_boundary(&self.text, self.cursor)
                };
                self.move_cursor(position, selecting);
                return Ok(true);
            }
            KeyCode::Right => {
                let position = if has_line_modifier(key.modifiers) {
                    self.text.len()
                } else if has_word_modifier(key.modifiers) {
                    next_word_boundary(&self.text, self.cursor)
                } else {
                    next_char_boundary(&self.text, self.cursor)
                };
                self.move_cursor(position, selecting);
                return Ok(true);
            }
            KeyCode::Home => {
                self.move_cursor(0, selecting);
                return Ok(true);
            }
            KeyCode::End => {
                self.move_cursor(self.text.len(), selecting);
                return Ok(true);
            }
            _ => {}
        }
        if let KeyCode::Char(character) = key.code {
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER)
            {
                self.replace_selection(&character.to_string());
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn replace_selection(&mut self, replacement: &str) {
        let (start, end) = self.selection().unwrap_or((self.cursor, self.cursor));
        let retained_bytes = self.text.len() - (end - start);
        let replacement =
            bounded_single_line(replacement, MAX_INPUT_BYTES.saturating_sub(retained_bytes));
        if start == end && replacement.is_empty() {
            return;
        }
        self.push_undo();
        self.text.replace_range(start..end, &replacement);
        self.cursor = start + replacement.len();
        self.anchor = None;
    }

    fn move_cursor(&mut self, position: usize, selecting: bool) {
        if selecting {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = position;
    }

    fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        (anchor != self.cursor).then(|| {
            if anchor < self.cursor {
                (anchor, self.cursor)
            } else {
                (self.cursor, anchor)
            }
        })
    }

    fn push_undo(&mut self) {
        let snapshot = self.snapshot();
        push_bounded(&mut self.undo, snapshot);
        self.redo.clear();
    }

    fn undo(&mut self) {
        if let Some(snapshot) = self.undo.pop() {
            let current = self.snapshot();
            push_bounded(&mut self.redo, current);
            self.restore(snapshot);
        }
    }

    fn redo(&mut self) {
        if let Some(snapshot) = self.redo.pop() {
            let current = self.snapshot();
            push_bounded(&mut self.undo, current);
            self.restore(snapshot);
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            cursor: self.cursor,
            anchor: self.anchor,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.text = snapshot.text;
        self.cursor = snapshot.cursor;
        self.anchor = snapshot.anchor;
    }
}

fn has_primary(mut modifiers: KeyModifiers) -> bool {
    modifiers.remove(KeyModifiers::SHIFT);
    modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::SUPER
}

fn has_word_modifier(mut modifiers: KeyModifiers) -> bool {
    modifiers.remove(KeyModifiers::SHIFT);
    modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::ALT
}

fn has_line_modifier(mut modifiers: KeyModifiers) -> bool {
    modifiers.remove(KeyModifiers::SHIFT);
    modifiers == KeyModifiers::SUPER
}

fn normalized_code(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char(character) => KeyCode::Char(character.to_ascii_lowercase()),
        other => other,
    }
}

fn push_bounded(history: &mut Vec<Snapshot>, snapshot: Snapshot) {
    const HISTORY_LIMIT: usize = 32;
    if history.len() == HISTORY_LIMIT {
        history.remove(0);
    }
    history.push(snapshot);
}

fn previous_char_boundary(text: &str, cursor: usize) -> usize {
    text[..cursor]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn next_char_boundary(text: &str, cursor: usize) -> usize {
    text[cursor..]
        .char_indices()
        .nth(1)
        .map_or(text.len(), |(index, _)| cursor + index)
}

fn previous_word_boundary(text: &str, cursor: usize) -> usize {
    let mut position = cursor;
    while position > 0 {
        let previous = previous_char_boundary(text, position);
        if !text[previous..position].chars().all(char::is_whitespace) {
            break;
        }
        position = previous;
    }
    while position > 0 {
        let previous = previous_char_boundary(text, position);
        if text[previous..position].chars().all(char::is_whitespace) {
            break;
        }
        position = previous;
    }
    position
}

fn next_word_boundary(text: &str, cursor: usize) -> usize {
    let mut position = cursor;
    while position < text.len() {
        let next = next_char_boundary(text, position);
        if text[position..next].chars().all(char::is_whitespace) {
            break;
        }
        position = next;
    }
    while position < text.len() {
        let next = next_char_boundary(text, position);
        if !text[position..next].chars().all(char::is_whitespace) {
            break;
        }
        position = next;
    }
    position
}

fn bounded_single_line(text: &str, byte_limit: usize) -> String {
    let mut bounded = String::with_capacity(text.len().min(byte_limit));
    for character in text.chars() {
        let character = if character.is_control() {
            ' '
        } else {
            character
        };
        if bounded.len() + character.len_utf8() > byte_limit {
            break;
        }
        bounded.push(character);
    }
    bounded
}
