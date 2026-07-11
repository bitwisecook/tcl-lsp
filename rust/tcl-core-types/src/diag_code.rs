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

//! Stable diagnostic-code identities and their published metadata.
//!
//! Every diagnostic the analyser, compiler-checks pipeline, CLI, and LSP emit is
//! tagged with one of these codes (`W210`, `E001`, `O100`, `IRULE3102`, …).
//! Historically each site carried a bare string literal; [`DiagCode`] makes the
//! set a single typed vocabulary so a code can no longer be mistyped, an unknown
//! code is a parse error at the one boundary that ingests user config, and the
//! full catalogue is enumerable ([`DiagCode::ALL`]).
//!
//! Each variant also carries its *documentation metadata* — the section/category,
//! one-line description, and default-on flag that the published code tables
//! (`docs/generated/diagnostic_tables.md` and friends) are generated from. This
//! module is therefore the single source of truth for both the code identities
//! and the docs: `cargo xtask diag-tables` renders the tables from
//! [`DiagCode::ALL`], and a check-mode guard fails the build if the committed
//! tables ever drift from what the enum would produce.
//!
//! The string spelling is the wire/UI contract (LSP `Diagnostic.code`, CLI
//! output, `--disable W123`, the docs); [`DiagCode::as_str`] / [`core::fmt::Display`]
//! produce it and [`core::str::FromStr`] parses it back.

use core::fmt;
use core::str::FromStr;

/// The coarse family a [`DiagCode`] belongs to, derived from its letter prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagFamily {
    /// `E###` — hard errors (parse / arity).
    Error,
    /// `W###` — lint warnings.
    Warning,
    /// `I###` — informational notes.
    Info,
    /// `S###` — shimmer (representation-change) notes.
    Shimmer,
    /// `T###` — taint / injection findings.
    Taint,
    /// `O###` — optimiser rewrites and constant-fold notes.
    Optimisation,
    /// `IRULE####` — iRules-dialect flow checks.
    IRule,
}

/// The documentation *section* a diagnostic belongs to — a finer grouping than
/// [`DiagFamily`] (e.g. `W###` codes split across `Warning`, `Variable`,
/// `Security`, `Hint`, `Tclpkg`). Declaration order is the order sections appear
/// in the generated tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagSection {
    /// Hard parse / arity errors.
    Error,
    /// Style & best-practice lints.
    Warning,
    /// Variable usage lints.
    Variable,
    /// Security / injection lints.
    Security,
    /// Non-actionable hints.
    Hint,
    /// Shimmer (representation-change) notes.
    Shimmer,
    /// Taint / injection findings.
    Taint,
    /// Tk-toolkit lints.
    Tk,
    /// iRules-dialect flow checks.
    Irules,
    /// iRules security findings.
    IrulesSecurity,
    /// iRules variable-scoping checks.
    IrulesVariable,
    /// `tclpkg` package-manager diagnostics.
    Tclpkg,
}

impl DiagSection {
    /// The lower-case section key used in the generated tables.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Variable => "variable",
            Self::Security => "security",
            Self::Hint => "hint",
            Self::Shimmer => "shimmer",
            Self::Taint => "taint",
            Self::Tk => "tk",
            Self::Irules => "irules",
            Self::IrulesSecurity => "irules_security",
            Self::IrulesVariable => "irules_variable",
            Self::Tclpkg => "tclpkg",
        }
    }
}

/// An optimisation-pass category. A profile enables a set of categories; the
/// generated `readability`/`standard`/`full` columns and the runtime
/// `profile_to_disabled` gate both key off this (see
/// `tcl_compiler::optimiser::profiles`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OptCategory {
    /// Idiomatic rewrites, no code removal/restructuring.
    Readability,
    /// Constant folding and propagation.
    ConstantFolding,
    /// Pattern-recognition rewrites.
    Pattern,
    /// Dead-code / dead-store elimination.
    Dce,
    /// Code motion (hoisting / sinking).
    CodeMotion,
    /// Tail-call / recursion transforms.
    Recursion,
}

impl OptCategory {
    /// The lower-case category key used in the generated tables.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Readability => "readability",
            Self::ConstantFolding => "constant_folding",
            Self::Pattern => "pattern",
            Self::Dce => "dce",
            Self::CodeMotion => "code_motion",
            Self::Recursion => "recursion",
        }
    }

    /// Whether the `readability` profile enables this category (it only enables
    /// [`OptCategory::Readability`]).
    #[must_use]
    pub const fn in_readability_profile(self) -> bool {
        matches!(self, Self::Readability)
    }

    /// Whether the `standard` profile enables this category (`readability` +
    /// constant folding + pattern recognition).
    #[must_use]
    pub const fn in_standard_profile(self) -> bool {
        matches!(
            self,
            Self::Readability | Self::ConstantFolding | Self::Pattern
        )
    }
}

