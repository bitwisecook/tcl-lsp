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

//! Dialect membership sets.

use bitflags::bitflags;

bitflags! {
    /// Compact set of Tcl dialects a command/subcommand is available in.
    ///
    /// `None` on a `CommandSpec` means "available in all dialects".
    /// A specific `DialectSet` restricts availability.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DialectSet: u16 {
        /// Tcl 8.4
        const TCL84     = 1 << 0;
        /// Tcl 8.5
        const TCL85     = 1 << 1;
        /// Tcl 8.6
        const TCL86     = 1 << 2;
        /// Tcl 9.0
        const TCL90     = 1 << 3;
        /// F5 iRules
        const IRULES    = 1 << 4;
        /// F5 iApps
        const IAPPS     = 1 << 5;
        /// Tk
        const TK        = 1 << 6;
        /// Expect
        const EXPECT    = 1 << 7;
        /// Synopsys EDA
        const SYNOPSYS  = 1 << 8;
        /// Cadence EDA
        const CADENCE   = 1 << 9;
        /// Xilinx/AMD EDA
        const XILINX    = 1 << 10;
        /// Intel Quartus EDA
        const QUARTUS   = 1 << 11;
        /// Mentor/Siemens EDA
        const MENTOR    = 1 << 12;
        /// BPF-Tcl (the eBPF framework dialect)
        const BPF       = 1 << 13;
        /// Tcl 9.1
        const TCL91     = 1 << 14;

        /// All standard Tcl versions.
        const ALL_TCL = Self::TCL84.bits() | Self::TCL85.bits()
                      | Self::TCL86.bits() | Self::TCL90.bits() | Self::TCL91.bits();

        /// Tcl 8.5 and later.
        const TCL85_PLUS = Self::TCL85.bits() | Self::TCL86.bits()
                         | Self::TCL90.bits() | Self::TCL91.bits();

        /// Tcl 8.6 and later.
        const TCL86_PLUS = Self::TCL86.bits() | Self::TCL90.bits() | Self::TCL91.bits();

        /// The Tcl 8 series only (8.4, 8.5, 8.6), excluding 9.0+.  For
        /// commands and options dropped or renamed at the 9.0 boundary — e.g.
        /// `tcltest::bytestring` (not defined under Tcl 9.0+) and the
        /// `teststaticpkg` / `testsaveresult` test-harness commands (removed
        /// in 9.0).
        const TCL8X = Self::TCL84.bits() | Self::TCL85.bits() | Self::TCL86.bits();

        /// Tcl 9.0 and later.  A command/option gated to "9.0" persists in
        /// 9.1 (a `.1` release is additive): a `{tcl9.0}` membership is
        /// inherited under `tcl9.1`.
        const TCL90_PLUS = Self::TCL90.bits() | Self::TCL91.bits();

        /// Dialects in which the Tk widget/window commands (`button`,
        /// `pack`, `wm`, `winfo`, the `ttk::` forms, …) are available:
        /// standard Tcl (a `wish`/`package require Tk` interpreter) plus the
        /// pure-`tk` dialect.
        ///
        /// Tk is *not* part of the restricted embedded dialects (F5 iRules /
        /// iApps) or the vendor EDA shells, so its commands must never be
        /// offered or accepted there.  Membership in `ALL_TCL` (not `TK`
        /// alone) is deliberate: a plain `.tcl` file that does
        /// `package require Tk` keeps its `tcl8.6`/`tcl9.0` dialect, so the
        /// commands have to resolve under a Tcl version — the *loaded*
        /// gating (only present once `package require Tk` ran) is layered on
        /// top by the LSP, not by this dialect set.
        const TK_AND_TCL = Self::ALL_TCL.bits() | Self::TK.bits();

        /// Every modelled dialect *except* F5 iRules and Tk.
        ///
        /// The math-operator commands
        /// (`+`, `eq`, `tcl::mathop::*`, …) are valid in every command
        /// dialect *except* `f5-irules` (in iRules, operators live
        /// inside `expr`, never as standalone command heads) and
        /// `tk`. This set captures that membership so the iRules
        /// event/command cross-product (`commands_for_event`) excludes them.
        const NON_IRULES_OPERATORS = Self::ALL_TCL.bits()
            | Self::IAPPS.bits() | Self::EXPECT.bits()
            | Self::SYNOPSYS.bits() | Self::CADENCE.bits()
            | Self::XILINX.bits() | Self::QUARTUS.bits()
            | Self::MENTOR.bits();
    }
}

/// Canonical dialect profile names, in sorted order.
///
/// Kept pre-sorted so [`available_dialects`] returns them in sorted
/// order. This
/// is the single source of truth for the explorer's dialect dropdown and
/// the CLI's `--dialect` choices. Note it is a *superset* of the names
/// [`DialectSet::parse`] resolves to a flag — config-only dialects
/// (`f5-bigip`, `f5-tmsh`) appear here but collapse to plain Tcl when
/// parsed.
pub const KNOWN_DIALECTS: &[&str] = &[
    "bpf",
    "cadence-eda-tcl",
    "expect",
    "f5-bigip",
    "f5-iapps",
    "f5-irules",
    "f5-tmsh",
    "intel-quartus-eda-tcl",
    "mentor-eda-tcl",
    "synopsys-eda-tcl",
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "tcl9.1",
    "xilinx-eda-tcl",
];

