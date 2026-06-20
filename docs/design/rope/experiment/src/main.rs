//! SRV-ROPE need-evaluation experiment.
//!
//! Measures, across file sizes and edit-burst sizes, the three quantities that
//! decide whether a rope-backed `DocumentState` is worth it:
//!
//!   * `string`     — the *current* server path: rebuild `LineIndex` + splice a
//!                    fresh `String` per content change (`apply_content_change`).
//!   * `rope_edit`  — apply the same edits to a persistent `Rope` (no flatten).
//!   * `rope_full`  — `rope_edit` + `Rope::to_string()` (the flatten the salsa
//!                    `SourceFile` input forces, since it stores a `String`).
//!
//! Plus: the isolated flatten tax, a high-edit-rate loop, a many-small-document
//! memory footprint, and the per-request position-lookup cost.
//!
//! Everything is ASCII so byte == char == UTF-16 code unit; that keeps the two
//! arms doing the *same logical work* and isolates the structural (rope vs
//! flat-buffer) difference, which is what the decision turns on.

use std::time::Instant;

use tcl_lexer::LineIndex;

/// Generate `target` bytes of Tcl-ish source (newline-terminated lines).
fn gen_source(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    let mut i = 0u64;
    while s.len() < target {
        // ~40-byte lines, varied so line lengths aren't all identical.
        s.push_str(&format!("set var_{i} [expr {{$base_{} + {i}}}]\n", i % 7));
        i += 1;
    }
    s
}

/// The production edit path: one LSP ranged content change applied to `text`,
/// rebuilding the line index exactly as `server::apply_content_change` does.
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

