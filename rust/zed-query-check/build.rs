// Compile the vendored tree-sitter-tcl grammar (parser.c + external scanner)
// pinned to the rev in `vendor/REV` / `editors/zed/extension.toml`.
fn main() {
    let vendor = std::path::Path::new("vendor");
    println!("cargo:rerun-if-changed=vendor");
    cc::Build::new()
        .include(vendor)
        .file(vendor.join("parser.c"))
        .file(vendor.join("scanner.c"))
        .warnings(false)
        .compile("tree_sitter_tcl");
}