/// The published-table metadata for one [`DiagCode`]: either a diagnostic
/// (section + default-on flag) or an optimisation (category). Both carry the
/// one-line description the tables render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocRow {
    /// A diagnostic row (`E###`/`W###`/`S###`/`T###`/`IRULE###`/…).
    Diagnostic {
        /// The documentation section it groups under.
        section: DiagSection,
        /// Whether the diagnostic is emitted by default (the table's `Default`
        /// column: `✓` when true, `✗` when opt-in).
        default_on: bool,
        /// Whether the code is *internal* — always active and not exposed as a
        /// user-configurable toggle (parse/structure errors, host-config
        /// validators, translation markers). Excluded from the editor-settings
        /// catalogue.
        internal: bool,
        /// The one-line description.
        description: &'static str,
    },
    /// An optimisation row (`O###`).
    Optimisation {
        /// The optimisation category.
        category: OptCategory,
        /// The one-line description.
        description: &'static str,
    },
}

/// Error returned when a string is not a known [`DiagCode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownDiagCode;

impl fmt::Display for UnknownDiagCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown diagnostic code")
    }
}

macro_rules! diagnostic_codes {
    (
        $( $variant:ident => $str:literal, $kind:ident ( $($meta:tt)* ) ; )+
    ) => {
        /// A stable diagnostic-code identity. See the [module docs](self).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum DiagCode {
            $(
                #[doc = concat!("The `", $str, "` diagnostic.")]
                $variant,
            )+
        }

        impl DiagCode {
            /// Every diagnostic code, in declaration order. Lets the doc
            /// generator and a completeness guard iterate the full catalogue.
            pub const ALL: &'static [DiagCode] = &[ $(DiagCode::$variant),+ ];

            /// The stable wire/UI spelling (e.g. `"W210"`).
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(DiagCode::$variant => $str,)+ }
            }

            /// The published-table metadata for this code (section/category,
            /// description, default-on flag). The single source the generated
            /// code tables are rendered from.
            #[must_use]
            pub const fn doc_row(self) -> DocRow {
                match self { $(DiagCode::$variant => diagnostic_codes!(@row $kind ( $($meta)* )),)+ }
            }

            /// Whether this is an *internal* diagnostic — always active and not
            /// exposed as a user-configurable toggle. Optimisations are never
            /// internal. Drives the editor-settings catalogue's exclusion set.
            #[must_use]
            pub const fn is_internal(self) -> bool {
                matches!(
                    self.doc_row(),
                    DocRow::Diagnostic { internal: true, .. }
                )
            }
        }

        impl FromStr for DiagCode {
            type Err = UnknownDiagCode;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($str => Ok(DiagCode::$variant),)+
                    _ => Err(UnknownDiagCode),
                }
            }
        }
    };

    (@row diag ( $sec:ident, $default:literal, $desc:literal )) => {
        DocRow::Diagnostic {
            section: DiagSection::$sec,
            default_on: $default,
            internal: false,
            description: $desc,
        }
    };
    (@row diag_internal ( $sec:ident, $default:literal, $desc:literal )) => {
        DocRow::Diagnostic {
            section: DiagSection::$sec,
            default_on: $default,
            internal: true,
            description: $desc,
        }
    };
    (@row opt ( $cat:ident, $desc:literal )) => {
        DocRow::Optimisation {
            category: OptCategory::$cat,
            description: $desc,
        }
    };
}

