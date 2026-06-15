//! Per-edit tail profiler for the incremental diagnostic path on practcl.tcl.

#![allow(clippy::cast_precision_loss)]

use std::time::Instant;

use salsa::Setter as _;
use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, TclDb, compiler_check_diagnostics,
    file_analysis_incremental,
};

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn time<T>(label: &str, iters: u32, mut f: impl FnMut() -> T) -> f64 {
    let s = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(f());
    }
    let per = ms(s.elapsed()) / f64::from(iters);
    println!("  {label:<46} {per:8.1} ms");
    per
}

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    let rel = std::env::var("FILE")
        .unwrap_or_else(|_| "tcllib-2.0/modules/practcl/practcl.tcl".to_owned());
    let path = root.join(&rel);
    let src = std::fs::read_to_string(&path).expect("read file");
    let dialect = "tcl8.6";
    println!("== {rel} ({} lines) ==", src.lines().count());

    // Is the per-item fast path taken, or does it fall back to the full walk?
    let t_analyse = time("analyse (full whole-file walk)", 3, || {
        Analyser::new().analyse(&src, dialect)
    });
    let t_per_item = time("analyse_per_item (no memo)", 3, || {
        Analyser::new().analyse_per_item(&src, dialect)
    });
    let t_structure = time("structure_only().analyse (no diagnostics)", 3, || {
        Analyser::new().structure_only().analyse(&src, dialect)
    });
    println!(
        "  -> fallback to full walk? {}  (diagnostic-emit cost ~= {:.0} ms)",
        if (t_per_item - t_analyse).abs() < t_analyse * 0.15 {
            "YES (per_item ~= analyse)"
        } else {
            "NO (fast path)"
        },
        t_analyse - t_structure,
    );

    println!("\n== full per-edit path (salsa, memoised) ==");
    let mut db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
    );
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file);

    let edit_pos = src.find("\n    ").map_or(src.len() / 2, |p| p + 5);
    let mut edited = src.clone();
    edited.insert(edit_pos, ' ');
    let mut t = false;
    time("file_analysis_incremental (per edit)", 5, || {
        t = !t;
        file.set_text(&mut db)
            .to(if t { edited.clone() } else { src.clone() });
        file_analysis_incremental(&db, file, cfg)
    });
    let mut t2 = false;
    time("compiler_check_diagnostics (per edit)", 5, || {
        t2 = !t2;
        file.set_text(&mut db)
            .to(if t2 { edited.clone() } else { src.clone() });
        compiler_check_diagnostics(&db, file)
    });
    // Production shape: the server demands BOTH queries after one didChange, so
    // the shared `compilation_unit` build is paid once per edit (the second
    // consumer's demand is a same-revision cache hit when configs coincide).
    let mut t3 = false;
    time("BOTH queries per edit (production)", 5, || {
        t3 = !t3;
        file.set_text(&mut db)
            .to(if t3 { edited.clone() } else { src.clone() });
        let a = file_analysis_incremental(&db, file, cfg);
        let c = compiler_check_diagnostics(&db, file);
        (a, c)
    });

    println!("\n== compiler-check tail breakdown (whole-file, no memo) ==");
    let registry = db.registry(dialect);
    let cfg_lexer = tcl_lexer::LexerConfig::for_dialect(dialect);
    let build = || {
        CompilationUnit::build_for_with_config(&src, &registry, false, cfg_lexer)
            .with_interprocedural(&registry, Some(dialect))
    };
    time("CompilationUnit::build_for + interproc", 3, build);
    let cu = build();
    time("run_all_checks", 3, || {
        tcl_compiler::compiler_checks::run_all_checks(&cu, &registry, Some(dialect))
    });
    time("optimise_unit", 3, || {
        tcl_compiler::optimiser::optimise_unit(&cu, &registry, Some(dialect))
    });
    println!("  (functions in unit: {})", cu.functions().count());
}
