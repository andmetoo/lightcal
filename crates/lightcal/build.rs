fn main() {
    println!("cargo:rerun-if-changed=assets/lightcal.ico");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config_files = [root.join(".env.local"), root.join(".env")];
    let contents: Vec<String> = config_files
        .iter()
        .map(|path| {
            println!("cargo:rerun-if-changed={}", path.display());
            std::fs::read_to_string(path).unwrap_or_default()
        })
        .collect();
    for key in [
        "LIGHTCAL_MICROSOFT_CLIENT_ID",
        "LIGHTCAL_UPDATE_CERT_SHA256",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
        let configured = std::env::var(key)
            .ok()
            .or_else(|| {
                contents
                    .iter()
                    .flat_map(|contents| contents.lines())
                    .filter(|line| !line.trim_start().starts_with('#'))
                    .filter_map(|line| line.trim().split_once('='))
                    .find(|(name, _)| name.trim() == key)
                    .map(|(_, value)| {
                        value
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string()
                    })
            })
            .unwrap_or_default();
        assert!(
            !configured.contains(['\r', '\n']),
            "Invalid multiline build configuration"
        );
        println!("cargo:rustc-env={key}={configured}");
    }
    slint_build::compile("ui/app.slint").expect("Failed to compile Slint UI");

    #[cfg(windows)]
    {
        let ico_path = std::path::Path::new("assets/lightcal.ico");
        if ico_path.exists() && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let mut res = winres::WindowsResource::new();
            res.set_icon("assets/lightcal.ico");
            res.set("ProductName", "Lightcal");
            res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
            res.set("FileVersion", env!("CARGO_PKG_VERSION"));
            res.set("FileDescription", "Lightcal Desktop Calendar");
            res.set("LegalCopyright", "GPL-3.0-only");
            if let Err(e) = res.compile() {
                println!("cargo:warning=Failed to compile Windows resource: {e}");
            }
        }
    }
}