diagnostic_codes! {
    E001 => "E001", diag(Error, true, "Missing dispatch word — e.g. bare `string` without a subcommand, or `$obj` with no TclOO method.");
    E002 => "E002", diag(Error, true, "Too few arguments for command.");
    E003 => "E003", diag(Error, true, "Too many arguments for command.");
    E004 => "E004", diag_internal(Error, true, "Malformed `if` command — missing clauses or extra words after `else`.");
    E005 => "E005", diag(Error, true, "Wrong argument-count shape for command — an in-range count that doesn't fit the command's key/value-pair or paired-argument pattern (e.g. an odd `dict create` tail, an unpaired `foreach` list, or a `switch` count matching neither its shorthand nor its pattern/body-pair form).");
    E100 => "E100", diag_internal(Error, true, "Unmatched `]` — missing opening `[`?");
    E101 => "E101", diag_internal(Error, true, "Missing `{` after `switch` — case bodies follow without braces.");
    E102 => "E102", diag_internal(Error, true, "Unmatched `}` — missing opening `{`?");
    E103 => "E103", diag_internal(Error, true, "Missing `}` — a nested body consumed this closing brace.");
    E200 => "E200", diag(Error, true, "Unterminated command — the parser could not tell where it ends (missing `]` / `\"` / `}`).");
    E201 => "E201", diag_internal(Error, true, "Unterminated command substitution — missing close bracket `]`.");
    E202 => "E202", diag_internal(Error, true, "Unterminated double-quoted string — missing closing `\"`.");
    E203 => "E203", diag_internal(Error, true, "Unterminated braced word — missing closing `}`.");
    E204 => "E204", diag_internal(Error, true, "Extra characters after the close brace of a `${name}` variable reference.");
    E205 => "E205", diag_internal(Error, true, "Extra characters after the close quote in a variable name.");
    E206 => "E206", diag_internal(Error, true, "Missing close brace for a `${name}` variable reference.");
    H300 => "H300", diag(Hint, true, "Possible paste error — repeated assignment to same variable with same value.");
    I230 => "I230", diag(Hint, true, "Constant branch condition — the alternate branch is provably unreachable.");
    I231 => "I231", diag(Hint, true, "Constant switch arm condition — the arm is provably unreachable.");
    Irule1001 => "IRULE1001", diag(Irules, true, "Command invalid or ineffective in this iRules event.");
    Irule1002 => "IRULE1002", diag(Irules, true, "Unknown iRules event name.");
    Irule1003 => "IRULE1003", diag(Irules, true, "Deprecated iRules event.");
    Irule1004 => "IRULE1004", diag(Irules, true, "`when` block missing explicit `priority`.");
    Irule1005 => "IRULE1005", diag(Irules, true, "Data event without a matching `*::collect` call.");
    Irule1006 => "IRULE1006", diag(Irules, true, "`*::payload` without a matching `*::collect` call.");
    Irule1007 => "IRULE1007", diag(Irules, true, "`*::collect` without a matching `*::release` on the same connection side.");
    Irule1008 => "IRULE1008", diag(Irules, true, "`*::release` without a matching `*::collect` on the same connection side.");
    Irule1201 => "IRULE1201", diag(Irules, true, "HTTP command used after `HTTP::respond`/`HTTP::redirect`.");
    Irule1202 => "IRULE1202", diag(Irules, true, "Multiple `HTTP::respond`/`HTTP::redirect` on different branches.");
    Irule2001 => "IRULE2001", diag(Irules, true, "Deprecated `matchclass` — use `class match` instead.");
    Irule2002 => "IRULE2002", diag(Irules, true, "Deprecated iRules command.");
    Irule2003 => "IRULE2003", diag(Irules, true, "Unsafe iRules command.");
    Irule2101 => "IRULE2101", diag(Irules, true, "Heavy `regexp` in a high-frequency event — consider `string match` or data-group.");
    Irule3001 => "IRULE3001", diag(IrulesSecurity, true, "Tainted data in HTTP response body.");
    Irule3002 => "IRULE3002", diag(IrulesSecurity, true, "Tainted data in HTTP header or cookie value.");
    Irule3003 => "IRULE3003", diag(IrulesSecurity, true, "Tainted data in `log` command — log injection risk.");
    Irule3004 => "IRULE3004", diag(IrulesSecurity, true, "Tainted data in an `HTTP::redirect` URL — open-redirect risk.");
    Irule3101 => "IRULE3101", diag(IrulesSecurity, true, "`HTTP::uri`/`HTTP::path` set to value not provably starting with `/`.");
    Irule3102 => "IRULE3102", diag(IrulesSecurity, true, "`HTTP::path`/`HTTP::uri`/`HTTP::query` getter used without `-normalized`.");
    Irule3103 => "IRULE3103", diag_internal(IrulesSecurity, true, "Manual split/match of an un-normalised URI getter — parse-differential / traversal risk.");
    Irule4001 => "IRULE4001", diag(IrulesVariable, true, "Write to `static::` variable outside `RULE_INIT`.");
    Irule4002 => "IRULE4002", diag(IrulesVariable, true, "Generic `static::` variable name — collision likely across iRules.");
    Irule4003 => "IRULE4003", diag(IrulesVariable, true, "Variable scoping concern across events.");
    Irule4004 => "IRULE4004", diag(IrulesVariable, true, "Constant `set` in per-request event could be hoisted to an earlier once-per-connection event.");
    Irule4005 => "IRULE4005", diag(IrulesVariable, true, "Potential race — `static::` variable written outside `RULE_INIT` and read in another event.");
    Irule5001 => "IRULE5001", diag(Irules, true, "Ungated `log` in a high-frequency event.");
    Irule5002 => "IRULE5002", diag(Irules, true, "`drop`/`reject`/`discard` without `event disable all` or `return`.");
    Irule5003 => "IRULE5003", diag_internal(Irules, true, "Loop condition `$x != 0` can skip zero when decremented past it — use `$x > 0`.");
    Irule5004 => "IRULE5004", diag(Irules, true, "`DNS::return` without `return`.");
    Irule5005 => "IRULE5005", diag(Irules, true, "Direct proc invocation without `call` — use `call proc_name`.");
    Irule5006 => "IRULE5006", diag(Irules, true, "Top-level-only command used inside a nested body.");
    Irule5007 => "IRULE5007", diag(Irules, true, "Event-context command used at top level outside a `when` block.");
    Irule6001 => "IRULE6001", diag_internal(Irules, true, "`global`/`::`-qualified variable forces CMP compatibility mode, pinning the virtual server to one TMM — use `static::`.");
    O100 => "O100", opt(ConstantFolding, "Propagate constant variables into expressions and command arguments.");
    O101 => "O101", opt(ConstantFolding, "Fold constant integer expressions.");
    O102 => "O102", opt(ConstantFolding, "Forward a variable's single reaching literal load to its use sites.");
    O103 => "O103", opt(ConstantFolding, "Fold static procedure calls using interprocedural summaries.");
    O104 => "O104", opt(Pattern, "Fold static string build chains into a single assignment.");
    O105 => "O105", opt(ConstantFolding, "Propagate constants into variable references and detect redundant computations (GVN/CSE).");
    O106 => "O106", opt(CodeMotion, "Hoist loop-invariant computations.");
    O107 => "O107", opt(Dce, "Eliminate unreachable dead code.");
    O108 => "O108", opt(Dce, "Eliminate transitively dead code.");
    O109 => "O109", opt(Dce, "Eliminate dead stores.");
    O110 => "O110", opt(ConstantFolding, "Canonicalise expressions (InstCombine).");
    O111 => "O111", opt(Readability, "Brace expression performance hints (paired with W100).");
    O112 => "O112", opt(Dce, "Eliminate constant-condition compound statements.");
    O113 => "O113", opt(ConstantFolding, "Strength-reduce expressions (`x**2` → `x*x`, `x%8` → `x&7`).");
    O114 => "O114", opt(Readability, "Recognise `incr` idiom (`set x [expr {$x + N}]` → `incr x N`).");
    O115 => "O115", opt(Readability, "Remove redundant nested `[expr {...}]` in expression context.");
    O116 => "O116", opt(ConstantFolding, "Fold constant `[list a b c]` to literal value.");
    O117 => "O117", opt(Readability, "Simplify `[string length $s] == 0` → `$s eq \"\"`.");
    O118 => "O118", opt(ConstantFolding, "Fold constant `[lindex {a b c} 1]` to element.");
    O119 => "O119", opt(Pattern, "Pack consecutive `set` literals into `lassign`/`foreach`.");
    O120 => "O120", opt(Readability, "Prefer `eq`/`ne` over `==`/`!=` for string comparisons.");
    O121 => "O121", opt(Recursion, "Rewrite self-recursive tail calls to `tailcall`.");
    O122 => "O122", opt(Recursion, "Convert fully tail-recursive proc to iterative `while` loop.");
    O123 => "O123", opt(Recursion, "Detect non-tail recursion eligible for accumulator introduction (hint only).");
    O124 => "O124", opt(Dce, "Comment out unused procs in iRules (not called from any event).");
    O125 => "O125", opt(CodeMotion, "Sink side-effect-free assignments into the deepest decision block (`if`/`switch`) that uses them.");
    O126 => "O126", opt(Dce, "Remove unused variable assignments — eliminate `set` statements for variables that are never read.");
    O127 => "O127", opt(CodeMotion, "Inline single-use variable assignment — eliminate redundant variable load by folding `set` into the use site.");
    O128 => "O128", opt(Readability, "Rewrite `[expr {[llength $L] - N}]` / `[expr {[string length $s] - N}]` to `end-(N-1)` when used as an index argument.");
    O129 => "O129", opt(ConstantFolding, "Fold a pure builtin command substitution with constant arguments (`[string length ...]`, `[join ...]`, `[format ...]`, `[dict get ...]`, …).");
    O130 => "O130", opt(Pattern, "Fold static `lappend` list build chains into a single assignment.");
    S100 => "S100", diag(Shimmer, true, "Single shimmer outside a loop — object internal representation changed.");
    S101 => "S101", diag(Shimmer, true, "Shimmer inside a loop body — per-iteration representation conversion cost.");
    S102 => "S102", diag(Shimmer, true, "Variable oscillates between two types across loop iterations.");
    S110 => "S110", diag(Shimmer, true, "Byte-array value coerced to a string by a string operation — binary representation corrupted.");
    T100 => "T100", diag(Taint, true, "Tainted data flows into a dangerous sink: `eval`/`uplevel`/`subst`/unbraced-`expr`/`exec` (code-execution); braced `expr` operands (numeric/type-coercion).");
    T101 => "T101", diag(Taint, true, "Tainted data flows into an output command (`puts`).");
    T102 => "T102", diag(Taint, true, "Tainted data in option position without `--` terminator — option injection risk.");
    T103 => "T103", diag_internal(Taint, true, "Tainted data in a `regexp`/`regsub` pattern — regex-injection or ReDoS risk.");
    T104 => "T104", diag(Taint, true, "Tainted data in a network-address argument (e.g. `socket`) — SSRF risk.");
    T105 => "T105", diag(Taint, true, "Tainted data in a cross-interpreter eval subcommand (`interp eval`/`invokehidden`) — code-execution risk.");
    T106 => "T106", diag_internal(Taint, true, "Already-encoded value passed through a command that re-encodes it — double-encoding.");
    Tk1001 => "TK1001", diag_internal(Tk, true, "Geometry-manager conflict — `pack` and `grid` used on the same parent.");
    Tk1002 => "TK1002", diag_internal(Tk, true, "Widget path references a non-existent parent widget.");
    Tk1003 => "TK1003", diag_internal(Tk, true, "Unknown option for a widget command.");
    W001 => "W001", diag(Warning, true, "Unknown subcommand.");
    W002 => "W002", diag(Warning, true, "Command is disabled in active dialect profile.");
    W003 => "W003", diag(Warning, true, "Expression operator not available in active dialect.");
    W004 => "W004", diag(Warning, true, "Command option is not available in the active dialect.");
    W100 => "W100", diag(Warning, true, "Unbraced expression argument — prevents byte-compilation and risks double substitution.");
    W101 => "W101", diag(Security, true, "`eval` with string concatenation — code injection risk.");
    W102 => "W102", diag(Security, true, "`subst` on variable input — code injection risk.");
    W103 => "W103", diag(Security, true, "`open` with pipeline `|` — command injection risk.");
    W104 => "W104", diag(Warning, true, "String concatenation for list building — use `lappend` instead.");
    W105 => "W105", diag(Warning, true, "Unbraced code block or missing `variable` declaration in `namespace eval`.");
    W106 => "W106", diag(Warning, true, "Dangerous unbraced `switch` body — risks double substitution.");
    W108 => "W108", diag(Warning, true, "Non-ASCII characters in token content.");
    W110 => "W110", diag(Warning, true, "Use `eq`/`ne` instead of `==`/`!=` for string comparison.");
    W111 => "W111", diag(Warning, true, "Line exceeds maximum length (see `tclLsp.style.lineLength`).");
    W112 => "W112", diag(Warning, true, "Trailing whitespace.");
    W113 => "W113", diag(Warning, true, "Procedure shadows built-in command.");
    W114 => "W114", diag(Warning, true, "Redundant nested `[expr {...}]` — already in expression context.");
    W115 => "W115", diag(Warning, true, "Backslash-newline in comment silently swallows the next line.");
    W116 => "W116", diag(Warning, true, "Stub command shadows built-in command.");
    W117 => "W117", diag(Warning, true, "Stub expression definition shadows built-in function or operator.");
    W118 => "W118", diag(Warning, true, "Inconsistent line endings.");
    W120 => "W120", diag(Warning, true, "Command used without a corresponding `package require`.");
    W121 => "W121", diag(Warning, true, "Subnet mask has non-contiguous bits.");
    W122 => "W122", diag(Warning, true, "Mistyped IPv4 address (octet > 255 or leading zero).");
    W123 => "W123", diag(Hint, true, "Unresolved command — not found in registry, user procs, or `unknown` handler.");
    W124 => "W124", diag(Warning, true, "Invalid IP address literal.");
    W125 => "W125", diag(Warning, true, "Orphaned control-flow keyword used as standalone command.");
    W126 => "W126", diag(Warning, true, "Non-channel value in channel argument position.");
    W127 => "W127", diag(Warning, true, "Value not in the command's allowed set.");
    W128 => "W128", diag(Warning, true, "Command called after it was renamed or deleted earlier in this file; the call falls through to the `unknown` handler.");
    W130 => "W130", diag(Tclpkg, true, "tclpkg.tcl requires package but it is not in tclpkg.lock — run 'tcl pkg install'.");
    W131 => "W131", diag(Tclpkg, true, "tclpkg.lock is out of sync with tclpkg.tcl — run 'tcl pkg install'.");
    W132 => "W132", diag(Tclpkg, true, "tclpkg.lock integrity mismatch — CAS hash differs from lockfile.");
    W133 => "W133", diag(Tclpkg, true, "tclpkg.tcl directive not permitted in safe mode.");
    W134 => "W134", diag(Tclpkg, true, "Package resolved but no pkgIndex.tcl found — 'package require' will fail at runtime.");
    W135 => "W135", diag(Warning, true, "Command requires a newer package version than the resolved `package require`.");
    W136 => "W136", diag(Warning, true, "Option requires a newer package version than the resolved `package require`.");
    W200 => "W200", diag(Warning, true, "`exec` result not captured or binary format modifier requires newer Tcl.");
    W201 => "W201", diag(Warning, true, "Manual path concatenation — use `file join` instead.");
    W210 => "W210", diag(Variable, true, "Variable read before set.");
    W211 => "W211", diag(Variable, true, "Variable set but never used.");
    W212 => "W212", diag(Variable, true, "Variable substitution where name expected (`set $x`, `incr $x`, `info exists $x`, etc.).");
    W213 => "W213", diag(Variable, true, "Variable may not exist — use `unset -nocomplain` to suppress the error.");
    W214 => "W214", diag(Variable, true, "Unused proc parameter — argument is declared but never read in the procedure body.");
    W215 => "W215", diag(Variable, true, "Variable name unreachable via $-substitution (creatable via set/info exists/upvar but no $-form can read it).");
    W216 => "W216", diag(Variable, true, "Broken brace-form array element reference — ``${arr}(x)`` parses as scalar+literal, ``${arr($foo)}`` does not substitute the index.");
    W217 => "W217", diag(Variable, true, "`unset` unsets nothing — every argument is consumed as an option (`-nocomplain` / `--`); prefix a `-`-named variable with `--`.");
    W220 => "W220", diag(Variable, true, "Dead store — variable set but overwritten before use.");
    W230 => "W230", diag(Warning, true, "Constant list index out of range — lindex/lrange/lreplace silently return empty or clamp.");
    W231 => "W231", diag(Warning, true, "Constant list index out of range — lset raises a runtime error.");
    W232 => "W232", diag(Warning, true, "Constant string index out of range — string index/range/replace/insert silently return empty or no-op.");
    W233 => "W233", diag(Warning, true, "Division or modulo by a provably-zero divisor — raises 'divide by zero' at runtime.");
    W240 => "W240", diag(Warning, true, "Loop condition is a constant false — body never executes.");
    W241 => "W241", diag(Warning, true, "Loop is provably infinite — constant-true condition with no break/return, zero/wrong-direction counter step.");
    W242 => "W242", diag(Hint, false, "Loop termination cannot be proven — counter not provably modified by the loop body or step.");
    W250 => "W250", diag(Warning, true, "Instantiating an `oo::abstract` class — abstract classes cannot be created directly; use a concrete subclass.");
    W300 => "W300", diag(Security, true, "`source` with variable argument — code execution risk.");
    W301 => "W301", diag(Security, true, "`uplevel` with string-built script — injection risk.");
    W302 => "W302", diag(Security, true, "`catch` without result variable — errors are silently swallowed.");
    W303 => "W303", diag(Security, true, "Regexp vulnerable to catastrophic backtracking (ReDoS).");
    W304 => "W304", diag(Security, true, "Missing option terminator `--` on option-bearing commands.");
    W306 => "W306", diag(Security, true, "Substitution in literal-expected argument position.");
    W307 => "W307", diag(Security, true, "Non-literal command name — variable or command substitution as command.");
    W308 => "W308", diag(Security, true, "`subst` without `-nocommands` — risk of unintended command execution.");
    W309 => "W309", diag(Security, true, "`eval`/`uplevel` with `subst` — double substitution risk.");
    W310 => "W310", diag_internal(Security, true, "Hardcoded credential in a password/auth argument — store secrets outside source.");
    W311 => "W311", diag_internal(Security, true, "Channel set to `-encoding binary` with a non-binary `-translation` — may corrupt data or enable encoding-differential attacks.");
    W312 => "W312", diag_internal(Security, true, "`interp eval` with multiple or unbraced script words — concatenated like `eval`, injection risk.");
    W313 => "W313", diag(Security, true, "Destructive file operation with variable path — path-traversal risk.");
}