/// Return the canonical dialect profile names in sorted order.
#[must_use]
pub fn available_dialects() -> &'static [&'static str] {
    KNOWN_DIALECTS
}

/// Number of leading lines scanned for a `# tcl-dialect:` directive.
pub const DIALECT_DIRECTIVE_SCAN_LINES: usize = 5;

/// Map a bare Tcl `<major.minor>` version to its canonical dialect name.
fn tcl_version_dialect(ver: &str) -> Option<&'static str> {
    Some(match ver {
        "8.4" => "tcl8.4",
        "8.5" => "tcl8.5",
        "8.6" => "tcl8.6",
        "9.0" => "tcl9.0",
        "9.1" => "tcl9.1",
        _ => return None,
    })
}

/// `true` when the ASCII byte is a `\w` word character.
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Whether `haystack` contains `word` delimited by `\b` boundaries (ASCII).
fn has_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut i = 0;
    while let Some(off) = haystack[i..].find(word) {
        let start = i + off;
        let end = start + word.len();
        let before = start == 0 || !is_word_byte(bytes[start - 1]);
        let after = end == bytes.len() || !is_word_byte(bytes[end]);
        if before && after {
            return true;
        }
        i = start + 1;
    }
    false
}

/// Extract `<x.y>` from a `…\btclsh<x.y>\b…` shebang (input already lowercased).
fn shebang_tclsh_version(lower: &str) -> Option<String> {
    let bytes = lower.as_bytes();
    let mut i = 0;
    while let Some(off) = lower[i..].find("tclsh") {
        let start = i + off;
        let before = start == 0 || !is_word_byte(bytes[start - 1]);
        let mut j = start + "tclsh".len();
        let d1 = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if before && j > d1 && j < bytes.len() && bytes[j] == b'.' {
            j += 1;
            let d2 = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let after = j == bytes.len() || !is_word_byte(bytes[j]);
            if j > d2 && after {
                return Some(lower[d1..j].to_string());
            }
        }
        i = start + 1;
    }
    None
}

/// Extract a leading `<major>.<minor>` version from `s` — one or more digits, a
/// `.`, and one or more digits. Trailing content (a patchlevel `.z`, a `-`
/// range suffix, whitespace) is ignored, so `"9.0"`, `"9.0.3"`, and `"8.5-9.0"`
/// all yield the leading `major.minor`. Returns `None` when `s` doesn't start
/// with a `major.minor` pair.
fn extract_major_minor(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut j = 0;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == 0 || j >= bytes.len() || bytes[j] != b'.' {
        return None;
    }
    j += 1;
    let d2 = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == d2 {
        return None;
    }
    Some(s[..j].to_string())
}

/// Maximum nesting depth `scan_tokens_for_tcl_version` descends into command
/// substitutions / braced words. The version guard is always near the top of a
/// file at shallow nesting (`if { [package vsatisfies …] }` is depth 2), so a
/// small bound keeps the scan cheap while covering every real idiom.
const VERSION_SCAN_DEPTH: u8 = 8;

/// One word of a tokenised command: its lexer kind and delimiter-stripped text.
struct ScanWord {
    kind: tcl_lexer::TokenType,
    text: String,
}

/// Tokenise `script` and search its *command structure* for a Tcl version
/// requirement, descending into command substitutions (`[…]`) and braced words
/// (`{…}`) so a guard nested inside `if { … }` is found too. Recognises both:
///
/// - `package require ?-exact? Tcl <x.y>` — the direct minimum-version require;
/// - `package vsatisfies [package require Tcl] <x.y>` — the idiomatic runtime
///   guard (Tcl 9's own `tclshrc` uses exactly this).
///
/// Working from tokens rather than raw text means a commented-out or
/// string-literal `package require` never matches (the lexer classes those as
/// `Comment` / a single quoted word), and bracket / brace nesting is handled
/// structurally instead of by fragile delimiter counting. Returns the first
/// `<major.minor>` found in reading order, or `None`.
fn scan_tokens_for_tcl_version(script: &str, depth: u8) -> Option<String> {
    if depth >= VERSION_SCAN_DEPTH {
        return None;
    }
    let tokens = tcl_lexer::Lexer::new(script).tokenise_all().ok()?;
    let sm = tcl_lexer::SourceMap::new(script);
    let mut cmd: Vec<ScanWord> = Vec::new();
    for tok in tokens {
        match tok.kind {
            // Command boundary: match the accumulated words, then reset.
            tcl_lexer::TokenType::Eol | tcl_lexer::TokenType::Eof => {
                if let Some(v) = match_version_command(&cmd, depth) {
                    return Some(v);
                }
                cmd.clear();
            }
            // Non-word tokens carry no command word.
            tcl_lexer::TokenType::Sep
            | tcl_lexer::TokenType::Comment
            | tcl_lexer::TokenType::Expand => {}
            _ => cmd.push(ScanWord {
                kind: tok.kind,
                text: sm.token_text(tok).to_string(),
            }),
        }
    }
    match_version_command(&cmd, depth)
}

