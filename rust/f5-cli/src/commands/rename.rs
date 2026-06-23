//! The `rename` verb — rename a BIG-IP object full-path and update every
//! reference.
//!
//! This is a thin shell over the `f5 query` engine: it constructs a
//! `rename(OLD, NEW)` DSL expression and runs it through
//! [`tcl_bigip_query::run_query`]. Routing both verbs through the same engine
//! keeps the rename logic — the token-bounded
//! [`rename_object`](tcl_bigip_query::rewrite::rename_object), the partition-
//! visibility refusals, and the diff renderer — in one place.
//!
//! The default is a unified-diff preview; `--write` prints the rewritten
//! config to stdout, `-o/--output` writes it to a file, and `--in-place`
//! overwrites the input. Every rename emits a `renamed 'old' -> 'new' (N
//! occurrence(s))` report on stderr.
//!
//! `--format tmsh` / `tmsh-delta` re-render the rewritten config via the BIG-IP
//! tmsh emit engine (`tcl_bigip::tmsh_emit`); the default unified-diff preview
//! is always SCF↔SCF. `--in-place` + `--format tmsh` is rejected (an in-place
//! write must preserve the SCF source format).

use std::io::Write as _;
use std::path::Path;

use tcl_bigip_io::read_path;
use tcl_bigip_query::{QueryOptions, run_query};

use tcl_cli_support::difflib;

use crate::cli::FormatArgs;

/// Wrap *value* as a DSL string literal, escaping `\` and `"`.
fn dsl_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// `f5 rename`.
pub fn run_rename(
    old: &str,
    new: &str,
    path: &str,
    output: Option<&Path>,
    in_place: bool,
    write: bool,
    format: &FormatArgs,
) -> anyhow::Result<u8> {
    if path == "-" && in_place {
        eprintln!("error: --in-place requires a path argument, not stdin");
        return Ok(2);
    }
    // `--in-place` must preserve the SCF source format; `--format tmsh`
    // produces a `tmsh modify` script and would otherwise silently overwrite
    // bigip.conf with a different file format.
    if in_place && format.format == "tmsh" {
        eprintln!(
            "error: --in-place is incompatible with --format tmsh \
             (in-place writes must preserve the SCF source format; \
             use --write, --output, or redirect for tmsh script output)"
        );
        return Ok(2);
    }
    if old.is_empty() {
        eprintln!("error: old name is empty");
        return Ok(2);
    }
    if new.is_empty() {
        eprintln!("error: new name is empty");
        return Ok(2);
    }
    if in_place && path != "-" && has_ucs_suffix(path) {
        eprintln!(
            "error: --in-place not supported for UCS archives ({path}); \
             extract first with `f5 extract` or use --write"
        );
        return Ok(2);
    }

    // Strict UTF-8 for in-place writes — a silent U+FFFD replacement followed
    // by an in-place rewrite would permanently overwrite the unreadable bytes.
    let opts = crate::cli::PassphraseArgs::default().to_options();
    let (uri, source) = match read_path(path, in_place, &opts) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let query = format!("rename({}, {})", dsl_string(old), dsl_string(new));
    let query_opts = QueryOptions::default();
    let result = match run_query(&query, &[(uri.clone(), source.clone())], &query_opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let applied = result.edits_per_file.iter().find(|(u, _)| *u == uri);
    let Some((_, applied)) = applied.filter(|(_, a)| a.new_source != a.original) else {
        eprintln!(
            "warning: no occurrences of {} found",
            tcl_bigip_query::eval::pyr_pub(old)
        );
        return Ok(1);
    };

    for rep in &applied.rename_reports {
        eprintln!(
            "renamed {} -> {} ({} occurrence(s))",
            tcl_bigip_query::eval::pyr_pub(&rep.old),
            tcl_bigip_query::eval::pyr_pub(&rep.new),
            rep.occurrences
        );
    }

    // `--format scf` returns the rewritten text verbatim; `tmsh` / `tmsh-delta`
    // re-render. No pre-edit original is threaded.
    let rewritten = crate::commands::emit::render_config(
        &applied.new_source,
        &format.format,
        "modify",
        format.transaction,
        "",
    );

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if in_place {
        std::fs::write(Path::new(path), &rewritten)?;
    } else if let Some(output) = output {
        std::fs::write(output, &rewritten)?;
    } else if write {
        write!(out, "{rewritten}")?;
    } else {
        // Default: a unified diff so the user sees what would change. The diff
        // is always SCF↔SCF; the LHS is the source file as-is.
        let from = path.to_owned();
        let to = format!("{path} (renamed)");
        let a = difflib::splitlines_keepends(&source);
        let b = difflib::splitlines_keepends(&applied.new_source);
        // The shared `difflib` yields the diff as a line list (each line keeps
        // its terminator); the default unified-diff context is 3 — join the
        // lines to reproduce the previous concatenated string.
        let diff = difflib::unified_diff(&a, &b, &from, &to, 3).join("");
        write!(out, "{diff}")?;
    }
    Ok(0)
}

/// Whether *path*'s extension is `.ucs` (case-insensitive).
fn has_ucs_suffix(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ucs"))
}
