//! Differential codegen test harness (C20).
//!
//! Feeds a corpus of Tcl scripts through both the Python emitter
//! (subprocess invocation of `core.compiler.codegen.codegen_module`
//! from the Python repo root) and the Rust pipeline
//! (`lower_to_ir` → `build_cfg` → `codegen_module`), then compares
//! the resulting disassembly.
//!
//! Three tiers of equivalence are reported:
//!
//! - **Exact** — disassembly is byte-for-byte identical, including
//!   auto-generated label names (`cmd_end_N`, `if_end_N`, …).
//! - **Semantic** — disassembly matches after stripping label-comment
//!   lines (`  # label:`) and trailing whitespace. Label names are
//!   internal comments only; jump targets are already compared via
//!   their resolved PC values. A semantic match means the emitted
//!   bytecode stream, literal table, LVT, and jump targets agree.
//! - **Divergent** — a real codegen gap: instruction count, literal
//!   values, LVT entries, or jump PCs differ.
//!
//! The corpus under `tests/fixtures/codegen/matching/` is asserted
//! to at least semantic-match. The corpus under
//! `tests/fixtures/codegen/divergent/` is run purely as a progress
//! tracker for C18/C19 — the test reports the current divergence
//! count but never fails. As each remaining gap closes, divergent
//! fixtures are promoted to the matching corpus in the same commit
//! that lands the fix.
//!
//! If the Python oracle can't be invoked (no `python3`, import
//! failure, etc.) the entire harness logs a skip message and returns
//! `Ok`. This keeps the test green on stripped-down sandboxes while
//! still running under `make prep-pr`.

use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use tcl_compiler::cfg_builder::build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::codegen::format::format_module_asm;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

// ---------------------------------------------------------------------------
// Pipeline wrappers
// ---------------------------------------------------------------------------

/// Compile `source` through the Rust pipeline and return the rendered
/// disassembly string.
fn rust_disasm(source: &str) -> String {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(source, &registry);
    let cfg = build_cfg(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    format_module_asm(&asm)
}

/// Repo root — two directories above `CARGO_MANIFEST_DIR`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Result of trying to invoke the Python oracle.
#[derive(Debug)]
enum OracleResult {
    /// Python ran successfully; contains the disassembly string.
    Ok(String),
    /// Python was available but rejected this input (syntax error,
    /// lowering failure, etc.). Carries the captured stderr.
    Error(String),
    /// Python was not available (couldn't spawn `python3`, import
    /// failed, etc.). Tests should degrade to a graceful skip.
    Unavailable(String),
}

/// The inline Python driver that reads a Tcl script from stdin and
/// writes the disassembly to stdout.
const PY_DRIVER: &str = r"
import sys
try:
    # Register bytecoded codegen hooks so specialised dispatches
    # (lassign, llength, array names, …) kick in for the oracle.
    from core.compiler.codegen.bytecoded import register_all
    register_all()
    from core.compiler.codegen import codegen_module
    from core.compiler.codegen.format import format_module_asm
    from core.compiler.cfg import build_cfg
    from core.compiler.lowering import lower_to_ir
except Exception as e:
    sys.stderr.write('ORACLE_IMPORT_FAIL: ' + repr(e) + '\n')
    sys.exit(2)
src = sys.stdin.read()
m = lower_to_ir(src)
c = build_cfg(m)
asm = codegen_module(c, m)
sys.stdout.write(format_module_asm(asm))
";

/// Probe whether the Python oracle is usable on this host. Cached so
/// the subprocess spawn happens at most once per test run.
fn oracle_status() -> &'static Result<(), String> {
    static STATUS: OnceLock<Result<(), String>> = OnceLock::new();
    STATUS.get_or_init(|| {
        let output = Command::new("python3")
            .arg("-c")
            .arg(
                "import sys
try:
    import core.compiler.codegen  # noqa: F401
    sys.exit(0)
except Exception as e:
    sys.stderr.write(repr(e))
    sys.exit(2)
",
            )
            .current_dir(repo_root())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .map_err(|e| format!("spawn python3: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "python oracle unavailable (exit {:?}): {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim(),
            ))
        }
    })
}

