// Compile every tree-sitter grammar whose Zed queries this crate validates:
//
//   * the vendored `tree-sitter-tcl`, pinned to the rev in `vendor/REV` /
//     `editors/zed/extension.toml`;
//   * the two grammars this repo owns — `tree-sitter-apl` and
//     `tree-sitter-bigip` (issue #903), which back Zed's APL and TMSH
//     languages. Those languages used to point at the *Tcl* grammar and ship no
//     queries at all, so this check only ever saw one grammar.
fn main() {
    let vendor = std::path::Path::new("vendor");
    println!("cargo:rerun-if-changed=vendor");
    cc::Build::new()
        .include(vendor)
        .file(vendor.join("parser.c"))
        .file(vendor.join("scanner.c"))
        .warnings(false)
        .compile("tree_sitter_tcl");

    for (name, dir) in [
        ("tree_sitter_apl", "../../grammars/tree-sitter-apl"),
        ("tree_sitter_bigip", "../../grammars/tree-sitter-bigip"),
    ] {
        let src = std::path::Path::new(dir).join("src");
        println!("cargo:rerun-if-changed={}", src.display());
        let mut build = cc::Build::new();
        build
            .include(&src)
            .file(src.join("parser.c"))
            .warnings(false);
        let scanner = src.join("scanner.c");
        if scanner.is_file() {
            build.file(scanner);
        }
        build.compile(name);
    }
}
