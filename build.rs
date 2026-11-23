// Embeds the app icon into the Windows executable so Explorer, the taskbar
// and the installer show it.
fn main() {
    println!("cargo:rerun-if-changed=assets/logo/riptide.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/logo/riptide.ico");
        res.set("ProductName", "Riptide");
        res.set("FileDescription", "Riptide BitTorrent client");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the icon: {e}");
        }
    }
}
