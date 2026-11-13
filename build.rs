fn main() {
    // 嵌入 Windows 资源（.exe 图标），仅在 Windows 目标下编译
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=assets/app.rc");
        println!("cargo:rerun-if-changed=assets/zbody-icon.ico");
        embed_resource::compile("assets/app.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
