// Temporary vm-runtime probe; deleted after use. Renders every shipped pack
// exactly as `cargo xtask pack-goldens` does (golden::render(evaluate_pack)).
#[test]
fn zz_goldens() {
    use tcl_spectcl::golden;
    let Some(out) = std::env::var_os("ZZ_OUT") else { return };
    let out = std::path::PathBuf::from(out);
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::create_dir_all(&out).unwrap();
    for path in golden::shipped_packs(&root) {
        let source = std::fs::read_to_string(&path).unwrap();
        let pack = tcl_spectcl::evaluate_pack(&source);
        let fresh = golden::render(&pack);
        let golden_path = golden::golden_path(&root, &path);
        let name = golden_path.file_name().unwrap().to_owned();
        std::fs::write(out.join(&name), &fresh).unwrap();
        let recorded = std::fs::read_to_string(&golden_path).unwrap_or_default();
        let mut renders = String::new();
        for (a, b) in recorded.lines().zip(fresh.lines()) {
            if a != b
                && let Some(cmd) = b.strip_prefix("command ").and_then(|r| r.split(' ').next())
                && let Some(r) = golden::spec_rendering(&pack, cmd)
            {
                renders.push_str(cmd);
                renders.push('\t');
                renders.push_str(&r);
                renders.push('\n');
            }
        }
        std::fs::write(out.join(name).with_extension("renders"), renders).unwrap();
    }
}
