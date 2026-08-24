fn main() {
    slint_build::compile("ui/app.slint").expect("failed to compile Slint UI");

    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=assets/app-icon.ico");

    #[cfg(target_os = "windows")]
    {
        embed_resource::compile("app.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to embed the Windows application icon");
    }
}
