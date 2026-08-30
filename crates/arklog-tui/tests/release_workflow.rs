use std::fs;
use std::path::PathBuf;

#[test]
fn version_tag_builds_and_publishes_a_windows_executable() {
    let workflow_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/release-windows.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap_or_else(|error| {
        panic!(
            "release workflow must exist at {}: {error}",
            workflow_path.display()
        )
    });

    for required_behavior in [
        "tags:",
        "- \"v*\"",
        "runs-on: windows-latest",
        "permissions:",
        "contents: write",
        "uses: pnpm/action-setup@v6",
        "run: pnpm test",
        "run: pnpm build",
        "run: cargo test --workspace",
        "run: cargo check -p arklog",
        "target/release/arklog.exe",
        "if-no-files-found: error",
        "gh release create",
    ] {
        assert!(
            workflow.contains(required_behavior),
            "release workflow is missing required behavior: {required_behavior}"
        );
    }
}
