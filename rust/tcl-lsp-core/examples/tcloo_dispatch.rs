// tcl-lsp — TclOO object-dispatch resolution-rate experiment.
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Measures, over a corpus of real Tcl files, what fraction of `TclOO`
// object-method dispatch sites the semantic-token resolver resolves (colours
// the method a callable) vs leaves unresolved.  This is the quantitative
// counterpart to the `tcloo_dispatch_pattern_fixture` golden test: run it
// before/after an object-typing change to prove a resolution-rate improvement.
//
//   cargo run --release -p tcl-lsp-core --example tcloo_dispatch -- <dir|file>...
//
// A "dispatch site" is a command whose head is an object *receiver* shape:
//   - `$var method …`         (variable handle)
//   - `[dict get …] method …` / `[lindex …] …` / `[Class new] …`  (`[cmd]` head)
//   - `my method …`           (self-call in a method body)
// The method word is `argv[1]`; it is "resolved" when the token stream marks it
// a `function` (a callable), else unresolved.  Genuinely-dynamic sites are
// expected to stay unresolved — this measures coverage, not correctness (the
// fixture guards correctness / no-false-positives).

// Coverage-stat arithmetic: counts cast to f64 for percentages and to u32 for
// LSP positions; the magnitudes are tiny, so truncation/precision loss is moot.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::collections::HashMap;
use std::path::Path;

use tcl_compiler::analyser::{Analyser, ClassDef, build_class_hierarchy};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{LexerConfig, TokenType};
use tcl_registry::CommandRegistry;

#[derive(Default, Clone, Copy)]
struct Counts {
    resolved: usize,
    total: usize,
}
impl Counts {
    fn add(&mut self, resolved: bool) {
        self.total += 1;
        self.resolved += usize::from(resolved);
    }
    fn rate(self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.resolved as f64 / self.total as f64
        }
    }
}

fn main() {
    let dirs: Vec<String> = std::env::args().skip(1).collect();
    if dirs.is_empty() {
        eprintln!("usage: tcloo_dispatch <dir|file>...");
        std::process::exit(2);
    }
    let registry = CommandRegistry::build_default();
    let func_idx = tcl_lsp_core::semantic_tokens::legend_token_types()
        .iter()
        .position(|t| *t == "function")
        .expect("function token type in legend") as u32;

    // Per receiver-form and overall tallies.
    let mut by_form: HashMap<&'static str, Counts> = HashMap::new();
    let mut total = Counts::default();
    let mut files = 0usize;

    let dialect = "tcl9.0";
    let mut args = dirs;
    // `--project` builds a workspace-merged class hierarchy across all files
    // (the shipping server's cross-file mode); default is single-file local.
    let project_mode = args.first().map(String::as_str) == Some("--project");
    if project_mode {
        args.remove(0);
    }
    let mut paths = Vec::new();
    for d in &args {
        collect_tcl(Path::new(d), &mut paths);
    }

    // Project mode: first pass unions every file's classes into one hierarchy.
    let project_hierarchy = project_mode.then(|| {
        let mut merged: HashMap<String, ClassDef> = HashMap::new();
        for path in &paths {
            if let Ok(src) = std::fs::read_to_string(path) {
                for (name, class) in Analyser::new().analyse(&src, dialect).all_classes {
                    merged.entry(name).or_insert(class);
                }
            }
        }
        build_class_hierarchy(merged)
    });

    for path in &paths {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        // Only files that actually define TclOO classes are interesting.
        if !src.contains("oo::") && !src.contains("snit::") && !src.contains("itcl::") {
            continue;
        }
        files += 1;
        let cu = CompilationUnit::build_for(&src, &registry, false);
        let analysis = Analyser::new().analyse(&src, dialect);
        let st = match &project_hierarchy {
            Some(h) => tcl_lsp_core::semantic_tokens::full_with_cu_and_classes(
                &src,
                tcl_dialect::DialectProfile::by_name(dialect),
                &registry,
                Some(&cu),
                Some(h),
            ),
            None => tcl_lsp_core::semantic_tokens::full_with_cu_and_analysis(
                &src,
                tcl_dialect::DialectProfile::by_name(dialect),
                &registry,
                Some(&cu),
                Some(&analysis),
            ),
        };
        let kinds = decode_kinds(&st.data);
        let line_starts = line_starts(&src);
        let mut sites = Vec::new();
        collect_sites(&src, 0, dialect, &mut sites, 0);
        for (form, method_start) in sites {
            let (line, col) = byte_to_line_col(&line_starts, &src, method_start);
            let resolved = kinds.get(&(line, col)) == Some(&func_idx);
            by_form.entry(form).or_default().add(resolved);
            total.add(resolved);
        }
    }

    println!("# TclOO dispatch resolution rate");
    println!("files with OO: {files}   dispatch sites: {}", total.total);
    println!();
    println!("| receiver form | resolved | sites | rate |");
    println!("|---|--:|--:|--:|");
    let mut forms: Vec<_> = by_form.iter().collect();
    forms.sort_by_key(|(_, c)| std::cmp::Reverse(c.total));
    for (form, c) in forms {
        println!(
            "| {form} | {} | {} | {:.1}% |",
            c.resolved,
            c.total,
            c.rate() * 100.0
        );
    }
    println!(
        "| **all** | **{}** | **{}** | **{:.1}%** |",
        total.resolved,
        total.total,
        total.rate() * 100.0
    );
}

