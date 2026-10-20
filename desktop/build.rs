use std::{env, fs, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=resources/icon.ico");
        winresource::WindowsResource::new()
            .set_icon("resources/icon.ico")
            .compile()
            .expect("embed Windows application icon");
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../dist/native");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    for role in ["main", "editor", "dock"] {
        let source = root.join(role).join("bundle.js");
        println!("cargo:rerun-if-changed={}", source.display());
        let code = fs::read_to_string(&source).unwrap_or_else(|_| {
            panic!(
                "Missing {}. Run `pnpm build:ui` before building the desktop app.",
                source.display()
            )
        });
        fs::write(out.join(format!("{role}.js")), code).unwrap();
    }
}