/// Median of repeated timings (ns) of `f`, run `iters` times after a warm-up.
fn time_ns(iters: u32, mut f: impl FnMut()) -> f64 {
    f(); // warm up
    let mut samples = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let t = Instant::now();
        f();
        samples.push(t.elapsed().as_nanos() as f64);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
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

/// Scale iteration count down as files grow so each row finishes quickly.
fn iters_for(size: usize) -> u32 {
    match size {
        s if s <= 16 << 10 => 2000,
        s if s <= 256 << 10 => 400,
        s if s <= 1 << 20 => 80,
        _ => 20,
    }
}

fn bench_edits() {
    println!("== Edit application: one `didChange` carrying B edits ==");
    println!("(ns per didChange; lower is better. rope persists across edits.)\n");
    println!(
        "{:>7} {:>4} {:>12} {:>12} {:>12} {:>12} {:>10}",
        "size", "B", "string", "rope_edit", "flatten", "rope_full", "full ÷ str"
    );
    let sizes = [1 << 10, 16 << 10, 256 << 10, 1 << 20, 4 << 20];
    let bursts = [1u32, 8, 64];
    for &size in &sizes {
        let base = gen_source(size);
        let iters = iters_for(size);
        for &b in &bursts {
            // Edits target the middle line; for ASCII, col 0 of that line.
            let line_count = base.bytes().filter(|&c| c == b'\n').count() as u32;
            let mid = line_count / 2;

            // -- string arm: B sequential splices, each rebuilding LineIndex --
            let string_ns = time_ns(iters, || {
                let mut t = base.clone();
                for k in 0..b {
                    t = string_apply(&t, mid + k % 3, 0, "set _tmp 0\n");
                }
                std::hint::black_box(&t);
            });

            // -- rope arms: persistent rope, B edits, optional flatten --
            // The rope is built ONCE (a `DocumentState` holds a live rope
            // between edits); each iteration starts from an O(1) Arc-share
            // clone of it, the faithful steady-state model — not a per-edit
            // `from_str` rebuild.
            let persistent = ropey::Rope::from_str(&base);
            let rope_edit_ns = time_ns(iters, || {
                let mut r = persistent.clone();
                for k in 0..b {
                    let ci = r.line_to_char((mid + k % 3) as usize);
                    r.insert(ci, "set _tmp 0\n");
                }
                std::hint::black_box(r.len_bytes());
            });
            let rope_full_ns = time_ns(iters, || {
                let mut r = persistent.clone();
                for k in 0..b {
                    let ci = r.line_to_char((mid + k % 3) as usize);
                    r.insert(ci, "set _tmp 0\n");
                }
                let s = r.to_string(); // the salsa `set_text(String)` tax
                std::hint::black_box(s.len());
            });
            let flatten_ns = (rope_full_ns - rope_edit_ns).max(0.0);

            println!(
                "{:>7} {:>4} {:>12.0} {:>12.0} {:>12.0} {:>12.0} {:>9.2}x",
                human(size),
                b,
                string_ns,
                rope_edit_ns,
                flatten_ns,
                rope_full_ns,
                rope_full_ns / string_ns,
            );
        }
    }
    println!();
}

fn bench_high_edit_rate() {
    println!("== High edit rate: 500 sequential single-edit didChanges ==");
    println!("(total ms; each didChange must hand salsa a fresh String.)\n");
    println!(
        "{:>7} {:>14} {:>14} {:>10}",
        "size", "string(ms)", "rope_full(ms)", "speedup"
    );
    for &size in &[16 << 10, 256 << 10, 1 << 20] {
        let base = gen_source(size);
        let line_count = base.bytes().filter(|&c| c == b'\n').count() as u32;
        let mid = line_count / 2;
        const N: u32 = 500;

        let t = Instant::now();
        {
            let mut s = base.clone();
            for k in 0..N {
                s = string_apply(&s, mid + k % 5, 0, "x ");
            }
            std::hint::black_box(s.len());
        }
        let string_ms = t.elapsed().as_secs_f64() * 1e3;

        let t = Instant::now();
        {
            let mut r = ropey::Rope::from_str(&base);
            for k in 0..N {
                let ci = r.line_to_char((mid + k % 5) as usize);
                r.insert(ci, "x ");
                let s = r.to_string(); // salsa still needs the String each edit
                std::hint::black_box(s.len());
            }
        }
        let rope_ms = t.elapsed().as_secs_f64() * 1e3;

        println!(
            "{:>7} {:>14.1} {:>14.1} {:>9.2}x",
            human(size),
            string_ms,
            rope_ms,
            string_ms / rope_ms,
        );
    }
    println!();
}

fn bench_position_lookup() {
    println!("== Position lookup: byte offset -> (line, col) for a request ==");
    println!("(ns per lookup; the `lift_span` / hover / definition per-call cost.)\n");
    println!("{:>7} {:>16} {:>16} {:>10}", "size", "LineIndex::new", "rope(persist)", "ratio");
    for &size in &[16 << 10, 256 << 10, 1 << 20, 4 << 20] {
        let base = gen_source(size);
        let off = (base.len() / 2) as u32;
        let iters = iters_for(size);
        // String: server rebuilds LineIndex per request batch (cold).
        let str_ns = time_ns(iters, || {
            let idx = LineIndex::new(&base);
            let p = idx.position_at_utf16(off, &base);
            std::hint::black_box((p.line, p.character));
        });
        // Rope: persistent, no rebuild — byte_to_line + column.
        let rope = ropey::Rope::from_str(&base);
        let rope_ns = time_ns(iters, || {
            let line = rope.byte_to_line(off as usize);
            let line_start = rope.line_to_byte(line);
            std::hint::black_box((line, off as usize - line_start));
        });
        println!(
            "{:>7} {:>16.0} {:>16.0} {:>9.2}x",
            human(size),
            str_ns,
            rope_ns,
            str_ns / rope_ns,
        );
    }
    println!();
}

fn bench_memory_many_small() {
    println!("== Memory: many small open documents (String vs Rope) ==");
    println!("(heap bytes for N live documents; rope carries B-tree overhead.)\n");
    println!("{:>6} {:>9} {:>14} {:>14} {:>10}", "N", "file", "strings", "ropes", "rope ÷ str");
    for &(n, fsize) in &[(1000usize, 2 << 10), (5000, 1 << 10), (200, 16 << 10)] {
        let base = gen_source(fsize);
        let strings: Vec<String> = (0..n).map(|_| base.clone()).collect();
        let str_heap: usize = strings.iter().map(String::capacity).sum();
        let ropes: Vec<ropey::Rope> = (0..n).map(|_| ropey::Rope::from_str(&base)).collect();
        // ropey exposes capacity() (total bytes the rope's leaves can hold).
        let rope_heap: usize = ropes.iter().map(ropey::Rope::capacity).sum();
        println!(
            "{:>6} {:>9} {:>14} {:>14} {:>9.2}x",
            n,
            human(fsize),
            human(str_heap),
            human(rope_heap),
            rope_heap as f64 / str_heap.max(1) as f64,
        );
        std::hint::black_box((strings.len(), ropes.len()));
    }
    println!();
}

fn main() {
    println!("\nSRV-ROPE need-evaluation — ropey {} vs String\n", env!("CARGO_PKG_VERSION"));
    println!("CPU-bound microbenchmarks; numbers are indicative, not absolute.\n");
    bench_edits();
    bench_high_edit_rate();
    bench_position_lookup();
    bench_memory_many_small();
    println!("Done.");
}
