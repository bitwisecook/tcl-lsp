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

//! Special-variable registry — the source of truth for interpreter-provided
//! globals (scalars, arrays, and namespaces the runtime, `init.tcl`, or the
//! platform inject into every interpreter).
//!
//! These variables — `auto_path`, `env`, `errorInfo`, `tcl_platform`, the
//! iRules `static::` namespace, … — behave differently from user variables in
//! two ways the analyser and LSP must respect:
//!
//! * A **write is observed by the runtime** even when the script never reads
//!   the value back (`set auto_path …` configures the package auto-loader).
//!   Such a write is *not* a dead store / unused variable (issue #831).
//! * A **read is always defined** — the interpreter seeds the variable before
//!   user code runs — so a bare read is never read-before-set.
//!
//! The set is **dialect-versioned**: it is a [`DialectSet`] membership table
//! exactly like [`crate::spec::CommandSpec`]. Standard Tcl, F5 iRules, and (in
//! future) Tk / EDA shells each provide a slightly different set — iRules has
//! no `argv`/`env`/`auto_path`, keeps its own BIG-IP `tcl_platform` keys, and
//! adds the CMP-safe [`static::`](https://clouddocs.f5.com/api/irules/static.html)
//! namespace. Consumers resolve the active dialect once and query this table;
//! no consumer hardcodes a name list.

use crate::dialects::DialectSet;

/// The value shape a special variable holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialVarKind {
    /// A plain scalar (`tcl_version`, `argv0`, `auto_path`).
    Scalar,
    /// An array indexed by string keys (`env`, `tcl_platform`, `auto_index`).
    Array,
    /// A namespace of related values rather than a single variable — the
    /// iRules `static::` CMP-safe global namespace. Accessed as
    /// `static::name`, never as a bare `$static`.
    Namespace,
}

/// Whether user code is expected to write a special variable, or only read
/// the interpreter-provided value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarAccess {
    /// The interpreter provides the value for reading; a user write is
    /// unusual (`tcl_version`, `tcl_platform`, `argv0`).
    ReadOnly,
    /// User code legitimately writes it to configure runtime behaviour
    /// (`auto_path`, `env`, `tcl_precision`, `errorInfo`).
    ReadWrite,
}

/// Where a special variable comes from — used only for hover prose grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarOrigin {
    /// Seeded by the C interpreter core (`tcl_version`, `argv`, `errorInfo`).
    Interpreter,
    /// Set up by the script library's `init.tcl` auto-loader (`auto_path`,
    /// `auto_index`).
    AutoLoader,
    /// Platform / build description (`tcl_platform`).
    Platform,
    /// The process environment (`env`).
    Environment,
    /// Injected by a non-standard dialect's runtime (iRules `static::`).
    Dialect,
}

/// One known key of an array-shaped special variable, with the dialects that
/// provide it. Arrays with open, data-dependent keys (`env`, `auto_index`)
/// carry an empty key list.
#[derive(Debug, Clone, Copy)]
pub struct SpecialVarKey {
    /// The array index (`os`, `wordSize`, `tmmVersion`).
    pub key: &'static str,
    /// Dialects in which this key is present. `tcl_platform(tmmVersion)` is
    /// iRules-only; `tcl_platform(pointerSize)` is Tcl 8.6+.
    pub dialects: DialectSet,
    /// One-line description for hover.
    pub summary: &'static str,
}

/// Metadata for one interpreter-provided special variable.
#[derive(Debug, Clone, Copy)]
pub struct SpecialVarSpec {
    /// The bare variable name, without a leading `$` or `::` (`auto_path`,
    /// `tcl_platform`, `static`).
    pub name: &'static str,
    /// Value shape.
    pub kind: SpecialVarKind,
    /// Read-only vs. user-writable semantics.
    pub access: VarAccess,
    /// Provenance, for hover grouping.
    pub origin: VarOrigin,
    /// Dialects that provide this variable. A membership test against the
    /// active dialect decides whether the variable exists there.
    pub dialects: DialectSet,
    /// Known array keys (empty for scalars, namespaces, and open-keyed
    /// arrays). Each key is itself dialect-gated.
    pub keys: &'static [SpecialVarKey],
    /// The runtime observes a *write* to this variable independent of any
    /// script-level read — so `set NAME …` is never a dead store / unused
    /// variable even when nothing reads `$NAME`. True for the auto-loader,
    /// environment, precision, and error-context variables; false for pure
    /// read-only info globals whose value the runtime never re-consumes.
    pub externally_read: bool,
    /// iRules only: accessing this variable as a plain global demotes the
    /// virtual server from CMP (clustered multiprocessing) to a single TMM.
    /// The CMP-safe form is the `static::` alias (e.g.
    /// `static::tcl_platform`). Always false outside iRules.
    pub cmp_unsafe: bool,
    /// One-line hover summary.
    pub summary: &'static str,
}

