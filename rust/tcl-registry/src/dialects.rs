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

        /// All standard Tcl versions.
        const ALL_TCL = Self::TCL84.bits() | Self::TCL85.bits()
                      | Self::TCL86.bits() | Self::TCL90.bits();

        /// Tcl 8.5 and later.
        const TCL85_PLUS = Self::TCL85.bits() | Self::TCL86.bits() | Self::TCL90.bits();

        /// Tcl 8.6 and later.
        const TCL86_PLUS = Self::TCL86.bits() | Self::TCL90.bits();

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
/// (`Tcl` is matched case-sensitively, mirroring the Python regex.)
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

/// Detect a Tcl dialect from a script's *content* — used when no explicit
/// dialect is configured. Checks, in priority order: a `# tcl-dialect:`
/// directive (first [`DIALECT_DIRECTIVE_SCAN_LINES`] lines), a
/// `#!…tclsh<x.y>` / `#!…expect` shebang (first line), then a
/// `package require ?-exact? Tcl <x.y>` line (first 30 lines). Returns `None`
/// when no hint is found.
///
/// (The Python source additionally falls back to conf-wrapped-iRules detection;
/// that depends on the BIG-IP layer and is handled there.)
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
        assert_eq!(d.len(), 15);
        let mut sorted = d.to_vec();
        sorted.sort_unstable();
        assert_eq!(d, sorted.as_slice(), "must be pre-sorted");
        // Spot-check the names that diverge from DialectSet::parse.
        assert!(d.contains(&"bpf"));
        assert!(d.contains(&"f5-bigip"));
        assert!(d.contains(&"f5-tmsh"));
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