/// Invoke the Python oracle on `source`.
fn python_disasm(source: &str) -> OracleResult {
    if let Err(msg) = oracle_status() {
        return OracleResult::Unavailable(msg.clone());
    }
    let mut child = match Command::new("python3")
        .arg("-c")
        .arg(PY_DRIVER)
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return OracleResult::Unavailable(format!("spawn: {e}")),
    };
    if let Err(e) = child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(source.as_bytes())
    {
        return OracleResult::Unavailable(format!("stdin write: {e}"));
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return OracleResult::Unavailable(format!("wait: {e}")),
    };
    if out.status.success() {
        OracleResult::Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        // The Python driver exits with code 2 and an `ORACLE_IMPORT_FAIL:`
        // stderr prefix when it can't import the oracle — mirror that as
        // `Unavailable` so environments without the Python side skip
        // gracefully instead of reporting divergence.
        if out.status.code() == Some(2) && stderr.starts_with("ORACLE_IMPORT_FAIL:") {
            OracleResult::Unavailable(stderr)
        } else {
            OracleResult::Error(stderr)
        }
    }
}

// ---------------------------------------------------------------------------
// Disassembly normalisation
// ---------------------------------------------------------------------------

/// Strip label-comment lines (`  # label:`) so a semantic compare
/// ignores Rust/Python disagreements over internal label names and
/// the ordering of coincident labels at the same offset.
fn strip_label_comments(disasm: &str) -> String {
    disasm
        .lines()
        .filter(|l| !l.trim_start().starts_with("# "))
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Equivalence tier for a disassembly pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
    /// Byte-for-byte match (including label names).
    Exact,
    /// Matches after label-comment stripping; bytecode identical.
    Semantic,
    /// Real divergence.
    Divergent,
}

fn classify(rust: &str, py: &str) -> Tier {
    if rust.trim_end() == py.trim_end() {
        Tier::Exact
    } else if strip_label_comments(rust) == strip_label_comments(py) {
        Tier::Semantic
    } else {
        Tier::Divergent
    }
}

