use std::env::var;
use winresource::WindowsResource;

fn main() {
    if var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut resource = WindowsResource::new();
        resource.set_icon("icon.ico");
        resource.compile().unwrap();
    }
}
