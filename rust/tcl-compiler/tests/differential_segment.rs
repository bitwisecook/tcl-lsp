//! Differential harness: CST-derived segmenter vs the token-loop oracle.
//!
//! `CST-PORT` strip 5 verification.  The canonical red-green CST
//! (`parsing::syntax::{build,red,segment}`) must derive a
//! `SegmentedCommand` list **byte-identical** to the current token-loop
//! `segment_commands_local` (the oracle), field-for-field, plus:
//!
//! - **losslessness** — the CST's reconstructed text equals the source;
//! - **position-equivalence** — the red fragment tokens reproduce the
//!   lexer's exact token spans (covered transitively by `all_tokens`
//!   parity, since each CST `all_tokens` entry is a red `to_token()`).
//!
//! Verified over a crafted edge-case table and, when present, the
//! `tmp/tcl{8.4,8.5,8.6,9.0}` source corpus.  The harness is the gate the
//! task requires green *before* the segmenter's internals are flipped to
//! the CST; it stays as a permanent regression net afterwards.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use tcl_lexer::{LexerConfig, SourceMap};

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::red::SyntaxTree;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_compiler::segmenter::{segment_commands_with_offset_and_config, SegmentedCommand};

/// Repo root — two directories above `CARGO_MANIFEST_DIR`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The token-loop oracle (local-offset space, `base_offset = 0`).
fn oracle(src: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
    segment_commands_with_offset_and_config(src, 0, config)
}

/// The CST-derived segmenter (local-offset space).
fn cst(src: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
    let sm = SourceMap::new(src);
    let (doc, _warnings) = build_document(src, config);
    segments_from_document(doc, &sm)
}

/// Assert two `SegmentedCommand`s are equal field-for-field.
fn assert_seg_eq(o: &SegmentedCommand, c: &SegmentedCommand, ctx: &str) {
    assert_eq!(o.span, c.span, "span mismatch [{ctx}]");
    assert_eq!(o.argv, c.argv, "argv mismatch [{ctx}]");
    assert_eq!(o.texts, c.texts, "texts mismatch [{ctx}]");
    assert_eq!(
        o.single_token_word, c.single_token_word,
        "single_token_word mismatch [{ctx}]"
    );
    assert_eq!(o.all_tokens, c.all_tokens, "all_tokens mismatch [{ctx}]");
    assert_eq!(o.is_partial, c.is_partial, "is_partial mismatch [{ctx}]");
    assert_eq!(o.expand_word, c.expand_word, "expand_word mismatch [{ctx}]");
    assert_eq!(
        o.preceding_comment, c.preceding_comment,
        "preceding_comment mismatch [{ctx}]"
    );
}

/// Compare CST vs oracle over `src` for a given dialect config, and check
/// losslessness.  Returns `Ok(())` or a descriptive error string.
fn check(src: &str, config: LexerConfig, ctx: &str) -> Result<(), String> {
    let o = oracle(src, config);
    let c = cst(src, config);
    if o.len() != c.len() {
        return Err(format!(
            "command count mismatch [{ctx}]: oracle {} vs cst {}",
            o.len(),
            c.len()
        ));
    }
    for (i, (oc, cc)) in o.iter().zip(c.iter()).enumerate() {
        // Use catch via manual field comparison so the corpus sweep can
        // accumulate rather than panicking on the first file.
        if oc.span != cc.span {
            return Err(format!(
                "span mismatch [{ctx}] cmd {i}: {:?} vs {:?}",
                oc.span, cc.span
            ));
        }
        if oc.argv != cc.argv {
            return Err(format!("argv mismatch [{ctx}] cmd {i}"));
        }
        if oc.texts != cc.texts {
            return Err(format!(
                "texts mismatch [{ctx}] cmd {i}: {:?} vs {:?}",
                oc.texts, cc.texts
            ));
        }
        if oc.single_token_word != cc.single_token_word {
            return Err(format!("single_token_word mismatch [{ctx}] cmd {i}"));
        }
        if oc.all_tokens != cc.all_tokens {
            return Err(format!("all_tokens mismatch [{ctx}] cmd {i}"));
        }
        if oc.is_partial != cc.is_partial {
            return Err(format!("is_partial mismatch [{ctx}] cmd {i}"));
        }
        if oc.expand_word != cc.expand_word {
            return Err(format!("expand_word mismatch [{ctx}] cmd {i}"));
        }
        if oc.preceding_comment != cc.preceding_comment {
            return Err(format!("preceding_comment mismatch [{ctx}] cmd {i}"));
        }
    }
    // Losslessness: the anchored tree's text reproduces the source.
    let (doc, _) = build_document(src, config);
    let tree = SyntaxTree::new(doc);
    if tree.text() != src {
        return Err(format!("losslessness mismatch [{ctx}]"));
    }
    Ok(())
}