/// Recursively segment `text` and record each object-dispatch site as
/// `(receiver-form, method-word byte start)`.  Descends into braced scripts and
/// `[…]` substitutions so method bodies / procs / loop bodies are covered.
fn collect_sites(
    source: &str,
    base: u32,
    dialect: &str,
    out: &mut Vec<(&'static str, u32)>,
    depth: u32,
) {
    if depth > 40 {
        return;
    }
    // Segment with local spans (offset 0); `base` is the absolute byte offset of
    // `source`, added only when recording an absolute position.
    for seg in segment_commands_with_offset_and_config(source, 0, LexerConfig::for_dialect(dialect))
    {
        if let (Some(head), Some(method_tok)) = (seg.argv.first(), seg.argv.get(1))
            && let Some(form) = receiver_form(head, &seg)
        {
            out.push((form, base + method_tok.span.start()));
        }
        // Recurse into braced-script words and `[…]` substitutions.
        for (i, tok) in seg.argv.iter().enumerate() {
            if !seg.single_token_word.get(i).copied().unwrap_or(false) {
                continue;
            }
            let (co, close) = match tok.kind {
                TokenType::Str => (tok.content_offset as usize, 1usize),
                TokenType::Cmd => (tok.content_offset as usize, 0usize),
                _ => continue,
            };
            let s = tok.span.start() as usize + co;
            let e = (tok.span.end() as usize)
                .saturating_sub(close)
                .min(source.len());
            if e > s
                && let Some(inner) = source.get(s..e)
            {
                collect_sites(
                    inner,
                    base + u32::try_from(s).unwrap_or(0),
                    dialect,
                    out,
                    depth + 1,
                );
            }
        }
    }
}

/// The object-receiver form of a command head, or `None` when the head is not
/// an object dispatch (an ordinary command / bareword).
fn receiver_form(head: &tcl_lexer::Token, seg: &SegmentedCommand) -> Option<&'static str> {
    match head.kind {
        TokenType::Var => Some("$var"),
        TokenType::Cmd => Some("[cmd]"),
        TokenType::Esc if seg.texts.first().map(String::as_str) == Some("my") => Some("my"),
        _ => None,
    }
}

fn decode_kinds(data: &[u32]) -> HashMap<(u32, u32), u32> {
    let mut out = HashMap::new();
    let (mut line, mut col) = (0u32, 0u32);
    for c in data.chunks(5) {
        let [dl, dc, _len, kind, _mods] = *c else {
            break;
        };
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.insert((line, col), kind);
    }
    out
}

fn line_starts(src: &str) -> Vec<usize> {
    let mut v = vec![0usize];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            v.push(i + 1);
        }
    }
    v
}

/// Byte offset → (line, utf16-ish column).  Columns are counted in UTF-16 code
/// units to match the semantic-token encoding; corpus files are near-ASCII so
/// the approximation is exact in practice.
fn byte_to_line_col(line_starts: &[usize], src: &str, byte: u32) -> (u32, u32) {
    let byte = byte as usize;
    let line = line_starts
        .partition_point(|&s| s <= byte)
        .saturating_sub(1);
    let col = src
        .get(line_starts[line]..byte.min(src.len()))
        .map_or(0, |s| s.chars().map(|c| c.len_utf16() as u32).sum());
    (line as u32, col)
}

fn collect_tcl(p: &Path, out: &mut Vec<std::path::PathBuf>) {
    if p.is_dir() {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                collect_tcl(&e.path(), out);
            }
        }
    } else if p.extension().is_some_and(|x| x == "tcl") {
        out.push(p.to_path_buf());
    }
}
