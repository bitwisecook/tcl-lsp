//! Port of `tests/test_detect_dialect.py` — content-based dialect detection
//! (`compiler.registry.dialect.detect_dialect_from_source`).
//!
//! Detection priority: `# tcl-dialect:` directive (first 5 lines) > shebang
//! (first line) > `package require Tcl <x.y>` (first 30 lines). All assertions
//! are structural (string → dialect-name), so no tclsh proof applies.

use tcl_registry::detect_dialect_from_source as detect;

#[test]
fn shebang_tclsh84() {
    assert_eq!(detect("#!/usr/bin/tclsh8.4\nset x 1\n"), Some("tcl8.4"));
}

#[test]
fn shebang_tclsh85() {
    assert_eq!(detect("#!/usr/bin/tclsh8.5\nset x 1\n"), Some("tcl8.5"));
}

#[test]
fn shebang_env_tclsh86() {
    assert_eq!(detect("#!/usr/bin/env tclsh8.6\nset x 1\n"), Some("tcl8.6"));
}

#[test]
fn shebang_tclsh90() {
    assert_eq!(detect("#!/usr/bin/env tclsh9.0\nset x 1\n"), Some("tcl9.0"));
}

#[test]
fn shebang_expect() {
    assert_eq!(detect("#!/usr/bin/expect\nspawn ssh\n"), Some("expect"));
}

#[test]
fn shebang_env_expect() {
    assert_eq!(detect("#!/usr/bin/env expect\nspawn ssh\n"), Some("expect"));
}

#[test]
fn directive_tcl84() {
    assert_eq!(detect("# tcl-dialect: tcl8.4\nset x 1\n"), Some("tcl8.4"));
}

#[test]
fn directive_irules() {
    assert_eq!(
        detect("# tcl-dialect: f5-irules\nwhen HTTP_REQUEST {\n"),
        Some("f5-irules")
    );
}

#[test]
fn directive_on_later_line() {
    let src = "#!/usr/bin/tclsh\n# My script\n# tcl-dialect: tcl8.5\nset x 1\n";
    assert_eq!(detect(src), Some("tcl8.5"));
}

#[test]
fn directive_takes_priority_over_shebang() {
    let src = "#!/usr/bin/tclsh8.6\n# tcl-dialect: tcl8.4\nset x 1\n";
    assert_eq!(detect(src), Some("tcl8.4"));
}

#[test]
fn directive_case_insensitive() {
    assert_eq!(detect("# TCL-DIALECT: tcl9.0\nset x 1\n"), Some("tcl9.0"));
}

#[test]
fn directive_crlf_line_endings() {
    assert_eq!(detect("# tcl-dialect: tcl8.4\r\nset x 1\r\n"), Some("tcl8.4"));
}

#[test]
fn directive_unknown_dialect_ignored() {
    assert_eq!(detect("# tcl-dialect: unknown\nset x 1\n"), None);
}

#[test]
fn no_hint_returns_none() {
    assert_eq!(detect("set x 1\nputs $x\n"), None);
}

#[test]
fn shebang_plain_tclsh_no_version() {
    assert_eq!(detect("#!/usr/bin/tclsh\nset x 1\n"), None);
}

#[test]
fn directive_beyond_scan_window() {
    let mut s = "# line\n".repeat(10);
    s.push_str("# tcl-dialect: tcl8.4\nset x 1\n");
    assert_eq!(detect(&s), None);
}

// --- extra coverage beyond the pytest: package-require path + shebang edges ---

#[test]
fn package_require_tcl_version() {
    assert_eq!(detect("# header\npackage require Tcl 8.6\n"), Some("tcl8.6"));
    assert_eq!(detect("package require -exact Tcl 9.0\n"), Some("tcl9.0"));
}

#[test]
fn package_require_non_tcl_ignored() {
    // Lowercase `tcl` is not the Tcl core package (regex is case-sensitive).
    assert_eq!(detect("package require tcl 8.6\n"), None);
    assert_eq!(detect("package require Tk 8.6\n"), None);
}