/// Whether a command evaluates its *braced / quoted* word arguments as script or
/// expression bodies — the gate for whether the version scan may recurse into
/// them. Command substitutions (`[…]`) are always genuine scripts and are
/// recursed into regardless of the command; this gate applies only to braced /
/// quoted words, so inert data such as `set msg {package require Tcl 8.4}` is
/// never mis-tokenised as a script and used to select a bogus dialect. The name
/// is matched with any leading `::` stripped so fully-qualified builtins
/// (`::if`, `::namespace`) are recognised too.
fn is_script_body_command(name: &str) -> bool {
    matches!(
        name.trim_start_matches("::"),
        "if" | "while"
            | "for"
            | "foreach"
            | "catch"
            | "try"
            | "eval"
            | "namespace"
            | "uplevel"
            | "apply"
            | "expr"
            | "subst"
            | "proc"
            | "time"
            | "coroutine"
    )
}

/// Match one tokenised command against the two Tcl-version idioms, first
/// descending into any command-substitution / script-body argument so a guard
/// nested inside this command (e.g. `if { [package vsatisfies …] }`) is found.
fn match_version_command(cmd: &[ScanWord], depth: u8) -> Option<String> {
    // Recurse into command substitutions unconditionally (they are always real
    // scripts), but into braced / quoted words only when this command evaluates
    // them as script / expression bodies. The gate is applied at *every* nesting
    // level, so even `if {$c} {set msg {package require Tcl 8.4}}` leaves the
    // inner `set` data untouched. (Reading order.)
    let descend_braced = cmd.first().is_some_and(|w| is_script_body_command(&w.text));
    for w in cmd {
        let recurse = match w.kind {
            tcl_lexer::TokenType::Cmd => true,
            tcl_lexer::TokenType::Str => descend_braced,
            _ => false,
        };
        if recurse && let Some(v) = scan_tokens_for_tcl_version(&w.text, depth + 1) {
            return Some(v);
        }
    }
    // This command's own shape must start with `package`.
    if cmd.first().map(|w| w.text.as_str()) != Some("package") {
        return None;
    }
    match cmd.get(1).map(|w| w.text.as_str()) {
        // `package require ?-exact? Tcl <x.y>`.
        Some("require") => {
            let mut i = 2;
            if cmd.get(i).map(|w| w.text.as_str()) == Some("-exact") {
                i += 1;
            }
            // `Tcl` is matched case-sensitively — `tcl` / `Tk` are other packages.
            if cmd.get(i).map(|w| w.text.as_str()) != Some("Tcl") {
                return None;
            }
            cmd.get(i + 1).and_then(|w| extract_major_minor(&w.text))
        }
        // `package vsatisfies [package require Tcl] <x.y>` — the subject must be
        // a `[package require Tcl]` substitution so a `vsatisfies` over some
        // other package can't select a Tcl dialect.
        Some("vsatisfies") => {
            let subject = cmd.get(2)?;
            if subject.kind != tcl_lexer::TokenType::Cmd || !is_package_require_tcl(&subject.text) {
                return None;
            }
            cmd.get(3).and_then(|w| extract_major_minor(&w.text))
        }
        _ => None,
    }
}

/// Whether `s` tokenises to exactly the command `package require Tcl` — the
/// subject of a `vsatisfies` Tcl-version guard.
fn is_package_require_tcl(s: &str) -> bool {
    let Ok(tokens) = tcl_lexer::Lexer::new(s).tokenise_all() else {
        return false;
    };
    let sm = tcl_lexer::SourceMap::new(s);
    let words: Vec<&str> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.kind,
                tcl_lexer::TokenType::Esc
                    | tcl_lexer::TokenType::Str
                    | tcl_lexer::TokenType::Var
                    | tcl_lexer::TokenType::Cmd
            )
        })
        .map(|t| sm.token_text(*t))
        .collect();
    words.as_slice() == ["package", "require", "Tcl"]
}

/// Return the dialect named by a `# tcl-dialect: <dialect>` comment directive in
/// the first [`DIALECT_DIRECTIVE_SCAN_LINES`] lines, or `None`. The directive
/// keyword is matched case-insensitively; the named dialect must be one of
/// [`KNOWN_DIALECTS`] (so an unknown name yields `None`).
#[must_use]
pub fn detect_dialect_directive(source: &str) -> Option<&'static str> {
    const KEY: &str = "tcl-dialect:";
    for line in source.lines().take(DIALECT_DIRECTIVE_SCAN_LINES) {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let rest = rest.trim_start();
        if rest.len() < KEY.len() || !rest[..KEY.len()].eq_ignore_ascii_case(KEY) {
            continue;
        }
        let candidate = rest[KEY.len()..]
            .split_whitespace()
            .next()
            .unwrap_or_default();
        return KNOWN_DIALECTS.iter().copied().find(|&d| d == candidate);
    }
    None
}

#[cfg(test)]
mod detect_tests {
    use super::detect_dialect;

    const DEF: &str = "tcl9.0";

