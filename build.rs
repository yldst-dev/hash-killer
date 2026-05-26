fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let icon_path = manifest_dir.join("resources/hashkiller.ico");
        let manifest_path = manifest_dir.join("resources/app.manifest");
        let rc_path = out_dir.join("app.rc");
        let icon_path = icon_path.to_string_lossy().replace('\\', "\\\\");
        let manifest_path = manifest_path.to_string_lossy().replace('\\', "\\\\");
        std::fs::write(
            &rc_path,
            format!("1 ICON \"{icon_path}\"\n2 RT_MANIFEST \"{manifest_path}\"\n"),
        )
        .unwrap();
        embed_resource::compile(rc_path, embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