impl SpecialVarSpec {
    /// Whether this variable is available in `dialect` (a resolved
    /// [`DialectSet`] flag for the active dialect).
    #[must_use]
    pub fn available_in(&self, dialect: DialectSet) -> bool {
        self.dialects.intersects(dialect)
    }

    /// The known keys of this array that are present in `dialect`.
    pub fn keys_in(&self, dialect: DialectSet) -> impl Iterator<Item = &SpecialVarKey> {
        self.keys
            .iter()
            .filter(move |k| k.dialects.intersects(dialect))
    }
}

/// Resolve a dialect *name* to the [`DialectSet`] flag used for membership
/// tests. Empty, generic (`"tcl"`), or config-only (`"f5-bigip"`) names — any
/// name [`DialectSet::parse`] does not recognise — resolve to the union of all
/// standard Tcl versions, so plain-Tcl analysis sees the full standard set.
#[must_use]
pub fn resolve_dialect(dialect: &str) -> DialectSet {
    DialectSet::parse(dialect).unwrap_or(DialectSet::ALL_TCL)
}

/// Look up a special variable by bare name, ignoring dialect.
///
/// Returns the single spec regardless of which dialects provide it; callers
/// that care about availability should test [`SpecialVarSpec::available_in`]
/// or use [`special_var_in_dialect`].
#[must_use]
pub fn special_var(name: &str) -> Option<&'static SpecialVarSpec> {
    SPECIAL_VARS.iter().find(|v| v.name == name)
}

/// Look up a special variable that is available in `dialect` (a dialect name
/// such as `"tcl8.6"`, `"f5-irules"`, or `""` for generic Tcl).
#[must_use]
pub fn special_var_in_dialect(name: &str, dialect: &str) -> Option<&'static SpecialVarSpec> {
    let d = resolve_dialect(dialect);
    special_var(name).filter(|v| v.available_in(d))
}

/// Whether `name` is an interpreter-provided special variable in `dialect`.
///
/// A read of such a name is always defined (the runtime seeds it before user
/// code runs), so it is never read-before-set.
#[must_use]
pub fn is_special_var(name: &str, dialect: &str) -> bool {
    special_var_in_dialect(name, dialect).is_some()
}

/// Whether a *write* to `name` in `dialect` is observed by the runtime — so
/// `set NAME …` must not be flagged as a dead store (W220) or unused variable
/// (W211) even when the script never reads `$NAME`. This is the fix for the
/// `set auto_path …` false positive (issue #831).
#[must_use]
pub fn is_externally_read(name: &str, dialect: &str) -> bool {
    special_var_in_dialect(name, dialect).is_some_and(|v| v.externally_read)
}

/// Iterate the special variables available in `dialect`, in table order.
pub fn special_vars_for_dialect(dialect: &str) -> impl Iterator<Item = &'static SpecialVarSpec> {
    let d = resolve_dialect(dialect);
    SPECIAL_VARS.iter().filter(move |v| v.available_in(d))
}

/// `tcl_platform` array keys. Standard Tcl and F5 iRules diverge here: iRules
/// reports BIG-IP values (`os` = `"BIG-IP"`) and adds `tmmVersion`, while
/// dropping the desktop-only keys.
const TCL_PLATFORM_KEYS: &[SpecialVarKey] = &[
    SpecialVarKey {
        key: "platform",
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        summary: "OS family: `unix`, `windows`, or (older) `macintosh`.",
    },
    SpecialVarKey {
        key: "os",
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        summary: "Operating-system name (`Linux`, `Windows NT`; `BIG-IP` on iRules).",
    },
    SpecialVarKey {
        key: "osVersion",
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        summary: "Operating-system version (the BIG-IP version on iRules).",
    },
    SpecialVarKey {
        key: "byteOrder",
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        summary: "`littleEndian` or `bigEndian`.",
    },
    SpecialVarKey {
        key: "wordSize",
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        summary: "Size in bytes of the native `long` (4 or 8).",
    },
    SpecialVarKey {
        key: "machine",
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        summary: "Hardware architecture (`x86_64`, `arm64`; hostname on iRules).",
    },
    SpecialVarKey {
        key: "pointerSize",
        dialects: DialectSet::TCL86_PLUS,
        summary: "Size in bytes of a native pointer (Tcl 8.6+).",
    },
    SpecialVarKey {
        key: "user",
        dialects: DialectSet::ALL_TCL,
        summary: "Login name of the user running the process.",
    },
    SpecialVarKey {
        key: "engine",
        dialects: DialectSet::TCL86_PLUS,
        summary: "Interpreter engine name — `Tcl` for the reference implementation (8.6+).",
    },
    SpecialVarKey {
        key: "pathSeparator",
        dialects: DialectSet::TCL90_PLUS,
        summary: "Separator between paths in a search-path list (Tcl 9.0+).",
    },
    SpecialVarKey {
        key: "threaded",
        dialects: DialectSet::ALL_TCL,
        summary: "Present and true when the interpreter was built with threads.",
    },
    SpecialVarKey {
        key: "debug",
        dialects: DialectSet::ALL_TCL,
        summary: "Present when built with symbolic debugging enabled.",
    },
    SpecialVarKey {
        key: "tmmVersion",
        dialects: DialectSet::IRULES,
        summary: "iRules only: full TMM version including build number.",
    },
];