    #[test]
    fn directive_wins() {
        assert_eq!(
            detect_dialect("# tcl-dialect: tcl8.5\nputs hi\n", None, DEF),
            "tcl8.5"
        );
    }

    #[test]
    fn extension_is_the_fallback_for_generic_content() {
        // With no content signal, the extension decides.
        assert_eq!(
            detect_dialect("puts hi\n", Some("x.irule"), DEF),
            "f5-irules"
        );
        assert_eq!(
            detect_dialect("spawn ssh host\n", Some("y.exp"), DEF),
            "expect"
        );
        assert_eq!(
            detect_dialect("read_xdc c.xdc\n", Some("c.xdc"), DEF),
            "xilinx-eda-tcl"
        );
    }

    #[test]
    fn content_signals_beat_extension() {
        // A `package require Tcl` guard outranks the filename extension …
        assert_eq!(
            detect_dialect("package require Tcl 9.0\n", Some("x.exp"), DEF),
            "tcl9.0"
        );
        // … and so does a `when`-clause content signature.
        assert_eq!(
            detect_dialect("when HTTP_REQUEST {\n  pool web\n}\n", Some("x.tcl"), DEF),
            "f5-irules"
        );
    }

    #[test]
    fn late_signal_in_large_script_is_caught() {
        // A version guard past the head-scan window is still found by the
        // full-source fallback pass.
        let mut s = "set x 1\n".repeat(2000); // ~16 KB of filler > DETECT_SCAN_BYTES
        s.push_str("if {[package vsatisfies [package require Tcl] 9.0]} { x }\n");
        assert_eq!(detect_dialect(&s, None, DEF), "tcl9.0");
    }

    #[test]
    fn shebang_detected() {
        assert_eq!(
            detect_dialect("#!/usr/bin/expect -f\nspawn ssh\n", None, DEF),
            "expect"
        );
        assert_eq!(
            detect_dialect("#!/usr/bin/tclsh8.6\nputs hi\n", None, DEF),
            "tcl8.6"
        );
    }

    #[test]
    fn irules_when_detected() {
        assert_eq!(
            detect_dialect("when HTTP_REQUEST {\n  pool web\n}\n", None, DEF),
            "f5-irules"
        );
    }

    #[test]
    fn eda_and_f5_content_signatures() {
        assert_eq!(
            detect_dialect("synth_design -top foo\n", None, DEF),
            "xilinx-eda-tcl"
        );
        assert_eq!(
            detect_dialect("compile_ultra -gate_clock\n", None, DEF),
            "synopsys-eda-tcl"
        );
        assert_eq!(
            detect_dialect("set_db init_design\ninit_design\n", None, DEF),
            "cadence-eda-tcl"
        );
        assert_eq!(
            detect_dialect("tmsh::create ltm pool p\n", None, DEF),
            "f5-tmsh"
        );
    }

    #[test]
    fn expect_content_without_shebang() {
        assert_eq!(
            detect_dialect("spawn ssh host\nexpect_before timeout\n", None, DEF),
            "expect"
        );
    }

    #[test]
    fn plain_tcl_falls_back_to_default() {
        assert_eq!(detect_dialect("set x 1\nputs $x\n", None, DEF), "tcl9.0");
        assert_eq!(
            detect_dialect("package require Tcl 8.6\n", None, DEF),
            "tcl8.6"
        );
    }
}

/// Maximum bytes of a file inspected by [`detect_dialect`]. Detection reads
/// only the head of a document — enough to catch a directive / shebang /
/// signature line without scanning a large file.
pub const DETECT_SCAN_BYTES: usize = 8192;

/// The dialect implied by a filename extension, or `None` when the extension
/// is generic (`.tcl`) or unknown — in which case content heuristics decide.
#[must_use]
pub fn dialect_from_extension(filename: &str) -> Option<&'static str> {
    let ext = filename.rsplit('.').next().map(str::to_ascii_lowercase)?;
    Some(match ext.as_str() {
        "irul" | "irule" | "irules" => "f5-irules",
        "iapp" => "f5-iapps",
        "tmsh" => "f5-tmsh",
        "exp" | "expect" => "expect",
        "xdc" => "xilinx-eda-tcl",
        "sdc" => "synopsys-eda-tcl",
        // Generic `.tcl`, HDL (`.sv`/`.svh`), or unknown → let content decide.
        _ => return None,
    })
}

/// Truncate `source` to at most [`DETECT_SCAN_BYTES`] on a UTF-8 char boundary.
fn scan_head(source: &str) -> &str {
    if source.len() <= DETECT_SCAN_BYTES {
        return source;
    }
    let mut end = DETECT_SCAN_BYTES;
    while end > 0 && !source.is_char_boundary(end) {
        end -= 1;
    }
    &source[..end]
}

/// Whether `head` contains an iRules `when EVENT {` handler (the strongest
/// iRules signal). Mirrors `^\s*when\s+[A-Z][A-Z0-9_]{2,}\s*\{`.
fn has_irules_when(head: &str) -> bool {
    for line in head.lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("when") else {
            continue;
        };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let name = rest.trim_start();
        let mut chars = name.chars();
        // `[A-Z]` then `[A-Z0-9_]{2,}`.
        if !matches!(chars.next(), Some(c) if c.is_ascii_uppercase()) {
            continue;
        }
        let ident: String = name
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        if ident.len() >= 3 && name[ident.len()..].trim_start().starts_with('{') {
            return true;
        }
    }
    false
}

