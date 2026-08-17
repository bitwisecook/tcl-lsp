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
//! * Some entries are **readable at startup** — either because the default
//!   host / interpreter / `init.tcl` has already bound them, or because a core
//!   read trace materialises their value.  That lifecycle fact, rather than
//!   special-variable availability alone, suppresses W210 for the initial
//!   top-level read.
//!
//! Beyond existence, each spec records the *effects* a read or write carries so
//! the side-effect and taint analyses can consult one table instead of
//! hardcoding names: [`SpecialVarSpec::write_effect`] names the interpreter
//! state a write mutates (the auto-loader path, float precision, the process
//! environment, …), and [`SpecialVarSpec::read_taint`] marks the
//! attacker-influenced external-input variables (`env`, `argv`, `argv0`) as
//! taint sources so a flow like `exec $env(CMD)` is seen as tainted.
//!
//! The set is **dialect-versioned**: it is a [`DialectSet`] membership table
//! exactly like [`crate::spec::CommandSpec`]. Standard Tcl, F5 iRules, and (in
//! future) Tk / EDA shells each provide a slightly different set — iRules has
//! no `argv`/`env`/`auto_path`, keeps its own BIG-IP `tcl_platform` keys, and
//! adds the CMP-safe [`static::`](https://clouddocs.f5.com/api/irules/static.html)
//! namespace. Consumers resolve the active dialect once and query this table;
//! no consumer hardcodes a name list.

use crate::dialects::DialectSet;
use crate::side_effects::SideEffectTarget;
use crate::taint::TaintColour;

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

/// The lifecycle event that makes a special variable readable before user
/// code.  Availability is deliberately separate: a recognised special
/// variable may instead be created lazily by a library command, on error, or
/// by user configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupBinding {
    /// No startup guarantee.  The variable may still be a meaningful runtime
    /// control, but a direct read must satisfy ordinary Tcl scoping rules.
    None,
    /// Bound by the standard interpreter / platform initialisation.
    Interpreter,
    /// Bound by a successful standard-library `Tcl_Init` / `init.tcl` run.
    TclInit,
    /// Bound by the default `Tcl_Main` command-line host.
    TclMain,
    /// Bound by the standard `tclsh` application initialisation.
    AppInit,
    /// A core read trace materialises the value on the first direct read.
    ReadTrace,
}

/// One known key of an array-shaped special variable, with the dialects that
/// provide it. Arrays with open, data-dependent keys (`env`, `auto_index`)
/// carry an empty key list.
#[derive(Debug, Clone, Copy)]
pub struct SpecialVarKey {
    /// The array index (`os`, `wordSize`, `tmmVersion`).
    pub key: &'static str,
    /// Dialects in which this key is present. `tcl_platform(tmmVersion)` is
    /// iRules-only; `tcl_platform(pointerSize)` is Tcl 8.5+.
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
    /// Dialects in which the default host has bound this global before user
    /// code runs.  This is deliberately independent of [`Self::dialects`]:
    /// a runtime-sensitive variable such as `errorInfo` or `auto_execs` can
    /// exist as a recognised special-variable surface without having an
    /// initial value.
    ///
    /// These are startup-*global* facts only.  They do not make a same-named
    /// procedure local defined, and a later `unset` removes the binding.
    pub initially_bound: DialectSet,
    /// Dialects where an otherwise-unbound direct read is defined by a core
    /// read trace.  Tcl 8.x's `tcl_precision` is the current example: it is
    /// not materialised in a fresh globals snapshot, but `$tcl_precision`
    /// reads as `0`.  A read trace is semantically equivalent to an entry
    /// binding for W210, but must remain distinct for lifecycle auditing.
    pub lazily_readable: DialectSet,
    /// Lifecycle event behind [`Self::initially_bound`] or
    /// [`Self::lazily_readable`].  It documents why W210 can treat only the
    /// initial global version as defined.
    pub startup_binding: StartupBinding,
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
    /// The extra interpreter/runtime state a *write* to this variable mutates,
    /// beyond the plain variable slot — the auto-loader search path
    /// (`auto_path`), the float-formatting precision (`tcl_precision`), the
    /// process environment (`env`), … `None` for read-only info globals and
    /// pure-data globals (`argv`) whose write carries no side effect. Lets the
    /// side-effect / optimiser passes treat `set auto_path …` as an
    /// interpreter-state mutation rather than an ordinary assignment.
    pub write_effect: Option<SideEffectTarget>,
    /// The taint colour a *read* of this variable produces when it is the
    /// interpreter-provided global (not shadowed by a local of the same name).
    /// `Some(TaintColour::TAINTED)` for attacker-influenced external input —
    /// the process environment (`env`) and the command line (`argv`, `argv0`) —
    /// so a flow like `exec $env(CMD)` is seen as tainted. `None` otherwise.
    pub read_taint: Option<TaintColour>,
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