/// A spread of crafted sources exercising the segmenter's quirks.
const EDGE_CASES: &[&str] = &[
    "",
    "puts hi",
    "puts hi\n",
    "puts hi\n\n",
    "set x 1\nputs $x\n",
    "if {$x} {body}",
    "if {$x} {\n  puts yes\n}\n",
    "proc f {} {}",
    "set x {}",
    "set x {}\n",
    "a {}}",          // empty {} followed by a stray `}` word
    "puts \"hello\"", // quoted last word — the closer-convention edge
    "puts \"hello\"\n",
    "puts \"hi\" tail", // quoted non-last word
    "foo {*}$args",     // expand
    "foo {*}{*}$args",  // double expand
    "foo {*}",          // trailing `{*}` is a literal Str, not an expand
    "list a \\\n b",    // backslash-newline continuation
    "a; b; c",          // semicolon separators
    "puts [expr {1 + 2}]",
    "set x \"a\nb\nc\"", // multi-line quoted
    "  indented puts hi  \n",
    "# a comment\nputs hi\n",
    "# c1\n# c2\nproc f {} {}\n",
    "# orphan\n\nputs hi\n", // blank-line comment reset
    "puts hi ;# dangling",   // dangling trailing comment
    "set v $arr(idx)",
    "set v ${name}",
    "puts $a$b", // compound var word
    "namespace eval ns {\n  proc g {} {}\n}\n",
    "switch $x {\n a {one}\n b {two}\n}\n",
];

#[test]
fn cst_matches_oracle_on_edge_cases() {
    for &src in EDGE_CASES {
        for (name, config) in [
            ("default", LexerConfig::default()),
            ("tcl8.4", LexerConfig::for_dialect("tcl8.4")),
            ("f5-irules", LexerConfig::for_dialect("f5-irules")),
        ] {
            let ctx = format!("{name}:{src:?}");
            if let Err(msg) = check(src, config, &ctx) {
                panic!("{msg}");
            }
        }
    }
}

#[test]
fn cst_matches_oracle_with_nonzero_base_offset() {
    // Both paths derive in local-offset space and relocate via
    // `shifted_by` (the segmenter never runs `word_piece` / `command_span`
    // on shifted tokens — `sm` is local).  A non-zero base must therefore
    // apply the same uniform shift on both sides.
    let src = "if {$x} {body}\nputs \"q\"\n";
    let base = 100u32;
    let oracle_shifted = segment_commands_with_offset_and_config(src, base, LexerConfig::default());

    let cst_shifted: Vec<SegmentedCommand> = cst(src, LexerConfig::default())
        .into_iter()
        .map(|c| c.shifted_by(base))
        .collect();

    assert_eq!(oracle_shifted.len(), cst_shifted.len());
    for (o, c) in oracle_shifted.iter().zip(cst_shifted.iter()) {
        assert_seg_eq(o, c, "nonzero-base");
    }
}

/// Recursively collect `.tcl` files under `dir`.
fn gather_tcl(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            gather_tcl(&p, out);
        } else if p.extension().is_some_and(|e| e == "tcl") {
            out.push(p);
        }
    }
}

#[test]
fn cst_matches_oracle_over_tcl_corpus() {
    let mut files = Vec::new();
    for version in ["tcl8.4.20", "tcl8.5.19", "tcl8.6.16", "tcl9.0.3"] {
        gather_tcl(&repo_root().join("tmp").join(version), &mut files);
    }
    if files.is_empty() {
        eprintln!("[differential_segment] skipped: no tmp/tcl* corpus present");
        return;
    }

    let mut checked = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for path in &files {
        let Ok(src) = fs::read_to_string(path) else {
            continue;
        };
        checked += 1;
        let ctx = path.display().to_string();
        if let Err(msg) = check(&src, LexerConfig::default(), &ctx) {
            failures.push(msg);
            if failures.len() >= 20 {
                break;
            }
        }
    }
    assert!(checked > 0, "corpus present but no readable .tcl files");
    assert!(
        failures.is_empty(),
        "CST/oracle divergence over {checked} corpus files:\n{}",
        failures.join("\n")
    );
    eprintln!("[differential_segment] {checked} corpus files: CST == oracle");
}

/// Recovery is a post-process over `segment_commands_local`; confirm the
/// known-commands recovery path still works once the local segmenter is
/// CST-backed (the rebase only touches `segment_commands_local`).
#[test]
fn recovery_known_commands_smoke() {
    use tcl_compiler::segmenter::segment_commands_with_recovery;
    let known: HashSet<&str> = ["puts", "set"].into_iter().collect();
    // An unclosed brace spanning several lines, then a recoverable `puts`.
    let src = "proc f {\n  body line\n  more\n  another\nputs recovered\n";
    let segs = segment_commands_with_recovery(src, &known);
    assert!(
        segs.iter().any(|s| s.is_partial),
        "expected a partial command from the unclosed brace"
    );
}
