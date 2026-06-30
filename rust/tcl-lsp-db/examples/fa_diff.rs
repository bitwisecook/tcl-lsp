//! Repro for a `file_analysis_incremental` vs `file_analysis` divergence:
//! dumps the per-file difference in emitted analyser diagnostics between the
//! incremental (memoised CU + per-item body) path and the full `analyse`.
//! Usage: `FILE=tmp/tcl8.4.20/library/safe.tcl cargo run -p tcl-lsp-db --example fa_diff`.

use std::collections::BTreeSet;

use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, file_analysis, file_analysis_incremental,
};

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rel = std::env::var("FILE").expect("set FILE (relative to repo root)");
    let dialect = std::env::var("DIALECT").unwrap_or_else(|_| "tcl8.6".to_owned());
    let src = std::fs::read_to_string(root.join(&rel)).expect("read FILE");

    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
    );
    let file = SourceFile::new(&db, src.clone(), dialect.clone());
    let inc = file_analysis_incremental(&db, file, cfg);
    let full = file_analysis(&db, file, cfg);

    let key = |d: &tcl_compiler::analyser::Diagnostic| {
        (d.span.start(), d.span.end(), d.code, d.message.clone())
    };
    let is: BTreeSet<_> = inc.diagnostics.iter().map(key).collect();
    let fs: BTreeSet<_> = full.diagnostics.iter().map(key).collect();
    println!(
        "DIAGS: inc={} full={} | {} inc-only, {} full-only",
        inc.diagnostics.len(),
        full.diagnostics.len(),
        is.difference(&fs).count(),
        fs.difference(&is).count(),
    );
    for k in is.difference(&fs).take(20) {
        println!("  inc-only  {k:?}");
    }
    for k in fs.difference(&is).take(20) {
        println!("  full-only {k:?}");
    }

    // Order-only divergence check: same multiset, different sequence?
    let iv: Vec<_> = inc.diagnostics.iter().map(&key).collect();
    let fv: Vec<_> = full.diagnostics.iter().map(&key).collect();
    if is == fs && iv != fv {
        println!("ORDER-ONLY divergence (same set, different sequence)");
        for (i, (a, b)) in iv.iter().zip(fv.iter()).enumerate() {
            if a != b {
                println!("  first diff at {i}: inc={a:?} full={b:?}");
                break;
            }
        }
    }
}