impl DiagCode {
    /// The family this code belongs to, from its letter prefix.
    #[must_use]
    pub const fn family(self) -> DiagFamily {
        let s = self.as_str().as_bytes();
        if s.len() >= 5
            && s[0] == b'I'
            && s[1] == b'R'
            && s[2] == b'U'
            && s[3] == b'L'
            && s[4] == b'E'
        {
            return DiagFamily::IRule;
        }
        // `TK###` Tk-toolkit lints share the `T` prefix with taint codes but are
        // plain warnings, so classify them before the single-byte match below.
        if s.len() >= 2 && s[0] == b'T' && s[1] == b'K' {
            return DiagFamily::Warning;
        }
        match s[0] {
            b'E' => DiagFamily::Error,
            b'I' => DiagFamily::Info,
            b'S' => DiagFamily::Shimmer,
            b'T' => DiagFamily::Taint,
            b'O' => DiagFamily::Optimisation,
            // `W###` (and any future prefix) fall through here; every variant's
            // spelling begins with one of the prefixes matched above.
            _ => DiagFamily::Warning,
        }
    }

    /// True for optimiser rewrite codes (`O###`) — replaces the scattered
    /// `code.starts_with('O')` checks the master optimiser switch keys on.
    #[must_use]
    pub const fn is_optimisation(self) -> bool {
        matches!(self.family(), DiagFamily::Optimisation)
    }

