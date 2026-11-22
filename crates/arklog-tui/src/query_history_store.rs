use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use super::{QueryHistory, QUERY_HISTORY_LIMIT};
use crate::text_input::MAX_INPUT_BYTES;

static NEXT_SAVE: AtomicU64 = AtomicU64::new(0);
const MAX_HISTORY_FILE_BYTES: u64 = ((MAX_INPUT_BYTES * 6 + 4) * QUERY_HISTORY_LIMIT + 2) as u64;

impl QueryHistory {
    pub fn load(path: &Path) -> io::Result<Self> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_HISTORY_FILE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_HISTORY_FILE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "regex history exceeds file size limit",
            ));
        }
        let entries: Vec<String> = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        let mut history = Self::new();
        for entry in entries {
            if !entry.chars().any(char::is_control) {
                history.record_applied(&entry);
            }
        }
        Ok(history)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let parent = path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(
            ".arklog-history-{}-{}.tmp",
            std::process::id(),
            NEXT_SAVE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        let result = (|| {
            serde_json::to_writer(&mut file, &self.entries).map_err(io::Error::other)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}
