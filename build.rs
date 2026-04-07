fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // This code only executes when the target is Windows
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon/rs-3d.ico");
        res.compile().unwrap();
    }
}