    /// True for hard-error codes (`E###`) — replaces the scattered
    /// `code.starts_with('E')` "did any error fire?" gates.
    #[must_use]
    pub const fn is_error(self) -> bool {
        matches!(self.family(), DiagFamily::Error)
    }

    /// Whether the analyser emits this code **speculatively from a single file**
    /// and a later workspace / cross-file resolution pass may *refine it away*.
    /// Such a code is not stable until the deep diagnostics pass has consulted
    /// the workspace package database and the cross-file source graph, so it is
    /// the only kind held back from the progressive **fast tier** (#844):
    /// publishing it un-refined would resurface a false positive the deep pass
    /// then retracts (the startup false-positive W120 that #841 eliminated).
    ///
    /// The set is intentionally tiny and intrinsic to what these codes *mean*:
    ///
    /// - **W120** — "command used without a corresponding `package require`":
    ///   suppressed once the workspace package database shows the command's
    ///   package is (transitively) available (`refine_workspace_w120`, #723/#804).
    /// - **W123** — "unresolved command": suppressed once the package database
    ///   resolves it (`refine_workspace_w123`, #832) or a workspace proc defines
    ///   it (the cross-file `project_diagnostics` pass).
    ///
    /// Codes that the deep pass only ever *adds* (compiler / optimiser findings,
    /// synthesised cross-file arity) are **not** listed here: they are simply
    /// absent from the fast tier and appear when the deep tier lands, which is
    /// additive, never a retraction.  This is the single source of truth for the
    /// classification — consumers (the LSP fast-tier partition) fetch it rather
    /// than re-encoding the code set.
    #[must_use]
    pub const fn refined_by_workspace(self) -> bool {
        matches!(self, Self::W120 | Self::W123)
    }

