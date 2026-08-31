fn main() {
    const ICON: &str = "../../assets/icons/icon.ico";
    println!("cargo:rerun-if-changed={ICON}");

    let targets_windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let runs_on_windows = std::env::var("HOST").is_ok_and(|host| host.contains("windows"));
    if !targets_windows || !runs_on_windows {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(ICON)
        .set("FileDescription", "ArkLog live HarmonyOS log viewer")
        .set("ProductName", "ArkLog")
        .set("OriginalFilename", "arklog.exe");
    resource.compile().expect("embed ArkLog Windows icon");
}
