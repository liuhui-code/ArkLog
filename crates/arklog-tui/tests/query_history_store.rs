use arklog::{QueryHistory, TextInputView};
use std::{fs, path::PathBuf};

struct TempDirectory(PathBuf);
impl TempDirectory {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("arklog-history-{name}-{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn saved_history_survives_reopening_and_replaces_the_previous_file() {
    let directory = TempDirectory::new("roundtrip");
    let path = directory.0.join("config/regex-history.json");
    let mut history = QueryHistory::load(&path).unwrap();
    history.record_applied("[App]中文|timeout");
    history.save(&path).unwrap();
    history.record_applied("error|failure");
    history.save(&path).unwrap();

    let mut loaded = QueryHistory::load(&path).unwrap();
    assert_eq!(
        loaded.suggestion(TextInputView::at_end("[App]")),
        Some("中文|timeout")
    );
    assert_eq!(loaded.older("draft"), Some("error|failure".into()));
    assert_eq!(
        loaded.older("error|failure"),
        Some("[App]中文|timeout".into())
    );
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn rejects_oversized_files_but_bounds_and_deduplicates_valid_history() {
    let directory = TempDirectory::new("bounds");
    let path = directory.0.join("history.json");
    fs::write(&path, format!("[\"{}\"]", "x".repeat(2 * 1024 * 1024))).unwrap();
    assert!(QueryHistory::load(&path).is_err());
    let mut entries = (0..60).map(|n| format!("query-{n}")).collect::<Vec<_>>();
    entries.push("query-59".into());
    entries.push(String::new());
    entries.push("x".repeat(4097));
    fs::write(&path, serde_json::to_vec(&entries).unwrap()).unwrap();
    let mut history = QueryHistory::load(&path).unwrap();
    let mut current = String::new();
    let mut count = 0;
    while let Some(entry) = history.older(&current) {
        current = entry;
        count += 1;
    }
    assert_eq!(count, 50);
    assert_eq!(current, "query-10");
    fs::write(&path, b"broken json").unwrap();
    assert!(QueryHistory::load(&path).is_err());
}

#[test]
fn failed_save_keeps_the_destination_and_removes_the_temporary_file() {
    let directory = TempDirectory::new("failed-save");
    let target = directory.0.join("destination");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("sentinel"), "unchanged").unwrap();
    let mut history = QueryHistory::new();
    history.record_applied("error");
    assert!(history.save(&target).is_err());
    assert_eq!(
        fs::read_to_string(target.join("sentinel")).unwrap(),
        "unchanged"
    );
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
}

#[test]
fn loaded_history_cannot_insert_control_sequences_into_the_preview() {
    let directory = TempDirectory::new("control-text");
    let path = directory.0.join("history.json");
    fs::write(
        &path,
        serde_json::to_vec(&["error|timeout", "error\u{1b}[2Jbad", "error\nnext line"]).unwrap(),
    )
    .unwrap();
    let history = QueryHistory::load(&path).unwrap();
    assert_eq!(
        history.suggestion(TextInputView::at_end("err")),
        Some("or|timeout")
    );
}
