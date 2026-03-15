use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out_dir);

    for (name, cfg) in [
        ("tcl-lsp-server.pyz", "bundled_lsp"),
        ("tcl-lsp-mcp-server.pyz", "bundled_mcp"),
    ] {
        println!("cargo::rustc-check-cfg=cfg({cfg})");
        let src = Path::new("bundled").join(name);
        println!("cargo:rerun-if-changed={}", src.display());
        if src.exists() {
            fs::copy(&src, out.join(name)).expect("failed to copy bundled asset");
            println!("cargo:rustc-cfg={cfg}");
        }
    }
}
