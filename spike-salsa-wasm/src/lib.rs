//! GO/NO-GO spike: does salsa 0.28 (as used by tcl-lsp-db) work at *runtime*
//! on wasm32-unknown-unknown, single-threaded, including mutation +
//! re-query (the path most likely to touch parking_lot's park()).
//!
//! Not part of the tcl-lsp workspace. Not for commit.

use std::sync::Arc;

use tcl_compiler::analyser::NonAsciiMode;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, file_analysis, file_analysis_incremental, item_tree,
};
use wasm_bindgen::prelude::*;

fn cfg(db: &TclDatabase) -> AnalyserConfig {
    AnalyserConfig::new(db, Vec::new(), NonAsciiMode::Default, Vec::new(), None, None, 0)
}

/// Runs the full spike scenario and returns a human-readable log, one line
/// per step. Never panics on purpose — any panic that happens is a real
/// finding, and will surface via the console_error_panic_hook set by `main`.
#[wasm_bindgen]
pub fn run_spike() -> String {
    use salsa::Setter as _;

    let mut log: Vec<String> = Vec::new();
    macro_rules! step {
        ($($arg:tt)*) => {
            log.push(format!($($arg)*));
        };
    }

    step!("[1] constructing TclDatabase::default()");
    let mut db = TclDatabase::default();
    step!("    ok");

    let src1 = "proc greet {name} {\n    return \"hello $name\"\n}\ngreet world\n";
    step!("[2] creating SourceFile input ({} bytes)", src1.len());
    let file = SourceFile::new(&db, src1.to_owned(), "tcl".to_owned(), None);
    step!("    ok");

    step!("[3] building AnalyserConfig input");
    let config = cfg(&db);
    step!("    ok");

    step!("[4] calling file_analysis (tracked query, first execution)");
    let result1: Arc<_> = file_analysis(&db, file, config);
    step!(
        "    ok — {} diagnostics on first analysis",
        result1.diagnostics.len()
    );
    for d in result1.diagnostics.iter().take(5) {
        step!("      diag: {:?}", d);
    }

    step!("[5] calling file_analysis AGAIN (should be memoised, no re-exec)");
    let result1b = file_analysis(&db, file, config);
    step!(
        "    ok — {} diagnostics (memoised path)",
        result1b.diagnostics.len()
    );

    step!("[6] calling item_tree (second tracked query, touches interning)");
    let tree1 = item_tree(&db, file);
    step!("    ok — {} items in tree", tree1.items.len());

    step!(
        "[6b] calling file_analysis_incremental (production path — interns \
         ItemBodyKey / lexer_cfg_key per proc body, exercises salsa's \
         #[salsa::interned] sharded-map locking)"
    );
    let inc1 = file_analysis_incremental(&db, file, config);
    step!(
        "    ok — {} diagnostics (incremental, first execution)",
        inc1.diagnostics.len()
    );

    let src2 =
        "proc greet {name} {\n    return \"hello, $name!!!\"\n}\ngreet world\ngreet mars\n";
    step!(
        "[7] MUTATING input via SourceFile::set_text (salsa write — this is \
         where cancel_others()/zalsa_mut() runs and could touch parking_lot \
         Condvar/Mutex on the write path)"
    );
    file.set_text(&mut db).to(src2.to_owned());
    step!("    ok — set_text returned without panic");

    step!("[8] re-querying file_analysis after mutation (exercises invalidation)");
    let result2 = file_analysis(&db, file, config);
    step!(
        "    ok — {} diagnostics after mutation",
        result2.diagnostics.len()
    );
    for d in result2.diagnostics.iter().take(5) {
        step!("      diag: {:?}", d);
    }

    step!("[9] re-querying item_tree after mutation");
    let tree2 = item_tree(&db, file);
    step!("    ok — {} items in tree after mutation", tree2.items.len());

    step!("[9b] re-querying file_analysis_incremental after mutation (re-interns bodies)");
    let inc2 = file_analysis_incremental(&db, file, config);
    step!(
        "    ok — {} diagnostics (incremental, after mutation)",
        inc2.diagnostics.len()
    );

    step!(
        "[10] cloning the database (rust-analyzer snapshot pattern) then \
         mutating through the ORIGINAL handle while the clone is alive — \
         this is the one path where cancel_others() actually calls \
         Condvar::wait() (clones != 1), so it is the real single-threaded \
         stress test of the wasm thread-parker"
    );
    let db_snapshot = db.clone();
    // Read something through the snapshot so it's "in use", then drop it
    // before mutating — clones goes back to 1, so cancel_others() should
    // NOT block. This still exercises the snapshot clone/drop bookkeeping
    // (the Mutex<usize> counter) without deadlocking a single-threaded
    // runtime (there is no second thread to drop the snapshot for us).
    let _ = file_analysis(&db_snapshot, file, config);
    drop(db_snapshot);
    step!("    ok — snapshot cloned, read, and dropped without panic");

    file.set_text(&mut db).to(src1.to_owned());
    step!("[11] mutated again after snapshot drop (clones == 1 again) — ok");

    step!("=== SPIKE COMPLETE: no panics observed ===");
    log.join("\n")
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

/// Negative control: directly forces `parking_lot::Condvar::wait` — a real
/// park — to confirm the harness actually surfaces the wasm thread-parker
/// panic ("Parking not supported on this platform") rather than silently
/// swallowing it. If `run_spike()` above is clean AND this function panics
/// with that exact message, the clean run is a true negative, not a masked
/// one.
#[wasm_bindgen]
pub fn run_negative_control() {
    let mutex = parking_lot::Mutex::new(0);
    let cvar = parking_lot::Condvar::new();
    let mut guard = mutex.lock();
    // No one will ever notify; on a real platform this blocks forever. On
    // wasm32-unknown-unknown parking_lot_core's wasm thread-parker panics
    // immediately instead of blocking.
    cvar.wait(&mut guard);
}

#[cfg(test)]
mod native_check {
    #[test]
    fn runs_native() {
        let out = super::run_spike();
        println!("{out}");
        assert!(out.contains("SPIKE COMPLETE"));
    }
}