/// The complete special-variable table, dialect-versioned.
///
/// Ordering groups related variables (auto-loader, error context, platform)
/// for readability; lookups are by name so order is not semantically
/// significant.
pub const SPECIAL_VARS: &[SpecialVarSpec] = &[
    // ── Command-line arguments (standard Tcl only; iRules has none) ─────────
    SpecialVarSpec {
        name: "argc",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Number of command-line arguments in `argv`.",
    },
    SpecialVarSpec {
        name: "argv",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "List of command-line arguments passed after the script name.",
    },
    SpecialVarSpec {
        name: "argv0",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Name of the script (or interpreter) being executed.",
    },
    // ── Auto-loader / package machinery (init.tcl) ─────────────────────────
    SpecialVarSpec {
        name: "auto_path",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Search path (a list of directories) the package/auto-loader \
                  consults at runtime. Usually configured, not read, by scripts.",
    },
    SpecialVarSpec {
        name: "auto_index",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Maps an auto-loadable command name to the script that defines it.",
    },
    SpecialVarSpec {
        name: "auto_execs",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Cache mapping external command names to their resolved executable paths.",
    },
    SpecialVarSpec {
        name: "auto_noexec",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "When set, disables auto-execution of external commands from the shell.",
    },
    SpecialVarSpec {
        name: "auto_noload",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "When set, disables the auto-loader (`unknown` will not auto-load).",
    },
    // ── Environment ────────────────────────────────────────────────────────
    SpecialVarSpec {
        name: "env",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Environment,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "The process environment. Writes propagate to child processes; \
                  keys are environment-variable names.",
    },
    // ── Error context ──────────────────────────────────────────────────────
    SpecialVarSpec {
        name: "errorInfo",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Human-readable stack trace of the most recent error.",
    },
    SpecialVarSpec {
        name: "errorCode",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Machine-readable error code (a list) of the most recent error.",
    },
    // ── Interpreter / library description ──────────────────────────────────
    SpecialVarSpec {
        name: "tcl_version",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        keys: &[],
        externally_read: false,
        cmp_unsafe: true,
        summary: "Interpreter version as `major.minor` (e.g. `8.6`).",
    },
    SpecialVarSpec {
        name: "tcl_patchLevel",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        keys: &[],
        externally_read: false,
        cmp_unsafe: true,
        summary: "Full patch level of the interpreter (e.g. `8.6.13`).",
    },
    SpecialVarSpec {
        name: "tcl_library",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Directory holding the standard Tcl script library (`init.tcl` et al.).",
    },
    SpecialVarSpec {
        name: "tcl_pkgPath",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "List of directories where binary packages are installed.",
    },
    SpecialVarSpec {
        name: "tcl_platform",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Platform,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        keys: TCL_PLATFORM_KEYS,
        externally_read: false,
        cmp_unsafe: true,
        summary: "Array of platform / build information. On iRules, reports BIG-IP \
                  values and is CMP-safe only via `static::tcl_platform`.",
    },
    SpecialVarSpec {
        name: "tcl_precision",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Number of significant digits used when converting floats to strings.",
    },
    SpecialVarSpec {
        name: "tcl_rcFileName",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Path of the user-specific startup script sourced by an interactive shell.",
    },
    SpecialVarSpec {
        name: "tcl_interactive",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "True when the interpreter is running interactively (a REPL).",
    },
    SpecialVarSpec {
        name: "tcl_wordchars",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Regexp matching word characters, used by `tcl_wordBreakAfter` and friends.",
    },
    SpecialVarSpec {
        name: "tcl_nonwordchars",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Regexp matching non-word characters, used by the word-break helpers.",
    },
    SpecialVarSpec {
        name: "tcl_traceCompile",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Bytecode-compiler trace level (debug builds).",
    },
    SpecialVarSpec {
        name: "tcl_traceExec",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Bytecode-execution trace level (debug builds).",
    },
    SpecialVarSpec {
        name: "tcl_prompt1",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Script evaluated to print the primary interactive prompt.",
    },
    SpecialVarSpec {
        name: "tcl_prompt2",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "Script evaluated to print the continuation prompt for an incomplete command.",
    },
    // ── iRules-specific ────────────────────────────────────────────────────
    SpecialVarSpec {
        name: "static",
        kind: SpecialVarKind::Namespace,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Dialect,
        dialects: DialectSet::IRULES,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        summary: "iRules CMP-safe global namespace. Values under `static::` are shared \
                  read-only across TMMs without demoting the virtual server from CMP.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_path_is_externally_read_in_tcl() {
        // The issue-#831 case: `set auto_path …` writes a runtime-observed
        // variable, so it must not be a dead store.
        assert!(is_externally_read("auto_path", "tcl8.6"));
        assert!(is_externally_read("auto_path", "")); // generic Tcl
        assert!(is_special_var("auto_path", "tcl8.6"));
    }

    #[test]
    fn plain_user_var_is_not_special() {
        assert!(!is_special_var("myVar", "tcl8.6"));
        assert!(!is_externally_read("myVar", "tcl8.6"));
        assert!(special_var("myVar").is_none());
    }

    #[test]
    fn irules_has_static_but_not_argv_or_env() {
        assert!(is_special_var("static", "f5-irules"));
        assert!(!is_special_var("static", "tcl8.6"));
        // Command-line / environment globals do not exist in the embedded
        // iRules interpreter.
        assert!(!is_special_var("argv", "f5-irules"));
        assert!(!is_special_var("env", "f5-irules"));
        assert!(!is_special_var("auto_path", "f5-irules"));
    }

    #[test]
    fn tcl_platform_available_in_both_tcl_and_irules() {
        assert!(is_special_var("tcl_platform", "tcl9.0"));
        assert!(is_special_var("tcl_platform", "f5-irules"));
        // Read-only platform info: a bare read is fine, but it is not the
        // "write is runtime-observed" class, so writes may still be flagged.
        assert!(!is_externally_read("tcl_platform", "tcl9.0"));
        // CMP demotion flag is iRules-relevant.
        assert!(special_var("tcl_platform").unwrap().cmp_unsafe);
    }

    #[test]
    fn tcl_platform_keys_are_dialect_versioned() {
        let spec = special_var("tcl_platform").unwrap();
        // `tmmVersion` is iRules-only.
        let irules_keys: Vec<_> = spec
            .keys_in(resolve_dialect("f5-irules"))
            .map(|k| k.key)
            .collect();
        assert!(irules_keys.contains(&"tmmVersion"));
        assert!(irules_keys.contains(&"os"));
        assert!(!irules_keys.contains(&"pointerSize")); // Tcl-only key

        // `pointerSize` is Tcl 8.6+, absent in 8.4.
        let keys_86: Vec<_> = spec
            .keys_in(resolve_dialect("tcl8.6"))
            .map(|k| k.key)
            .collect();
        let keys_84: Vec<_> = spec
            .keys_in(resolve_dialect("tcl8.4"))
            .map(|k| k.key)
            .collect();
        assert!(keys_86.contains(&"pointerSize"));
        assert!(!keys_84.contains(&"pointerSize"));
        assert!(!keys_86.contains(&"tmmVersion")); // iRules-only key absent in Tcl
    }

    #[test]
    fn read_only_info_vars_are_not_dead_store_suppressed_by_external_read() {
        // These are readable interpreter values; writing then never reading
        // them is genuinely suspicious, so `externally_read` is false.
        for name in ["tcl_version", "tcl_patchLevel"] {
            assert!(is_special_var(name, "tcl8.6"));
            assert!(!is_externally_read(name, "tcl8.6"));
        }
    }

    #[test]
    fn every_spec_name_is_unique() {
        let mut names: Vec<&str> = SPECIAL_VARS.iter().map(|v| v.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len, "duplicate special-variable name in table");
    }

    #[test]
    fn dialect_iteration_excludes_out_of_dialect_vars() {
        let irules: Vec<&str> = special_vars_for_dialect("f5-irules")
            .map(|v| v.name)
            .collect();
        assert!(irules.contains(&"static"));
        assert!(irules.contains(&"tcl_platform"));
        assert!(!irules.contains(&"argv"));
    }
}
