//! On macOS, embeds assets/Info.plist in the binary: it carries the reason
//! shown when macOS asks for access to the calendars.

fn main() {
    println!("cargo:rerun-if-changed=assets/Info.plist");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let plist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/Info.plist");
        println!(
            "cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            plist.display()
        );
    }
}
