// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SRV-ROPE decision: per-edit *pipeline* cost vs. the apply-side a rope helps.
//!
//! The sibling `experiment/` proved a rope is 5–12× faster at **edit
//! application + flatten** measured in isolation. The SRV-ROPE doc then asserts
//! a "governing caveat": that apply is only ~5–15% of real per-edit latency
//! because re-lex + salsa invalidation + diagnostics dominate, so the rope's
//! isolated win is a single-digit-% end-to-end improvement at best. That caveat
//! is the linchpin of the whole decision and it was never measured.
//!
//! This harness measures it directly, against the **real** production code:
//!
//!   * `apply`      — `apply_content_change`: `LineIndex::new` + splice a fresh
//!                    `String` (exactly what the rope would replace).
//!   * `relex`      — `Lexer::tokenise_all` over the whole buffer.
//!   * `analyse`    — `Analyser::analyse` from scratch (cold full analysis).
//!   * `salsa_edit` — the real per-edit server work: `SourceFile::set_text`
//!                    then the two memoised queries the server runs
//!                    (`file_analysis_incremental` + `compiler_check_diagnostics`)
//!                    on a WARM db, applying a realistic single-body edit.
//!
//! The decisive number is the last column: `apply ÷ (apply + salsa_edit)` — the
//! fraction of per-edit latency a rope could even theoretically remove. If that
//! is small, the rope cannot move the needle and the caveat holds.
//!
//! Two source shapes bracket reality:
//!   * `procs` — many small procs; the per-item firewall's BEST case (a body
//!     edit recomputes one item), the shape most favourable to "no rope needed".
//!   * `flat`  — top-level statements; no proc bodies to firewall, so the edit
//!     re-analyses the whole top level (apply fraction is even smaller).

use std::hint::black_box;
use std::time::Instant;

use salsa::Setter as _;
use tcl_compiler::analyser::{Analyser, NonAsciiMode};
use tcl_lexer::{Lexer, LineIndex};
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, compiler_check_diagnostics, file_analysis_incremental,
};

const DIALECT: &str = "tcl8.6";

/// Many small procs — exercises the per-item incremental firewall.
/// `salt` perturbs proc `salted`'s body literal (a realistic single-body edit).
fn gen_procs(target: usize, salted: usize, salt: u64) -> String {
    let mut s = String::with_capacity(target + 128);
    let mut i = 0usize;
    while s.len() < target {
        let lit = if i == salted { i as u64 + salt } else { i as u64 };
        s.push_str(&format!(
            "proc p_{i} {{a b}} {{\n    set x [expr {{$a + $b + {lit}}}]\n    return $x\n}}\n"
        ));
        i += 1;
    }
    s
}

/// Flat top-level statements — no proc bodies, so an edit re-analyses the whole
/// top level (the per-item firewall has nothing to firewall).
fn gen_flat(target: usize, salted: usize, salt: u64) -> String {
    let mut s = String::with_capacity(target + 128);
    let mut i = 0usize;
    while s.len() < target {
        let lit = if i == salted { i as u64 + salt } else { i as u64 };
        s.push_str(&format!("set var_{i} [expr {{$base_{} + {lit}}}]\n", i % 7));
        i += 1;
    }
    s
}

/// The production edit path: one ranged content change, rebuilding the line
/// index exactly as `server::apply_content_change` does.
fn string_apply(text: &str, line: u32, character: u32, new_text: &str) -> String {
    let index = LineIndex::new(text);
    let a = index.offset_at_utf16(line, character, text) as usize;
    let len = text.len();
    let start = a.min(len);
    let mut out = String::with_capacity(len + new_text.len());
    out.push_str(&text[..start]);
    out.push_str(new_text);
    out.push_str(&text[start..]);
    out
}

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
}

fn time_ns(iters: u32, mut f: impl FnMut()) -> f64 {
    f(); // warm up
    let mut samples = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let t = Instant::now();
        f();
        samples.push(t.elapsed().as_nanos() as f64);
    }
    median(samples)
}