/// Produce a unified-ish diff of the first N differing lines for
/// human triage.
fn brief_diff(rust: &str, py: &str, context: usize) -> String {
    let r: Vec<&str> = rust.lines().collect();
    let p: Vec<&str> = py.lines().collect();
    let n = r.len().min(p.len());
    let mut first = None;
    for i in 0..n {
        if r[i] != p[i] {
            first = Some(i);
            break;
        }
    }
    let first = first.unwrap_or(n);
    let start = first.saturating_sub(context);
    let end_r = (first + context + 1).min(r.len());
    let end_p = (first + context + 1).min(p.len());
    let mut out = String::new();
    writeln!(out, "    first divergence at line {first}").unwrap();
    out.push_str("    --- rust ---\n");
    for line in &r[start..end_r] {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("    --- python ---\n");
    for line in &p[start..end_p] {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    if r.len() != p.len() {
        writeln!(
            out,
            "    (rust has {} lines, python has {} lines)",
            r.len(),
            p.len()
        )
        .unwrap();
    }
    out
}

// ---------------------------------------------------------------------------
// Fixture discovery
// ---------------------------------------------------------------------------

fn fixture_dir(subdir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("codegen")
        .join(subdir)
}

fn gather_fixtures(dir: &Path) -> Vec<(String, String)> {
    let mut entries: Vec<(String, String)> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "tcl"))
        .map(|e| {
            let name = e
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("<unknown>")
                .to_string();
            let source = fs::read_to_string(e.path())
                .unwrap_or_else(|err| panic!("read {}: {err}", e.path().display()));
            (name, source)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// The matching corpus must at least semantic-match the Python oracle.
#[test]
fn matching_corpus_is_semantically_equivalent() {
    if let Err(msg) = oracle_status() {
        eprintln!("[differential_codegen] skipped: {msg}");
        return;
    }

    let dir = fixture_dir("matching");
    let cases = gather_fixtures(&dir);
    assert!(
        !cases.is_empty(),
        "no fixtures found in {dir:?}; corpus is empty"
    );

    let mut failures: Vec<(String, String)> = Vec::new();
    let mut exact = 0usize;
    let mut semantic = 0usize;
    for (name, src) in &cases {
        let rust = rust_disasm(src);
        match python_disasm(src) {
            OracleResult::Ok(py) => match classify(&rust, &py) {
                Tier::Exact => exact += 1,
                Tier::Semantic => semantic += 1,
                Tier::Divergent => {
                    failures.push((name.clone(), brief_diff(&rust, &py, 4)));
                }
            },
            OracleResult::Error(err) => {
                failures.push((name.clone(), format!("python oracle error: {err}")));
            }
            OracleResult::Unavailable(msg) => {
                eprintln!("[differential_codegen] oracle went away mid-run: {msg}");
                return;
            }
        }
    }

    eprintln!(
        "[differential_codegen] matching corpus: {} exact, {} semantic, {} divergent / {} total",
        exact,
        semantic,
        failures.len(),
        cases.len(),
    );

    if !failures.is_empty() {
        let mut msg = format!(
            "{} fixture(s) in matching/ diverged from the Python oracle:\n",
            failures.len()
        );
        for (name, diff) in &failures {
            write!(msg, "\n[{name}]\n{diff}").unwrap();
        }
        panic!("{msg}");
    }
}

/// The divergent corpus tracks known codegen gaps (driven by C18/C19).
/// This test never fails — it only reports the current divergence
/// count so progress is visible in test output and CI logs. When a
/// gap closes, its fixture graduates to `matching/` in the same
/// commit that lands the fix.
#[test]
fn divergent_corpus_reports_progress() {
    if let Err(msg) = oracle_status() {
        eprintln!("[differential_codegen] skipped: {msg}");
        return;
    }

    let dir = fixture_dir("divergent");
    let cases = gather_fixtures(&dir);
    if cases.is_empty() {
        eprintln!("[differential_codegen] divergent corpus empty — all gaps closed?");
        return;
    }

    let mut still_divergent: Vec<String> = Vec::new();
    let mut closed: Vec<String> = Vec::new();
    for (name, src) in &cases {
        let rust = rust_disasm(src);
        match python_disasm(src) {
            OracleResult::Ok(py) => match classify(&rust, &py) {
                Tier::Exact | Tier::Semantic => closed.push(name.clone()),
                Tier::Divergent => still_divergent.push(name.clone()),
            },
            OracleResult::Error(_) | OracleResult::Unavailable(_) => {
                still_divergent.push(format!("{name} (oracle failed)"));
            }
        }
    }

    eprintln!(
        "[differential_codegen] divergent corpus: {} still divergent, {} ready to promote / {} total",
        still_divergent.len(),
        closed.len(),
        cases.len(),
    );
    if !closed.is_empty() {
        eprintln!("[differential_codegen] these fixtures now match — promote them to matching/:");
        for name in &closed {
            eprintln!("    {name}");
        }
    }
    if !still_divergent.is_empty() {
        eprintln!("[differential_codegen] still divergent:");
        for name in &still_divergent {
            eprintln!("    {name}");
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests for the normalisation helpers
// ---------------------------------------------------------------------------

#[test]
fn classify_exact_match() {
    let s = "ByteCode ::top\n  Instructions:\n  # entry_1:\n    (0) done";
    assert_eq!(classify(s, s), Tier::Exact);
}

#[test]
fn classify_semantic_match_ignores_label_names() {
    let a = "  Instructions:\n  # cmd_end_0:\n    (0) done";
    let b = "  Instructions:\n  # cmd_end_17:\n    (0) done";
    assert_eq!(classify(a, b), Tier::Semantic);
}

#[test]
fn classify_semantic_match_ignores_label_ordering() {
    let a = "  Instructions:\n  # entry_1:\n  # exit_2:\n    (0) done";
    let b = "  Instructions:\n  # exit_2:\n  # entry_1:\n    (0) done";
    assert_eq!(classify(a, b), Tier::Semantic);
}

#[test]
fn classify_divergent_on_instruction_mismatch() {
    let a = "  Instructions:\n    (0) push1 0\n    (2) done";
    let b = "  Instructions:\n    (0) push1 0\n    (2) pop\n    (3) done";
    assert_eq!(classify(a, b), Tier::Divergent);
}

#[test]
fn classify_divergent_on_literal_mismatch() {
    let a = "  Literals:\n    0: \"a\"\n  Instructions:\n    (0) done";
    let b = "  Literals:\n    0: \"b\"\n  Instructions:\n    (0) done";
    assert_eq!(classify(a, b), Tier::Divergent);
}

#[test]
fn strip_label_comments_removes_only_label_lines() {
    let input = "  Instructions:\n  # entry_1:\n    (0) done\n  # after:\n";
    let stripped = strip_label_comments(input);
    assert_eq!(stripped, "  Instructions:\n    (0) done");
}

#[test]
fn rust_pipeline_round_trips_simple_assign() {
    // Smoke: the Rust side runs end-to-end without panicking.
    let disasm = rust_disasm("set x 1");
    assert!(disasm.contains("storeStk"));
    assert!(disasm.contains("\"x\""));
    assert!(disasm.contains("\"1\""));
}