/// Content signatures for the non-Tcl-core dialects, checked in priority order.
/// Each entry is `(dialect, &[marker words])`; a marker matches when it appears
/// as a whole word anywhere in the scanned head. Ordered most-specific first so
/// an EDA-tool script never falls through to a weaker signal.
const CONTENT_SIGNATURES: &[(&str, &[&str])] = &[
    // F5 tmsh / iApp management scripts.
    (
        "f5-iapps",
        &["iapp::", "tmsh::create_app", "sys application template"],
    ),
    (
        "f5-tmsh",
        &["tmsh::", "tmsh create", "tmsh modify", "tmsh list"],
    ),
    // EDA-tool Tcl (synthesis / P&R / simulation).
    (
        "xilinx-eda-tcl",
        &["synth_design", "launch_runs", "create_project", "read_xdc"],
    ),
    (
        "synopsys-eda-tcl",
        &["compile_ultra", "dc_shell", "link_design", "set_max_area"],
    ),
    (
        "cadence-eda-tcl",
        &["set_db", "innovus", "genus", "init_design"],
    ),
    (
        "intel-quartus-eda-tcl",
        &["quartus_", "project_new", "set_global_assignment"],
    ),
    ("mentor-eda-tcl", &["vsim", "vlog", "vcom", "questa"]),
    // Expect automation.
    (
        "expect",
        &["spawn", "expect_before", "send_user", "interact"],
    ),
];

/// Whether `marker` appears in `haystack` at a word boundary on its **left**
/// (start-of-string or a non-word byte before it). Unlike [`has_word`] this
/// puts no constraint on the byte after the marker, so command-prefix markers
/// like `tmsh::` / `quartus_` (followed by more identifier) still match.
fn contains_token(haystack: &str, marker: &str) -> bool {
    let hbytes = haystack.as_bytes();
    let mut i = 0;
    while let Some(off) = haystack[i..].find(marker) {
        let start = i + off;
        if start == 0 || !is_word_byte(hbytes[start - 1]) {
            return true;
        }
        i = start + 1;
    }
    false
}

/// Detect a dialect from a script's *content* signatures (iRules `when`,
/// F5 tmsh/iApp, EDA-tool commands, expect), over the scanned `head`.
fn detect_from_content(head: &str) -> Option<&'static str> {
    if has_irules_when(head) {
        return Some("f5-irules");
    }
    for (dialect, markers) in CONTENT_SIGNATURES {
        if markers.iter().any(|m| contains_token(head, m)) {
            return Some(dialect);
        }
    }
    None
}

/// The dialect named by a `#!…` shebang on the first line (`expect`, or
/// `tclsh<x.y>`), or `None`.
fn shebang_dialect(source: &str) -> Option<&'static str> {
    let first = source.lines().next()?;
    if !first.starts_with("#!") {
        return None;
    }
    let lower = first.to_ascii_lowercase();
    if has_word(&lower, "expect") {
        return Some("expect");
    }
    shebang_tclsh_version(&lower).and_then(|ver| tcl_version_dialect(&ver))
}

/// The *content-borne* dialect signals, in the priority the project wants:
/// a `package require` / `vsatisfies` Tcl-version guard (tokenised, so comments
/// and string literals never match) first, then the command / `when`-clause
/// content signatures (iRules, F5 tmsh / iApp, EDA tools, expect). Returns the
/// dialect for the first signal found in `head`, or `None`.
///
/// Split out from [`detect_dialect`] so it can be applied to a cheap head
/// first and — only on a miss — the full source, catching a signal that a very
/// large script reveals only near its end without paying that cost on the
/// common (signal-near-the-top) path.
fn detect_content_signals(head: &str) -> Option<&'static str> {
    if let Some(ver) = scan_tokens_for_tcl_version(head, 0)
        && let Some(d) = tcl_version_dialect(&ver)
    {
        return Some(d);
    }
    detect_from_content(head)
}

