use std::{fs, path::Path};

#[test]
fn regex_suggestion_production_modules_stay_within_five_hundred_lines() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in [
        "src/main.rs",
        "src/app.rs",
        "src/app/input.rs",
        "src/app/actions.rs",
        "src/app/history.rs",
        "src/input_ui.rs",
        "src/query_history.rs",
        "src/query_history_store.rs",
        "src/state.rs",
        "src/text_input.rs",
        "src/theme.rs",
        "src/ui.rs",
        "src/ui/log_lines.rs",
    ] {
        let source = fs::read_to_string(root.join(file)).unwrap();
        assert!(source.lines().count() <= 500, "{file} exceeds 500 lines");
    }
}
