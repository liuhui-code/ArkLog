use std::fs;
use std::path::PathBuf;

#[test]
fn release_package_version_is_v0_2_6() {
    assert_eq!(env!("CARGO_PKG_VERSION"), "0.2.6");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let package = fs::read_to_string(root.join("package.json")).expect("root package metadata");
    assert!(package.contains("\"version\": \"0.2.6\""));
}

#[test]
fn version_tag_builds_and_publishes_windows_and_macos_executables() {
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
        "runs-on: ${{ matrix.os }}",
        "os: windows-latest",
        "os: macos-15-intel",
        "os: macos-15",
        "permissions:",
        "contents: write",
        "uses: pnpm/action-setup@v6",
        "run: pnpm test",
        "run: pnpm build",
        "run: cargo test --workspace",
        "run: cargo check -p arklog",
        "run: pnpm memory:check",
        "target/release/arklog.exe",
        "target/release/arklog",
        "ArkLog-windows-x86_64.exe",
        "ArkLog-macos-x86_64",
        "ArkLog-macos-aarch64",
        "if-no-files-found: error",
        "needs: build",
        "uses: actions/download-artifact@v8",
        "pattern: ArkLog-*",
        "merge-multiple: true",
        "GH_REPO: ${{ github.repository }}",
        "gh release create",
    ] {
        assert!(
            workflow.contains(required_behavior),
            "release workflow is missing required behavior: {required_behavior}"
        );
    }
}

#[test]
fn pushes_and_pull_requests_verify_macos_and_windows() {
    let workflow_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).unwrap_or_else(|error| {
        panic!(
            "CI workflow must exist at {}: {error}",
            workflow_path.display()
        )
    });

    for required_behavior in [
        "pull_request:",
        "push:",
        "macos-latest",
        "windows-latest",
        "run: pnpm test",
        "run: pnpm build",
        "run: cargo test --workspace",
        "run: cargo check -p arklog",
        "run: pnpm memory:check",
    ] {
        assert!(
            workflow.contains(required_behavior),
            "CI workflow is missing required behavior: {required_behavior}"
        );
    }
}

#[test]
fn windows_release_embeds_the_multisize_arklog_application_icon() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let icon_path = root.join("assets/icons/icon.ico");
    let icon = fs::read(&icon_path).unwrap_or_else(|error| {
        panic!(
            "release icon must exist at {}: {error}",
            icon_path.display()
        )
    });
    assert!(icon.len() >= 6, "release icon must contain an ICO header");
    assert_eq!(&icon[..4], &[0, 0, 1, 0], "release icon must be ICO");
    assert!(
        u16::from_le_bytes([icon[4], icon[5]]) >= 4,
        "release icon must contain multiple desktop sizes"
    );

    let manifest =
        fs::read_to_string(root.join("crates/arklog-tui/Cargo.toml")).expect("ArkLog manifest");
    let build_script =
        fs::read_to_string(root.join("crates/arklog-tui/build.rs")).expect("ArkLog build script");
    assert!(manifest.contains("build = \"build.rs\""));
    assert!(manifest.contains("winresource"));
    assert!(build_script.contains("CARGO_CFG_TARGET_OS"));
    assert!(build_script.contains("HOST"));
    assert!(build_script.contains("../../assets/icons/icon.ico"));
}