    /// Whether a read from the initial, default global scope is defined in
    /// `dialect`.  This combines an eager startup binding with any documented
    /// lazy read trace; callers must still account for scope and later writes.
    #[must_use]
    pub fn readable_at_startup_in(&self, dialect: DialectSet) -> bool {
        self.initially_bound.intersects(dialect) || self.lazily_readable.intersects(dialect)
    }

    /// The known keys of this array that are present in `dialect`.
    pub fn keys_in(&self, dialect: DialectSet) -> impl Iterator<Item = &SpecialVarKey> {
        self.keys
            .iter()
            .filter(move |k| k.dialects.intersects(dialect))
    }
}

/// Resolve a dialect *name* to the [`DialectSet`] flag used for membership
/// tests — the profile's availability mask
/// ([`tcl_dialect::DialectProfile::availability_mask`]).
///
/// This was the availability-mask widening precedent the compositional
/// profile generalises (design doc §2.1): special variables and commands now
/// share ONE per-dialect mask instead of two hand-kept tables. Consequences
/// worth naming:
///
/// * An unrecognised name — empty, generic (`"tcl"`), or config-only
///   (`"f5-bigip"` / `"f5-tmsh"`) — resolves to every standard Tcl version,
///   so plain-Tcl analysis sees the full standard set (the `PLAIN_TCL` sink).
/// * **iRules** stays restricted to its bare bit: the TMM interpreter does
///   not provide the command-line / auto-loader / environment globals, so
///   `env` / `argv` / `auto_path` are *not* recognised there.
/// * **iApps** now composes `TCL85|IAPPS`: an iApp runs a real Tcl 8.5.13
///   *host* interpreter (not the TMM sandbox), so the standard interpreter
///   globals resolve there — the old bare-`IAPPS` view wrongly hid them.
/// * A specific Tcl version (`tcl8.6`) keeps its exact bit, so per-key
///   version gating (`tcl_platform(pointerSize)` is 8.5+) stays precise.
/// * A vendor shell (Expect, the EDA tools) composes its documented embedded
///   Tcl base with its own bit (`TCL86|EXPECT`, …) — the standard globals
///   resolve, but keys introduced *after* the embedded base version no
///   longer leak in.
/// * `tk` has no profile (it is a library over a Tcl base, not a dialect —
///   design doc §7.2); its parseable bit keeps the historical
///   superset-widening behaviour until the versioned-library axis models it.
#[must_use]
pub fn resolve_dialect(dialect: &str) -> DialectSet {
    if let Some(profile) = tcl_dialect::DialectProfile::find(dialect) {
        return profile.availability_mask;
    }
    // Parseable but not a catalog profile — today exactly `tk`: a Tcl
    // superset that provides the standard globals on top of its own.
    DialectSet::parse(dialect).map_or(DialectSet::ALL_TCL, |bit| bit | DialectSet::ALL_TCL)
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
/// This is an availability / metadata query.  It is intentionally *not* a
/// W210 predicate: a number of special variables are created only after a
/// runtime event or library call.
#[must_use]
pub fn is_special_var(name: &str, dialect: &str) -> bool {
    special_var_in_dialect(name, dialect).is_some()
}

/// Whether a bare global `name` is readable before user code has written it
/// in the default startup context for `dialect`.
///
/// Consumers must apply this only to an initial global SSA version.  A
/// procedure-local variable of the same name, or a version killed by `unset`,
/// remains a genuine read-before-set.
#[must_use]
pub fn is_readable_at_startup(name: &str, dialect: &str) -> bool {
    let resolved = resolve_dialect(dialect);
    special_var(name).is_some_and(|v| v.readable_at_startup_in(resolved))
}

/// Whether reading `name` in `dialect` invokes a registry-declared Tcl read
/// trace which can materialise the value again after `unset`.  Unlike
/// [`is_readable_at_startup`], this intentionally excludes eager startup
/// bindings such as `argv`: deleting those leaves an ordinary undefined Tcl
/// variable.
#[must_use]
pub fn is_lazily_readable(name: &str, dialect: &str) -> bool {
    let resolved = resolve_dialect(dialect);
    special_var(name).is_some_and(|v| v.lazily_readable.intersects(resolved))
}

/// Whether a *write* to `name` in `dialect` is observed by the runtime — so
/// `set NAME …` must not be flagged as a dead store (W220) or unused variable
/// (W211) even when the script never reads `$NAME`. This is the fix for the
/// `set auto_path …` false positive (issue #831).
#[must_use]
pub fn is_externally_read(name: &str, dialect: &str) -> bool {
    special_var_in_dialect(name, dialect).is_some_and(|v| v.externally_read)
}

/// The extra interpreter/runtime state a *write* to special variable `name`
/// mutates in `dialect`, or `None` if `name` is not a writable-with-effect
/// special variable there. Lets the side-effect analysis treat
/// `set auto_path …` as an [`SideEffectTarget::InterpState`] mutation.
#[must_use]
pub fn special_var_write_effect(name: &str, dialect: &str) -> Option<SideEffectTarget> {
    special_var_in_dialect(name, dialect).and_then(|v| v.write_effect)
}

/// The taint colour a *read* of special variable `name` produces in `dialect`,
/// or `None` if reading it is not a taint source there. `env` / `argv` /
/// `argv0` are attacker-influenced external input.
#[must_use]
pub fn special_var_read_taint(name: &str, dialect: &str) -> Option<TaintColour> {
    special_var_in_dialect(name, dialect).and_then(|v| v.read_taint)
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
        dialects: DialectSet::TCL85_PLUS,
        summary: "Size in bytes of a native pointer (Tcl 8.5+).",
    },
    SpecialVarKey {
        key: "user",
        dialects: DialectSet::ALL_TCL,
        summary: "Login name of the user running the process.",
    },
    SpecialVarKey {
        key: "engine",
        dialects: DialectSet::TCL85_PLUS,
        summary: "Interpreter engine name — `Tcl` for the reference implementation (8.5+).",
    },
    SpecialVarKey {
        key: "pathSeparator",
        dialects: DialectSet::TCL86_PLUS,
        summary: "Separator between paths in a search-path list (Tcl 8.6+).",
    },
    SpecialVarKey {
        key: "threaded",
        // Build conditional (`TCL_THREADS`), so no release-only profile can
        // promise this key to ordinary user code.
        dialects: DialectSet::empty(),
        summary: "Present only on Tcl 8.x builds configured with thread support.",
    },
    SpecialVarKey {
        key: "debug",
        // Windows debug-build conditional, likewise not a release fact.
        dialects: DialectSet::empty(),
        summary: "Present only on Tcl 8.x Windows debug builds.",
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
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::TclMain,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: None,
        read_taint: None,
        summary: "Number of command-line arguments in `argv`.",
    },
    SpecialVarSpec {
        name: "argv",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::TclMain,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: None,
        read_taint: Some(TaintColour::TAINTED),
        summary: "List of command-line arguments passed after the script name.",
    },
    SpecialVarSpec {
        name: "argv0",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::TclMain,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: None,
        read_taint: Some(TaintColour::TAINTED),
        summary: "Name of the script (or interpreter) being executed.",
    },
    // ── Auto-loader / package machinery (init.tcl) ─────────────────────────
    SpecialVarSpec {
        name: "auto_path",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::TclInit,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Search path (a list of directories) the package/auto-loader \
                  consults at runtime. Usually configured, not read, by scripts.",
    },
    SpecialVarSpec {
        name: "auto_index",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Maps an auto-loadable command name to the script that defines it.",
    },
    SpecialVarSpec {
        name: "auto_execs",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Cache mapping external command names to their resolved executable paths.",
    },
    SpecialVarSpec {
        name: "auto_noexec",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "When set, disables auto-execution of external commands from the shell.",
    },
    SpecialVarSpec {
        name: "auto_noload",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::AutoLoader,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "When set, disables the auto-loader (`unknown` will not auto-load).",
    },
    // ── Environment ────────────────────────────────────────────────────────
    SpecialVarSpec {
        name: "env",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Environment,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::Interpreter,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: Some(TaintColour::TAINTED),
        summary: "The process environment. Writes propagate to child processes; \
                  keys are environment-variable names.",
    },
    // ── Error context ──────────────────────────────────────────────────────
    // Tcl 8.4's default `init.tcl` unconditionally initialises these after a
    // successful Tcl_Init. Later releases intentionally leave them absent
    // until an error occurs, and plain Tcl_CreateInterp does not qualify.
    SpecialVarSpec {
        name: "errorInfo",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        initially_bound: DialectSet::TCL84,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::TclInit,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Human-readable stack trace of the most recent error.",
    },
    SpecialVarSpec {
        name: "errorCode",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        initially_bound: DialectSet::TCL84,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::TclInit,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Machine-readable error code (a list) of the most recent error.",
    },
    // ── Interpreter / library description ──────────────────────────────────
    SpecialVarSpec {
        name: "tcl_version",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        initially_bound: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::Interpreter,
        keys: &[],
        externally_read: false,
        cmp_unsafe: true,
        write_effect: None,
        read_taint: None,
        summary: "Interpreter version as `major.minor` (e.g. `8.6`).",
    },
    SpecialVarSpec {
        name: "tcl_patchLevel",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        initially_bound: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::Interpreter,
        keys: &[],
        externally_read: false,
        cmp_unsafe: true,
        write_effect: None,
        read_taint: None,
        summary: "Full patch level of the interpreter (e.g. `8.6.13`).",
    },
    SpecialVarSpec {
        name: "tcl_library",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::TclInit,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Directory holding the standard Tcl script library (`init.tcl` et al.).",
    },
    SpecialVarSpec {
        name: "tcl_pkgPath",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::Interpreter,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "List of directories where binary packages are installed.",
    },
    SpecialVarSpec {
        name: "tcl_platform",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Platform,
        dialects: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        initially_bound: DialectSet::ALL_TCL.union(DialectSet::IRULES),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::Interpreter,
        keys: TCL_PLATFORM_KEYS,
        externally_read: false,
        cmp_unsafe: true,
        write_effect: None,
        read_taint: None,
        summary: "Array of platform / build information. On iRules, reports BIG-IP \
                  values and is CMP-safe only via `static::tcl_platform`.",
    },
    SpecialVarSpec {
        name: "tcl_precision",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::TCL8X,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::TCL8X,
        startup_binding: StartupBinding::ReadTrace,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Number of significant digits used when converting floats to strings.",
    },
    SpecialVarSpec {
        name: "tcl_rcFileName",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::AppInit,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Path of the user-specific startup script sourced by an interactive shell.",
    },
    SpecialVarSpec {
        name: "tcl_interactive",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::ALL_TCL,
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::TclMain,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "True when the interpreter is running interactively (a REPL).",
    },
    SpecialVarSpec {
        name: "tcl_wordchars",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Regexp matching word characters, used by `tcl_wordBreakAfter` and friends.",
    },
    SpecialVarSpec {
        name: "tcl_nonwordchars",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Regexp matching non-word characters, used by the word-break helpers.",
    },
    SpecialVarSpec {
        name: "tcl_traceCompile",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Bytecode-compiler trace level (debug builds).",
    },
    SpecialVarSpec {
        name: "tcl_traceExec",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Bytecode-execution trace level (debug builds).",
    },
    SpecialVarSpec {
        name: "tcl_prompt1",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Script evaluated to print the primary interactive prompt.",
    },
    SpecialVarSpec {
        name: "tcl_prompt2",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        dialects: DialectSet::ALL_TCL,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Script evaluated to print the continuation prompt for an incomplete command.",
    },
    // ── iRules-specific ────────────────────────────────────────────────────
    SpecialVarSpec {
        name: "static",
        kind: SpecialVarKind::Namespace,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Dialect,
        dialects: DialectSet::IRULES,
        initially_bound: DialectSet::empty(),
        lazily_readable: DialectSet::empty(),
        startup_binding: StartupBinding::None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: None,
        read_taint: None,
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
    fn startup_readability_is_lifecycle_not_special_var_availability() {
        // This fixture is the audited default-host contract for an ordinary
        // Tcl script: it catches both an omitted startup global and an
        // accidental promotion of a lazy/configuration-only special var.
        // In particular, 8.4's successful default Tcl_Init runs init.tcl's
        // unconditional `set errorCode ""` / `set errorInfo ""`; 8.5+ does
        // not seed either name until error handling creates it.
        let common = [
            "argc",
            "argv",
            "argv0",
            "auto_path",
            "env",
            "tcl_interactive",
            "tcl_library",
            "tcl_patchLevel",
            "tcl_pkgPath",
            "tcl_platform",
            "tcl_rcFileName",
            "tcl_version",
        ];
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut expected = common.to_vec();
            if dialect == "tcl8.4" {
                expected.extend(["errorCode", "errorInfo"]);
            }
            if matches!(dialect, "tcl8.4" | "tcl8.5" | "tcl8.6") {
                expected.push("tcl_precision");
            }
            expected.sort_unstable();

            let mut actual: Vec<_> = SPECIAL_VARS
                .iter()
                .filter(|spec| spec.readable_at_startup_in(resolve_dialect(dialect)))
                .map(|spec| spec.name)
                .collect();
            actual.sort_unstable();
            assert_eq!(actual, expected, "startup-readable vars for {dialect}");
        }

        // The embedded iRules runtime has a deliberately smaller proven
        // startup surface.  In particular, namespace availability for
        // `static::` does not promise a value for arbitrary static variables.
        let mut irules: Vec<_> = SPECIAL_VARS
            .iter()
            .filter(|spec| spec.readable_at_startup_in(resolve_dialect("f5-irules")))
            .map(|spec| spec.name)
            .collect();
        irules.sort_unstable();
        assert_eq!(irules, ["tcl_patchLevel", "tcl_platform", "tcl_version"]);

        assert_eq!(
            special_var("argv").unwrap().startup_binding,
            StartupBinding::TclMain
        );
        assert_eq!(
            special_var("tcl_precision").unwrap().startup_binding,
            StartupBinding::ReadTrace
        );
        assert!(is_readable_at_startup("tcl_precision", "tcl8.6"));
        assert!(!is_readable_at_startup("tcl_precision", "tcl9.0"));
        assert!(is_lazily_readable("tcl_precision", "tcl8.6"));
        assert!(!is_lazily_readable("tcl_precision", "tcl9.0"));
        assert!(!is_lazily_readable("argv", "tcl8.6"));
        // iApps uses its host Tcl interpreter profile, whereas the embedded
        // iRules runtime has only the explicitly evidenced metadata globals.
        assert!(is_readable_at_startup("argv", "f5-iapps"));
        assert!(!is_readable_at_startup("argv", "f5-irules"));
        // `auto_index` can materialise after interactive activity or an
        // auto-loader operation, but an ordinary default-host script begins
        // with no array and a direct `$auto_index` / `$auto_index(key)` read
        // still errors. The other names below are likewise special metadata,
        // not default startup bindings.
        for name in [
            "auto_index",
            "auto_execs",
            "auto_noexec",
            "auto_noload",
            "static",
        ] {
            assert!(!is_readable_at_startup(name, "tcl8.6"), "{name}");
        }
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

        // `pointerSize` arrived in Tcl 8.5, while `pathSeparator` arrived in
        // Tcl 8.6; build-only keys are deliberately not advertised by a
        // release-only profile.
        let keys_85: Vec<_> = spec
            .keys_in(resolve_dialect("tcl8.5"))
            .map(|k| k.key)
            .collect();
        let keys_86: Vec<_> = spec
            .keys_in(resolve_dialect("tcl8.6"))
            .map(|k| k.key)
            .collect();
        let keys_84: Vec<_> = spec
            .keys_in(resolve_dialect("tcl8.4"))
            .map(|k| k.key)
            .collect();
        assert!(keys_85.contains(&"pointerSize"));
        assert!(keys_86.contains(&"pointerSize"));
        assert!(!keys_84.contains(&"pointerSize"));
        assert!(!keys_85.contains(&"pathSeparator"));
        assert!(keys_86.contains(&"pathSeparator"));
        assert!(!keys_86.contains(&"threaded"));
        assert!(!keys_86.contains(&"debug"));
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

    #[test]
    fn write_effects_are_recorded_for_interpreter_state_writes() {
        // Writing these mutates interpreter/runtime state, not just a variable.
        for name in ["auto_path", "tcl_precision", "env", "tcl_library"] {
            assert_eq!(
                special_var_write_effect(name, "tcl8.6"),
                Some(SideEffectTarget::InterpState),
                "{name} should carry an InterpState write effect",
            );
        }
        // Read-only info globals and pure-data globals carry no write effect.
        for name in ["tcl_version", "tcl_platform", "argv", "argc"] {
            assert_eq!(special_var_write_effect(name, "tcl8.6"), None, "{name}");
        }
    }

    #[test]
    fn read_taint_marks_external_input_variables() {
        for name in ["env", "argv", "argv0"] {
            assert_eq!(
                special_var_read_taint(name, "tcl8.6"),
                Some(TaintColour::TAINTED),
                "{name} is attacker-influenced external input",
            );
        }
        // Interpreter-provided info is not attacker-controlled.
        for name in ["tcl_version", "tcl_platform", "auto_path"] {
            assert_eq!(special_var_read_taint(name, "tcl8.6"), None, "{name}");
        }
    }

    #[test]
    fn tcl_derived_dialects_keep_the_standard_globals() {
        // Regression for the dialect-resolution fix: Tk / Expect / EDA shells
        // are Tcl supersets and must still recognise the standard interpreter
        // globals (they did before the registry existed).
        for dialect in ["tk", "expect", "synopsys-eda-tcl", "cadence-eda-tcl"] {
            assert!(
                is_special_var("env", dialect),
                "env must be a special var in {dialect}",
            );
            assert!(is_special_var("auto_path", dialect), "{dialect}");
            assert!(is_special_var("argv", dialect), "{dialect}");
            // The write-effect / taint queries resolve there too.
            assert_eq!(
                special_var_read_taint("env", dialect),
                Some(TaintColour::TAINTED),
                "{dialect}",
            );
        }
        // The restricted TMM sandbox (iRules) still does NOT get them.
        assert!(!is_special_var("env", "f5-irules"));
        assert!(!is_special_var("argv", "f5-irules"));
        assert!(!is_special_var("auto_path", "f5-irules"));
        // iApps are NOT the TMM sandbox: they run a real Tcl 8.5.13 *host*
        // interpreter (dialect-profile-model.md §7, D3-adjacent ratification),
        // so the standard interpreter globals resolve there like any other
        // Tcl superset. The old bare-`IAPPS` view wrongly hid them.
        assert!(is_special_var("env", "f5-iapps"));
        assert!(is_special_var("auto_path", "f5-iapps"));
    }

    #[test]
    fn version_dialects_keep_exact_bit_for_per_key_gating() {
        // A specific Tcl version must not be widened to ALL_TCL, or per-key
        // version gating (pointerSize is 8.5+) would leak into 8.4.
        assert_eq!(resolve_dialect("tcl8.4"), DialectSet::TCL84);
        let spec = special_var("tcl_platform").unwrap();
        let keys_84: Vec<_> = spec
            .keys_in(resolve_dialect("tcl8.4"))
            .map(|k| k.key)
            .collect();
        assert!(!keys_84.contains(&"pointerSize"));
    }
}
