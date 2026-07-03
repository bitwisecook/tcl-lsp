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
/// Number of leading lines scanned for a `package require Tcl` line.
const PKG_REQUIRE_SCAN_LINES: usize = 30;

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

/// Extract `<x.y>` from a `^\s*package\s+require\s+(-exact\s+)?Tcl\s*<x.y>` line.
/// (`Tcl` is matched case-sensitively.)
fn package_require_tcl_version(line: &str) -> Option<String> {
    let mut t = line.trim_start().strip_prefix("package")?;
    if !t.starts_with(char::is_whitespace) {
        return None;
    }
    t = t.trim_start().strip_prefix("require")?;
    if !t.starts_with(char::is_whitespace) {
        return None;
    }
    t = t.trim_start();
    if let Some(rest) = t.strip_prefix("-exact") {
        if !rest.starts_with(char::is_whitespace) {
            return None;
        }
        t = rest.trim_start();
    }
    let t = t.strip_prefix("Tcl")?.trim_start();
    let bytes = t.as_bytes();
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
    Some(t[..j].to_string())
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
        assert_eq!(detect_dialect("# tcl-dialect: tcl8.5\nputs hi\n", None, DEF), "tcl8.5");
    }

    #[test]
    fn extension_beats_generic_content() {
        assert_eq!(detect_dialect("puts hi\n", Some("x.irule"), DEF), "f5-irules");
        assert_eq!(detect_dialect("spawn ssh host\n", Some("y.exp"), DEF), "expect");
        assert_eq!(detect_dialect("read_xdc c.xdc\n", Some("c.xdc"), DEF), "xilinx-eda-tcl");
    }

    #[test]
    fn shebang_detected() {
        assert_eq!(detect_dialect("#!/usr/bin/expect -f\nspawn ssh\n", None, DEF), "expect");
        assert_eq!(detect_dialect("#!/usr/bin/tclsh8.6\nputs hi\n", None, DEF), "tcl8.6");
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
        assert_eq!(detect_dialect("synth_design -top foo\n", None, DEF), "xilinx-eda-tcl");
        assert_eq!(detect_dialect("compile_ultra -gate_clock\n", None, DEF), "synopsys-eda-tcl");
        assert_eq!(detect_dialect("set_db init_design\ninit_design\n", None, DEF), "cadence-eda-tcl");
        assert_eq!(detect_dialect("tmsh::create ltm pool p\n", None, DEF), "f5-tmsh");
    }

    #[test]
    fn expect_content_without_shebang() {
        assert_eq!(detect_dialect("spawn ssh host\nexpect_before timeout\n", None, DEF), "expect");
    }

    #[test]
    fn plain_tcl_falls_back_to_default() {
        assert_eq!(detect_dialect("set x 1\nputs $x\n", None, DEF), "tcl9.0");
        assert_eq!(detect_dialect("package require Tcl 8.6\n", None, DEF), "tcl8.6");
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
    ("f5-iapps", &["iapp::", "tmsh::create_app", "sys application template"]),
    ("f5-tmsh", &["tmsh::", "tmsh create", "tmsh modify", "tmsh list"]),
    // EDA-tool Tcl (synthesis / P&R / simulation).
    ("xilinx-eda-tcl", &["synth_design", "launch_runs", "create_project", "read_xdc"]),
    ("synopsys-eda-tcl", &["compile_ultra", "dc_shell", "link_design", "set_max_area"]),
    ("cadence-eda-tcl", &["set_db", "innovus", "genus", "init_design"]),
    ("intel-quartus-eda-tcl", &["quartus_", "project_new", "set_global_assignment"]),
    ("mentor-eda-tcl", &["vsim", "vlog", "vcom", "questa"]),
    // Expect automation.
    ("expect", &["spawn", "expect_before", "send_user", "interact"]),
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

/// The canonical dialect detector shared by the LSP, editors, CLI tooling, and
/// AI integrations. Given a document's `source` and optional `filename`,
/// returns a best-guess dialect (never fails — falls back to `default` when no
/// heuristic fires).
///
/// Heuristics are applied in priority order over the first
/// [`DETECT_SCAN_BYTES`] bytes:
/// 1. an explicit `# tcl-dialect: <name>` directive;
/// 2. the filename extension ([`dialect_from_extension`]);
/// 3. the `#!…` shebang (`expect`, `tclsh<x.y>`);
/// 4. content signatures — iRules `when EVENT {`, F5 `tmsh::` / iApp, EDA-tool
///    commands (Xilinx / Synopsys / Cadence / Quartus / Mentor), `expect`;
/// 5. a `package require ?-exact? Tcl <x.y>` line.
#[must_use]
pub fn detect_dialect(source: &str, filename: Option<&str>, default: &'static str) -> &'static str {
    let head = scan_head(source);

    // 1. Explicit directive always wins.
    if let Some(d) = detect_dialect_directive(head) {
        return d;
    }
    // 2. Extension (a decisive `.irule` / `.exp` beats ambiguous content).
    if let Some(d) = filename.and_then(dialect_from_extension) {
        return d;
    }
    // 3. Shebang.
    if let Some(first) = head.lines().next()
        && first.starts_with("#!")
    {
        let lower = first.to_ascii_lowercase();
        if has_word(&lower, "expect") {
            return "expect";
        }
        if let Some(ver) = shebang_tclsh_version(&lower)
            && let Some(d) = tcl_version_dialect(&ver)
        {
            return d;
        }
    }
    // 4. Content signatures.
    if let Some(d) = detect_from_content(head) {
        return d;
    }
    // 5. `package require Tcl <x.y>`.
    for line in head.lines().take(PKG_REQUIRE_SCAN_LINES) {
        if let Some(ver) = package_require_tcl_version(line)
            && let Some(d) = tcl_version_dialect(&ver)
        {
            return d;
        }
    }
    default
}

/// Detect a Tcl dialect from a script's *content* — used when no explicit
/// dialect is configured. Checks, in priority order: a `# tcl-dialect:`
/// directive (first [`DIALECT_DIRECTIVE_SCAN_LINES`] lines), a
/// `#!…tclsh<x.y>` / `#!…expect` shebang (first line), then a
/// `package require ?-exact? Tcl <x.y>` line (first 30 lines). Returns `None`
/// when no hint is found.
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
    for line in source.lines().take(PKG_REQUIRE_SCAN_LINES) {
        if let Some(ver) = package_require_tcl_version(line)
            && let Some(d) = tcl_version_dialect(&ver)
        {
            return Some(d);
        }
    }
    None
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