fn human(bytes: usize) -> String {
    if bytes >= 1 << 20 {
        format!("{}MiB", bytes >> 20)
    } else if bytes >= 1 << 10 {
        format!("{}KiB", bytes >> 10)
    } else {
        format!("{bytes}B")
    }
}

fn iters_for(size: usize) -> u32 {
    match size {
        s if s <= 16 << 10 => 400,
        s if s <= 64 << 10 => 120,
        _ => 40,
    }
}

/// Measure the real warm-db per-edit analysis cost: cycle a single-body edit
/// across the procs and time `set_text` + the two queries the server runs.
fn measure_salsa_edit(shape: &str, size: usize, n_items: usize, iters: u32) -> f64 {
    let mut db = TclDatabase::default();
    let config = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
    let gen = |salted: usize, salt: u64| -> String {
        if shape == "procs" {
            gen_procs(size, salted, salt)
        } else {
            gen_flat(size, salted, salt)
        }
    };
    let file = SourceFile::new(&db, gen(0, 0), DIALECT.to_owned());
    // Warm: full cold build so the loop measures incremental reuse.
    let _ = file_analysis_incremental(&db, file, config);
    let _ = compiler_check_diagnostics(&db, file);

    let mut samples = Vec::with_capacity(iters as usize);
    for k in 1..=iters {
        // Build the edited text OUTSIDE the timed region — we are timing the
        // analysis the edit forces, not the splice (measured separately).
        let salted = (k as usize) % n_items.max(1);
        let text = gen(salted, k as u64 + 1);
        let t = Instant::now();
        file.set_text(&mut db).to(text);
        let a = file_analysis_incremental(&db, file, config);
        let c = compiler_check_diagnostics(&db, file);
        samples.push(t.elapsed().as_nanos() as f64);
        black_box((a.diagnostics.len(), c.checks.len()));
    }
    median(samples)
}

fn main() {
    println!("\nSRV-ROPE per-edit pipeline experiment — real analyser + salsa db\n");
    println!("Question: what fraction of per-edit latency is buffer apply (the only");
    println!("part a rope speeds up)? If small, the rope cannot move the needle.\n");

    let sizes = [4 << 10, 16 << 10, 64 << 10, 256 << 10];
    for shape in ["procs", "flat"] {
        println!("== shape = {shape} ==");
        println!(
            "{:>7} {:>9} {:>10} {:>12} {:>12} {:>10}",
            "size", "apply", "relex", "analyse", "salsa_edit", "apply%"
        );
        for &size in &sizes {
            let base = if shape == "procs" {
                gen_procs(size, usize::MAX, 0)
            } else {
                gen_flat(size, usize::MAX, 0)
            };
            let n_items = base.matches("proc p_").count().max(base.lines().count());
            let iters = iters_for(size);
            let line_count = base.bytes().filter(|&c| c == b'\n').count() as u32;
            let mid = line_count / 2;

            let apply_ns = time_ns(iters, || {
                let t = string_apply(&base, mid, 0, "set _t 0\n");
                black_box(t.len());
            });
            let relex_ns = time_ns(iters, || {
                let toks = Lexer::new(&base).tokenise_all().expect("lex");
                black_box(toks.len());
            });
            let analyse_ns = time_ns(iters.min(120), || {
                let mut an = Analyser::new();
                let r = an.analyse(&base, DIALECT);
                black_box(r.diagnostics.len());
            });
            let salsa_ns = measure_salsa_edit(shape, size, n_items, iters);

            let apply_pct = 100.0 * apply_ns / (apply_ns + salsa_ns);
            println!(
                "{:>7} {:>9.0} {:>10.0} {:>12.0} {:>12.0} {:>9.2}%",
                human(size),
                apply_ns,
                relex_ns,
                analyse_ns,
                salsa_ns,
                apply_pct,
            );
        }
        println!();
    }
    println!("apply%  = apply ÷ (apply + salsa_edit): the ceiling on a rope's");
    println!("          per-edit win. relex/analyse shown for context (cold, full-buffer).");
    println!("Done.");
}