/// The canonical dialect detector shared by the LSP, editors, CLI tooling, and
/// AI integrations. Given a document's `source` and optional `filename`,
/// returns a best-guess dialect (never fails — falls back to `default` when no
/// heuristic fires).
///
/// Heuristics are applied in this priority order, most-trusted first:
/// 1. an explicit `# tcl-dialect: <name>` directive (first
///    [`DIALECT_DIRECTIVE_SCAN_LINES`] lines);
/// 2. the `#!…` shebang (`expect`, `tclsh<x.y>`);
/// 3. a tokenised `package require ?-exact? Tcl <x.y>` or `package vsatisfies
///    [package require Tcl] <x.y>` version guard;
/// 4. content signatures — iRules `when EVENT {`, F5 `tmsh::` / iApp, EDA-tool
///    commands (Xilinx / Synopsys / Cadence / Quartus / Mentor), `expect`;
/// 5. the filename extension ([`dialect_from_extension`]) — a fallback, since a
///    `.tcl` file's *content* is a stronger signal than its name;
/// 6. the caller's `default` (the editor / LSP / XDG configuration).
///
/// The `BigIP` config-object tier (tier between content and extension in the
/// project's model) is applied by the `tcl-bigip` layer, which wraps this
/// detector — it isn't reachable from the registry crate.
///
/// Tiers 3–4 (the *content-borne* signals) are scanned streaming-style: the
/// cheap [`DETECT_SCAN_BYTES`]-byte head is tried first and, only if it is
/// inconclusive, the full source — so a signal that a very large script reveals
/// only near its end is still caught without paying that cost when a signal
/// sits near the top (the common case).
///
/// An explicit editor / workspace-folder dialect is applied by the caller
/// *above* this function and overrides everything here.
#[must_use]
pub fn detect_dialect(source: &str, filename: Option<&str>, default: &'static str) -> &'static str {
    // 1. Explicit directive always wins.
    if let Some(d) = detect_dialect_directive(source) {
        return d;
    }
    // 2. Shebang.
    if let Some(d) = shebang_dialect(source) {
        return d;
    }
    // 3-4. Content-borne signals (package/vsatisfies version, then content
    //       signatures). Try the cheap head first; on a miss, scan the whole
    //       source so a late signal in a huge script is still caught.
    let head = scan_head(source);
    if let Some(d) = detect_content_signals(head).or_else(|| {
        (source.len() > head.len())
            .then(|| detect_content_signals(source))
            .flatten()
    }) {
        return d;
    }
    // 5. Filename extension — only when the content gave nothing away.
    if let Some(d) = filename.and_then(dialect_from_extension) {
        return d;
    }
    // 6. Caller default (editor / LSP / XDG config).
    default
}

/// Detect a Tcl dialect from a script's *content* — used when no explicit
/// dialect is configured. Checks, in priority order: a `# tcl-dialect:`
/// directive (first [`DIALECT_DIRECTIVE_SCAN_LINES`] lines), a
/// `#!…tclsh<x.y>` / `#!…expect` shebang (first line), then a tokenised
/// `package require ?-exact? Tcl <x.y>` or `package vsatisfies [package require
/// Tcl] <x.y>` version guard over the first [`DETECT_SCAN_BYTES`] bytes.
/// Returns `None` when no hint is found.
///
/// (Conf-wrapped-iRules detection is an additional fallback; it depends on
/// the BIG-IP layer and is handled there.)
#[must_use]
pub fn detect_dialect_from_source(source: &str) -> Option<&'static str> {
    if let Some(d) = detect_dialect_directive(source) {
        return Some(d);
    }
    if let Some(first) = source.lines().next()
        && first.starts_with("#!")
    {
        let lower = first.to_ascii_lowercase();
        if has_word(&lower, "expect") {
            return Some("expect");
        }
        if let Some(ver) = shebang_tclsh_version(&lower)
            && let Some(d) = tcl_version_dialect(&ver)
        {
            return Some(d);
        }
    }
    let head = scan_head(source);
    let scan =
        |s: &str| scan_tokens_for_tcl_version(s, 0).and_then(|ver| tcl_version_dialect(&ver));
    // Head first (cheap); on a miss, the full source so a late guard in a huge
    // script is still caught.
    scan(head).or_else(|| (source.len() > head.len()).then(|| scan(source)).flatten())
}

impl DialectSet {
    /// Whether `name` denotes the F5 iRules dialect.  Accepts the
    /// canonical `f5-irules` and the legacy `irules` alias.  This is
    /// the single source of truth for the "is this iRules?" check
    /// that compiler / LSP passes need, replacing the per-module
    /// `matches!(dialect, Some("f5-irules" | "irules"))` copies.
    #[must_use]
    pub fn is_irules_dialect(name: Option<&str>) -> bool {
        matches!(name, Some("f5-irules" | "irules"))
    }

    /// Whether `name`'s ensemble commands are *fixed* — the dialect
    /// ships a closed set of subcommands with no user-extensible
    /// ensembles — so the minifier may safely shorten subcommands to
    /// their unambiguous prefix.  True for the F5 dialect family
    /// (`f5-irules` / `f5-iapps` / `f5-bigip`).  Single source of
    /// truth for the minifier's former `_FIXED_ENSEMBLE_DIALECTS`
    /// list.
    #[must_use]
    pub fn has_fixed_ensembles(name: Option<&str>) -> bool {
        matches!(name, Some("f5-irules" | "f5-iapps" | "f5-bigip"))
    }

