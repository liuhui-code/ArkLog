use arklog::Clipboard;

#[derive(Default)]
pub struct SystemClipboard {
    inner: Option<arboard::Clipboard>,
}

impl SystemClipboard {
    fn inner(&mut self) -> Result<&mut arboard::Clipboard, String> {
        if self.inner.is_none() {
            self.inner = Some(arboard::Clipboard::new().map_err(|error| error.to_string())?);
        }
        Ok(self.inner.as_mut().expect("clipboard initialized"))
    }
}

impl Clipboard for SystemClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        self.inner()?.get_text().map_err(|error| error.to_string())
    }

    fn set_text(&mut self, text: &str) -> Result<(), String> {
        self.inner()?
            .set_text(text)
            .map_err(|error| error.to_string())
    }
}