    /// The one-line description rendered in the published code tables.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self.doc_row() {
            DocRow::Diagnostic { description, .. } | DocRow::Optimisation { description, .. } => {
                description
            }
        }
    }

    /// The optimisation category, for `O###` codes (`None` for diagnostics).
    #[must_use]
    pub const fn opt_category(self) -> Option<OptCategory> {
        match self.doc_row() {
            DocRow::Optimisation { category, .. } => Some(category),
            DocRow::Diagnostic { .. } => None,
        }
    }

    /// The documentation section, for diagnostics (`None` for `O###` codes).
    #[must_use]
    pub const fn diag_section(self) -> Option<DiagSection> {
        match self.doc_row() {
            DocRow::Diagnostic { section, .. } => Some(section),
            DocRow::Optimisation { .. } => None,
        }
    }

    /// Whether the code is emitted by default — the table's `Default` column.
    /// Optimisations are always on within their profile, so they report `true`.
    #[must_use]
    pub const fn default_on(self) -> bool {
        match self.doc_row() {
            DocRow::Diagnostic { default_on, .. } => default_on,
            DocRow::Optimisation { .. } => true,
        }
    }
}

impl fmt::Display for DiagCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_from_str_round_trip() {
        for &code in DiagCode::ALL {
            assert_eq!(DiagCode::from_str(code.as_str()), Ok(code));
        }
    }

    #[test]
    fn unknown_code_is_rejected() {
        assert_eq!(DiagCode::from_str("W999"), Err(UnknownDiagCode));
        assert_eq!(DiagCode::from_str("nonsense"), Err(UnknownDiagCode));
    }

    #[test]
    fn strings_are_unique() {
        let mut seen: heapless_set::Set = heapless_set::Set::new();
        for &code in DiagCode::ALL {
            assert!(
                seen.insert(code.as_str()),
                "duplicate spelling {}",
                code.as_str()
            );
        }
    }

    #[test]
    fn refined_by_workspace_is_exactly_w120_and_w123() {
        // #844: the progressive fast tier holds back exactly the codes a
        // workspace / cross-file pass can retract.  Pin the whole set so it
        // cannot silently grow (which would delay a stable diagnostic) or shrink
        // (which would resurface the #841 startup false positive).
        for &code in DiagCode::ALL {
            let expected = matches!(code, DiagCode::W120 | DiagCode::W123);
            assert_eq!(
                code.refined_by_workspace(),
                expected,
                "{code} misclassified: refined_by_workspace must be true iff the \
                 code is W120 or W123",
            );
        }
        assert!(DiagCode::W120.refined_by_workspace());
        assert!(DiagCode::W123.refined_by_workspace());
        assert!(!DiagCode::W121.refined_by_workspace());
        assert!(!DiagCode::E002.refined_by_workspace());
        assert!(!DiagCode::O111.refined_by_workspace());
    }

    #[test]
    fn families_are_classified() {
        assert_eq!(DiagCode::W210.family(), DiagFamily::Warning);
        assert_eq!(DiagCode::E001.family(), DiagFamily::Error);
        assert_eq!(DiagCode::O100.family(), DiagFamily::Optimisation);
        assert_eq!(DiagCode::T100.family(), DiagFamily::Taint);
        assert_eq!(DiagCode::S100.family(), DiagFamily::Shimmer);
        assert_eq!(DiagCode::I230.family(), DiagFamily::Info);
        assert_eq!(DiagCode::Irule3102.family(), DiagFamily::IRule);
        assert!(DiagCode::O129.is_optimisation());
        assert!(!DiagCode::W210.is_optimisation());
    }

    #[test]
    fn doc_metadata_is_consistent() {
        for &code in DiagCode::ALL {
            // Every code carries a non-empty description.
            assert!(
                !code.description().is_empty(),
                "{} has an empty description",
                code.as_str()
            );
            // The two metadata shapes line up with `is_optimisation()`:
            // optimisations carry a category, diagnostics carry a section.
            match code.doc_row() {
                DocRow::Optimisation { .. } => {
                    assert!(code.is_optimisation(), "{} is not O-family", code.as_str());
                    assert!(code.opt_category().is_some());
                    assert!(code.diag_section().is_none());
                }
                DocRow::Diagnostic { .. } => {
                    assert!(!code.is_optimisation(), "{} is O-family", code.as_str());
                    assert!(code.diag_section().is_some());
                    assert!(code.opt_category().is_none());
                }
            }
        }
    }

    #[test]
    fn is_error_matches_error_family() {
        for &code in DiagCode::ALL {
            assert_eq!(
                code.is_error(),
                code.family() == DiagFamily::Error,
                "{} is_error disagrees with family",
                code.as_str()
            );
        }
        assert!(DiagCode::E001.is_error());
        assert!(!DiagCode::W210.is_error());
    }

    #[test]
    fn default_on_is_total_and_true_for_optimisations() {
        for &code in DiagCode::ALL {
            // Every code answers default_on without panicking; optimisations
            // are always on within their profile.
            let on = code.default_on();
            if code.is_optimisation() {
                assert!(on, "{} optimisation must be default-on", code.as_str());
            }
        }
    }

    #[test]
    fn internal_flag_classifies_non_configurable_codes() {
        use core::str::FromStr;
        // Internal — always-on parse/structure errors, host-config validators,
        // and translation markers; excluded from the user-configurable editor
        // settings.  The internal codes are the E20x/E10x/E004 parse errors,
        // IRULE3103/5003/6001 flow internals, TK100x, and W31x; E204–E206 are
        // the parse-error siblings of E201–E203.
        for s in [
            "E004",
            "E100",
            "E101",
            "E102",
            "E103",
            "E201",
            "E202",
            "E203",
            "E204",
            "E205",
            "E206",
            "IRULE3103",
            "IRULE5003",
            "IRULE6001",
            "T103",
            "T106",
            "TK1001",
            "TK1002",
            "TK1003",
            "W310",
            "W311",
            "W312",
        ] {
            assert!(
                DiagCode::from_str(s).unwrap().is_internal(),
                "{s} must be internal"
            );
        }
        // User-configurable — regular lints, taint/security warnings (including
        // IRULE3004/T104/T105), and iRules flow
        // checks are all togglable, so never internal.
        for s in [
            "E001",
            "W001",
            "W210",
            "IRULE1001",
            "IRULE3001",
            "IRULE3004",
            "T101",
            "T104",
            "T105",
        ] {
            assert!(
                !DiagCode::from_str(s).unwrap().is_internal(),
                "{s} must be user-configurable (not internal)"
            );
        }
        // Optimisations are never internal.
        for &code in DiagCode::ALL {
            if code.is_optimisation() {
                assert!(!code.is_internal(), "{} opt is not internal", code.as_str());
            }
        }
    }

    #[test]
    fn diag_section_as_str_covers_every_variant() {
        use DiagSection::*;
        for (section, key) in [
            (Error, "error"),
            (Warning, "warning"),
            (Variable, "variable"),
            (Security, "security"),
            (Hint, "hint"),
            (Shimmer, "shimmer"),
            (Taint, "taint"),
            (Tk, "tk"),
            (Irules, "irules"),
            (IrulesSecurity, "irules_security"),
            (IrulesVariable, "irules_variable"),
            (Tclpkg, "tclpkg"),
        ] {
            assert_eq!(section.as_str(), key);
        }
    }

    #[test]
    fn opt_category_as_str_and_profiles() {
        use OptCategory::*;
        for (cat, key) in [
            (Readability, "readability"),
            (ConstantFolding, "constant_folding"),
            (Pattern, "pattern"),
            (Dce, "dce"),
            (CodeMotion, "code_motion"),
            (Recursion, "recursion"),
        ] {
            assert_eq!(cat.as_str(), key);
        }
        // readability profile enables only Readability.
        assert!(Readability.in_readability_profile());
        assert!(!ConstantFolding.in_readability_profile());
        assert!(!Dce.in_readability_profile());
        // standard profile enables readability + constant folding + pattern.
        assert!(Readability.in_standard_profile());
        assert!(ConstantFolding.in_standard_profile());
        assert!(Pattern.in_standard_profile());
        assert!(!Dce.in_standard_profile());
        assert!(!CodeMotion.in_standard_profile());
        assert!(!Recursion.in_standard_profile());
    }

    #[test]
    fn diag_code_display_matches_as_str() {
        use core::fmt::Write;
        for &code in DiagCode::ALL {
            let mut buf: heapless_fmt::Buf = heapless_fmt::Buf::new();
            write!(buf, "{code}").unwrap();
            assert_eq!(buf.as_str(), code.as_str());
        }
    }

    #[test]
    fn unknown_diag_code_displays_message() {
        use core::fmt::Write;
        // Render `UnknownDiagCode` through Display (exercises the fmt impl).
        let mut buf: heapless_fmt::Buf = heapless_fmt::Buf::new();
        write!(buf, "{UnknownDiagCode}").unwrap();
        assert_eq!(buf.as_str(), "unknown diagnostic code");
    }

    /// Minimal `no_std` `fmt::Write` sink so the Display test needs no `String`.
    mod heapless_fmt {
        pub struct Buf {
            bytes: [u8; 64],
            len: usize,
        }
        impl Buf {
            pub const fn new() -> Self {
                Self {
                    bytes: [0; 64],
                    len: 0,
                }
            }
            pub fn as_str(&self) -> &str {
                core::str::from_utf8(&self.bytes[..self.len]).unwrap()
            }
        }
        impl core::fmt::Write for Buf {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let b = s.as_bytes();
                if self.len + b.len() > self.bytes.len() {
                    return Err(core::fmt::Error);
                }
                self.bytes[self.len..self.len + b.len()].copy_from_slice(b);
                self.len += b.len();
                Ok(())
            }
        }
    }

    /// Tiny `no_std` set for the uniqueness test (avoids pulling in std).
    mod heapless_set {
        pub struct Set {
            items: [&'static str; 256],
            len: usize,
        }
        impl Set {
            pub const fn new() -> Self {
                Self {
                    items: [""; 256],
                    len: 0,
                }
            }
            pub fn insert(&mut self, s: &'static str) -> bool {
                let mut i = 0;
                while i < self.len {
                    if self.items[i] == s {
                        return false;
                    }
                    i += 1;
                }
                self.items[self.len] = s;
                self.len += 1;
                true
            }
        }
    }
}