    /// Parse a dialect name string to a single-bit set.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "bpf" => Self::BPF,
            "tcl8.4" => Self::TCL84,
            "tcl8.5" => Self::TCL85,
            "tcl8.6" => Self::TCL86,
            "tcl9.0" => Self::TCL90,
            "tcl9.1" => Self::TCL91,
            "f5-irules" => Self::IRULES,
            "f5-iapps" => Self::IAPPS,
            "tk" => Self::TK,
            "expect" => Self::EXPECT,
            "synopsys-eda-tcl" => Self::SYNOPSYS,
            "cadence-eda-tcl" => Self::CADENCE,
            "xilinx-eda-tcl" => Self::XILINX,
            "intel-quartus-eda-tcl" => Self::QUARTUS,
            "mentor-eda-tcl" => Self::MENTOR,
            _ => return None,
        })
    }

    /// The canonical dialect name for a single-bit set — the inverse of
    /// [`Self::parse`]. `None` for an empty set, a combinator flag with no
    /// single dialect name (`ALL_TCL`, `TCL85_PLUS`, …), or a multi-bit set.
    #[must_use]
    pub fn canonical_name(self) -> Option<&'static str> {
        Some(match self {
            Self::BPF => "bpf",
            Self::TCL84 => "tcl8.4",
            Self::TCL85 => "tcl8.5",
            Self::TCL86 => "tcl8.6",
            Self::TCL90 => "tcl9.0",
            Self::TCL91 => "tcl9.1",
            Self::IRULES => "f5-irules",
            Self::IAPPS => "f5-iapps",
            Self::TK => "tk",
            Self::EXPECT => "expect",
            Self::SYNOPSYS => "synopsys-eda-tcl",
            Self::CADENCE => "cadence-eda-tcl",
            Self::XILINX => "xilinx-eda-tcl",
            Self::QUARTUS => "intel-quartus-eda-tcl",
            Self::MENTOR => "mentor-eda-tcl",
            _ => return None,
        })
    }

    /// The canonical dialect names of every single bit set in `self`, in
    /// declaration order (`TCL84` before `TCL85` before … before `MENTOR`) —
    /// so a Tcl-version run always lists lowest-to-highest.  Combinator bits
    /// with no single name (see [`Self::canonical_name`]) contribute nothing;
    /// every *primitive* dialect this set contains is still listed via its
    /// own bit.
    #[must_use]
    pub fn member_names(self) -> Vec<&'static str> {
        const PRIMITIVES: &[DialectSet] = &[
            DialectSet::TCL84,
            DialectSet::TCL85,
            DialectSet::TCL86,
            DialectSet::TCL90,
            DialectSet::TCL91,
            DialectSet::IRULES,
            DialectSet::IAPPS,
            DialectSet::TK,
            DialectSet::EXPECT,
            DialectSet::SYNOPSYS,
            DialectSet::CADENCE,
            DialectSet::XILINX,
            DialectSet::QUARTUS,
            DialectSet::MENTOR,
            DialectSet::BPF,
        ];
        PRIMITIVES
            .iter()
            .filter(|&&bit| self.contains(bit))
            .filter_map(|&bit| bit.canonical_name())
            .collect()
    }

    /// Resolve `name` to the Tcl core-grammar version its `expr` evaluator
    /// behaves like, for version-gated *language grammar* features — e.g.
    /// TIP 201 (`in`/`ni`, 8.5+) and TIP 461 (`lt`/`le`/`gt`/`ge`, 9.0+).
    ///
    /// Distinct from [`Self::parse`] + `TCL85_PLUS`/`TCL90_PLUS`, which model
    /// per-*command* dialect gating and deliberately keep vendor dialects
    /// (the EDA tools, F5 iApps/tmsh) as their own bits rather than folding
    /// them into a Tcl version — most standard-library commands they don't
    /// ship are missing because of their own restricted command surface, not
    /// because of version. Expr grammar is different: every dialect here is
    /// either plain Tcl or a vendor shell embedding a real Tcl core, so it
    /// inherits that embedded core's operators wholesale. `f5-irules` is the
    /// one case where the two deliberately disagree: it advertises an
    /// 8.6-shaped command *signature* but its `expr` evaluator is a genuine
    /// embedded Tcl 8.4.6, so version-*dependent behaviour* (this included)
    /// follows 8.4, not 8.6.
    ///
    /// Source: the per-dialect base-Tcl-version table in
    /// `docs/design/compiler/dialects-events.md`. Returns `None` for
    /// `f5-bigip` (a custom config parser, not Tcl at all) and for any name
    /// this table has no documented base version for (`tk`, `bpf`) — callers
    /// should treat `None` as "can't reason about this dialect's grammar
    /// version," not "assume plain Tcl."
    #[must_use]
    pub fn expr_grammar_base_version(name: &str) -> Option<Self> {
        Some(match name {
            "tcl8.4" | "f5-irules" => Self::TCL84,
            "tcl8.5"
            | "f5-iapps"
            | "f5-tmsh"
            | "xilinx-eda-tcl"
            | "intel-quartus-eda-tcl"
            | "mentor-eda-tcl" => Self::TCL85,
            "tcl8.6" | "synopsys-eda-tcl" | "cadence-eda-tcl" | "expect" => Self::TCL86,
            "tcl9.0" => Self::TCL90,
            "tcl9.1" => Self::TCL91,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_dialects_is_sorted_and_complete() {
        let d = available_dialects();
        assert_eq!(d.len(), 16);
        let mut sorted = d.to_vec();
        sorted.sort_unstable();
        assert_eq!(d, sorted.as_slice(), "must be pre-sorted");
        // Spot-check the names that diverge from DialectSet::parse.
        assert!(d.contains(&"bpf"));
        assert!(d.contains(&"f5-bigip"));
        assert!(d.contains(&"f5-tmsh"));
        assert!(d.contains(&"tcl9.1"));
        assert!(!d.contains(&"tk"));
    }

    #[test]
    fn from_name_roundtrip() {
        assert_eq!(DialectSet::parse("tcl8.6"), Some(DialectSet::TCL86));
        assert_eq!(DialectSet::parse("f5-irules"), Some(DialectSet::IRULES));
        assert_eq!(DialectSet::parse("unknown"), None);
    }

    #[test]
    fn canonical_name_is_the_exact_inverse_of_parse() {
        // Every name `parse` accepts round-trips through `canonical_name`
        // back to the identical string, for every primitive dialect.
        for name in [
            "bpf",
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "f5-irules",
            "f5-iapps",
            "tk",
            "expect",
            "synopsys-eda-tcl",
            "cadence-eda-tcl",
            "xilinx-eda-tcl",
            "intel-quartus-eda-tcl",
            "mentor-eda-tcl",
        ] {
            let bit = DialectSet::parse(name).unwrap_or_else(|| panic!("{name} must parse"));
            assert_eq!(bit.canonical_name(), Some(name));
        }
    }

    #[test]
    fn canonical_name_is_none_for_combinators_and_empty() {
        assert_eq!(DialectSet::empty().canonical_name(), None);
        assert_eq!(DialectSet::ALL_TCL.canonical_name(), None);
        assert_eq!(DialectSet::TCL85_PLUS.canonical_name(), None);
        assert_eq!(
            (DialectSet::TCL84 | DialectSet::TCL85).canonical_name(),
            None
        );
    }

    #[test]
    fn member_names_lists_lowest_tcl_version_first() {
        assert_eq!(
            DialectSet::ALL_TCL.member_names(),
            vec!["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
        );
        assert_eq!(
            DialectSet::TCL85_PLUS.member_names(),
            vec!["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
        );
        assert_eq!(DialectSet::TCL84.member_names(), vec!["tcl8.4"]);
        assert!(DialectSet::empty().member_names().is_empty());
    }

    #[test]
    fn expr_grammar_base_version_matches_documented_runtime_bases() {
        let base = DialectSet::expr_grammar_base_version;
        // Plain Tcl versions map to themselves.
        assert_eq!(base("tcl8.4"), Some(DialectSet::TCL84));
        assert_eq!(base("tcl9.1"), Some(DialectSet::TCL91));
        // iRules' *runtime* base (8.4.6) wins over its 8.6-shaped command
        // signature — version-dependent behaviour follows the runtime.
        assert_eq!(base("f5-irules"), Some(DialectSet::TCL84));
        // EDA / F5 vendor shells inherit their documented embedded core —
        // unlike `DialectSet::TCL85_PLUS`, which deliberately excludes them.
        for d in [
            "f5-iapps",
            "f5-tmsh",
            "xilinx-eda-tcl",
            "intel-quartus-eda-tcl",
            "mentor-eda-tcl",
        ] {
            assert_eq!(base(d), Some(DialectSet::TCL85), "{d}");
        }
        for d in ["synopsys-eda-tcl", "cadence-eda-tcl", "expect"] {
            assert_eq!(base(d), Some(DialectSet::TCL86), "{d}");
        }
        // Not Tcl at all / no documented base version: stay `None` rather
        // than guessing.
        assert_eq!(base("f5-bigip"), None);
        assert_eq!(base("tk"), None);
        assert_eq!(base("bpf"), None);
        assert_eq!(base("nonsense"), None);
    }

    #[test]
    fn fixed_ensembles_cover_the_f5_family_only() {
        for d in ["f5-irules", "f5-iapps", "f5-bigip"] {
            assert!(
                DialectSet::has_fixed_ensembles(Some(d)),
                "{d} should be fixed"
            );
        }
        assert!(!DialectSet::has_fixed_ensembles(Some("tcl8.6")));
        assert!(!DialectSet::has_fixed_ensembles(Some("irules")));
        assert!(!DialectSet::has_fixed_ensembles(None));
    }

    #[test]
    fn is_irules_dialect_accepts_canonical_and_legacy_alias() {
        // The canonical `f5-irules` and the legacy `irules` alias both match;
        // every other dialect — and `None` — do not. There is no
        // active-dialect-wrapper case: that contextvar mechanism has no
        // global equivalent here by design.
        assert!(DialectSet::is_irules_dialect(Some("f5-irules")));
        assert!(DialectSet::is_irules_dialect(Some("irules")));
        assert!(!DialectSet::is_irules_dialect(Some("tcl8.6")));
        assert!(!DialectSet::is_irules_dialect(Some("f5-iapps")));
        assert!(!DialectSet::is_irules_dialect(Some("f5-bigip")));
        assert!(!DialectSet::is_irules_dialect(None));
    }

    #[test]
    fn tcl85_plus_contains_86_and_90() {
        assert!(DialectSet::TCL85_PLUS.contains(DialectSet::TCL85));
        assert!(DialectSet::TCL85_PLUS.contains(DialectSet::TCL86));
        assert!(DialectSet::TCL85_PLUS.contains(DialectSet::TCL90));
        assert!(!DialectSet::TCL85_PLUS.contains(DialectSet::TCL84));
    }
}
