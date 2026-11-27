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
//!   Such a write is *not* a dead store / unused variable.
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
//! The set is **dialect-versioned**: it is a `SpecSurface` membership table
//! exactly like [`crate::spec::CommandSpec`]. Standard Tcl, F5 iRules, and (in
//! future) Tk / EDA shells each provide a slightly different set — iRules has
//! no `argv`/`env`/`auto_path`, keeps its own BIG-IP `tcl_platform` keys, and
//! adds the CMP-safe [`static::`](https://clouddocs.f5.com/api/irules/static.html)
//! namespace. Consumers resolve the active dialect once and query this table;
//! no consumer hardcodes a name list.

use crate::side_effects::SideEffectTarget;
use crate::taint::TaintColour;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::{CorePoints, Family, SpecProvider, SurfaceQuery, surface_admits};

/// Startup binding and read-trace facts for a special variable.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StartupExistence {
    /// The interpreter binds the variable before the script runs.
    pub initially_bound: bool,
    /// A read trace materialises the variable, including after an unset.
    pub lazily_readable: bool,
}

impl StartupExistence {
    /// Whether an initial global read can succeed before any script writes.
    #[must_use]
    pub const fn readable(self) -> bool {
        self.initially_bound || self.lazily_readable
    }
}

/// Native interpreter storage written while presenting an error completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeErrorStorageVariable {
    /// Structured native error classification.
    ErrorCode,
    /// Native error context and stack presentation.
    ErrorInfo,
}

impl NativeErrorStorageVariable {
    /// Global lookup spelling used by the native interpreter, including aliases.
    #[must_use]
    pub const fn global_name(self) -> &'static str {
        match self {
            Self::ErrorCode => "::errorCode",
            Self::ErrorInfo => "::errorInfo",
        }
    }

    /// The variable-table key in the root namespace.
    #[must_use]
    pub fn simple_name(self) -> &'static str {
        tcl_syntax::naming::unroot_rooted_key(self.global_name()).expect("rooted error storage key")
    }
}

/// C Tcl error presentation can write both global cells, independently of the
/// physical frame where an invocation or compilation failure occurred. A
/// source fork of the C core (the F5 trunk and its iRules offshoot) keeps
/// that presentation. A reimplementation retains an unresolved footprint.
#[must_use]
pub fn native_error_storage(
    dialect: tcl_native_model::InvocationDialect,
) -> Option<&'static [NativeErrorStorageVariable]> {
    dialect.family().is_some_and(c_core_lineage).then_some(&[
        NativeErrorStorageVariable::ErrorCode,
        NativeErrorStorageVariable::ErrorInfo,
    ])
}

fn c_core_lineage(family: tcl_dialect::model::Family) -> bool {
    match family.ancestry() {
        None => family == tcl_dialect::model::Family::Tcl,
        Some(ancestry) => {
            ancestry.lineage == tcl_dialect::model::family::Lineage::Fork
                && c_core_lineage(ancestry.parent)
        }
    }
}

/// Audited C hidden error-variable callbacks installed by interpreter creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeErrorVariableProtocol;

/// Action of the hidden read callback, after user callbacks have run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeErrorVariableRead {
    /// Preserve the current public cell and private state.
    Preserve,
    /// Store the same retained private object in the public cell.
    CopyPrivate,
    /// Define an empty public value without creating a private object.
    DefineEmpty,
}

impl NativeErrorVariableProtocol {
    /// Read callbacks are gated by `ERR_LEGACY_COPY`, independently of contents.
    #[must_use]
    pub const fn read(
        self,
        legacy_copy: bool,
        private: bool,
        defined: bool,
    ) -> NativeErrorVariableRead {
        if !legacy_copy {
            NativeErrorVariableRead::Preserve
        } else if private {
            NativeErrorVariableRead::CopyPrivate
        } else if !defined {
            NativeErrorVariableRead::DefineEmpty
        } else {
            NativeErrorVariableRead::Preserve
        }
    }

    /// `Tcl_ResetResult` publishes code before info, then releases private owners.
    #[must_use]
    pub const fn reset_order(self) -> &'static [NativeErrorStorageVariable] {
        &[
            NativeErrorStorageVariable::ErrorCode,
            NativeErrorStorageVariable::ErrorInfo,
        ]
    }
}

/// [`InvocationDialect`](tcl_native_model::InvocationDialect) selectors for this module's types. The types
/// live in the registry core, so the selectors are an extension trait.
pub trait ErrorVariableDialect {
    /// Actual C8.5+ callbacks; C8.4, Jim and authored vendor simulations abstain.
    #[must_use]
    fn native_error_variable_protocol(self) -> Option<NativeErrorVariableProtocol>;
}

impl ErrorVariableDialect for tcl_native_model::InvocationDialect {
    fn native_error_variable_protocol(self) -> Option<NativeErrorVariableProtocol> {
        match self.native_name_protocol()? {
            tcl_syntax::naming::NativeNameProtocol::C(version)
                if version >= tcl_dialect::TclVersion::V8_5 =>
            {
                Some(NativeErrorVariableProtocol)
            }
            _ => None,
        }
    }
}

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
    /// Bound by the default command-line shell before it runs a script.
    TclMain,
    /// Bound by the shell's application or ambient package initialisation.
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
    pub surface: &'static [SpecSurface],
    /// One-line description for hover.
    pub summary: &'static str,
}

/// Native callbacks attached to the actual interpreter-provided storage cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialVariableHook {
    /// Thread-shared floating string precision; reads canonicalise the raw slot.
    DoublePrecision,
}

impl SpecialVariableHook {
    /// Native global name selected independently of catalogue availability.
    #[must_use]
    pub fn name_in(self, dialect: tcl_native_model::InvocationDialect) -> Option<&'static str> {
        match self {
            Self::DoublePrecision => dialect.double_string_policy()?.precision_variable(),
        }
    }

    /// A raw stored value alone cannot prove the next observed read result.
    #[must_use]
    pub const fn changes_read_value(self) -> bool {
        match self {
            Self::DoublePrecision => true,
        }
    }
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
    pub surface: &'static [SpecSurface],
    /// Dialects in which the default host has bound this global before user
    /// code runs.  This is deliberately independent of [`Self::dialects`]:
    /// a runtime-sensitive variable such as `errorInfo` or `auto_execs` can
    /// exist as a recognised special-variable surface without having an
    /// initial value.
    ///
    /// These are startup-*global* facts only.  They do not make a same-named
    /// procedure local defined, and a later `unset` removes the binding.
    pub initially_bound: &'static [SpecSurface],
    /// Dialects where an otherwise-unbound direct read is defined by a core
    /// read trace.  Tcl 8.x's `tcl_precision` is the current example: it is
    /// not materialised in a fresh globals snapshot, but `$tcl_precision`
    /// reads as `0`.  A read trace is semantically equivalent to an entry
    /// binding for W210, but must remain distinct for lifecycle auditing.
    pub lazily_readable: &'static [SpecSurface],
    /// Lifecycle event behind [`Self::initially_bound`] or
    /// [`Self::lazily_readable`].  It documents why W210 can treat only the
    /// initial global version as defined.
    pub startup_binding: StartupBinding,
    /// Native observer independent of script-level trace registrations.
    pub runtime_hook: Option<SpecialVariableHook>,
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
    /// `SpecSurface` flag for the active dialect).
    #[must_use]
    pub fn available_in(&self, dialect: Option<SurfaceQuery<'_>>) -> bool {
        let dialect = variable_surface(dialect);
        surface_admits(self.surface, dialect.as_ref())
    }

    /// Whether the row describes a core startup binding or read trace.
    /// The registry's startup query also accounts for ambient packages.
    #[must_use]
    pub fn readable_at_startup_in(&self, dialect: Option<SurfaceQuery<'_>>) -> bool {
        self.startup_existence(dialect, |_| false).readable()
    }

    pub(crate) fn startup_existence(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        ambient_package: impl Fn(&str) -> bool,
    ) -> StartupExistence {
        let Some(dialect) = variable_surface(dialect) else {
            return StartupExistence::default();
        };
        let provides = |surface: &SpecSurface| {
            surface.admits(&dialect)
                && match surface.provider {
                    SpecProvider::Core(_) => true,
                    SpecProvider::Package(package) => ambient_package(package),
                }
        };
        StartupExistence {
            initially_bound: self.initially_bound.iter().any(provides),
            lazily_readable: self.lazily_readable.iter().any(provides),
        }
    }

    /// The known keys of this array that are present in `dialect`.
    pub fn keys_in(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> impl Iterator<Item = &SpecialVarKey> {
        let dialect = variable_surface(dialect);
        self.keys
            .iter()
            .filter(move |k| surface_admits(k.surface, dialect.as_ref()))
    }
}

fn variable_surface(dialect: Option<SurfaceQuery<'_>>) -> Option<SurfaceQuery<'_>> {
    let mut dialect = dialect?;
    // Jim inherits command vocabulary from its Tcl anchor. Its interpreter
    // creates its own variables and does not run the C Tcl bootstrap.
    if let Some((Family::Jim, release)) = dialect.core.nearest() {
        dialect.core = CorePoints::one(Family::Jim, release);
    }
    Some(dialect)
}

/// The availability set the special-variable membership tests answer under for
/// an already-resolved profile — the profile's
/// [`surface_query`](tcl_dialect::DialectProfile::surface_query). The
/// LSP/CLI ingress resolves the dialect name once (through
/// `tcl_registry::model::ingress`) and threads the resolved profile.
///
/// `None` (no dialect resolved) answers the permissive `PLAIN_TCL`
/// profile's own point.
#[must_use]
pub fn surface_query_for_profile(
    profile: Option<&tcl_dialect::DialectProfile>,
) -> SurfaceQuery<'static> {
    profile
        .unwrap_or_else(|| tcl_dialect::DialectProfile::plain_tcl())
        .surface_query()
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

/// Look up a special variable that is available in `dialect` (the resolved
/// availability point, from [`surface_query_for_profile`]).
#[must_use]
pub fn special_var_in_dialect(
    name: &str,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<&'static SpecialVarSpec> {
    special_var(name).filter(|v| v.available_in(dialect))
}

/// Whether `name` is an interpreter-provided special variable in `dialect`.
///
/// This is an availability / metadata query.  It is intentionally *not* a
/// W210 predicate: a number of special variables are created only after a
/// runtime event or library call.
#[cfg(test)]
fn is_special_var(name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
    special_var_in_dialect(name, dialect).is_some()
}

#[cfg(test)]
fn is_readable_at_startup(name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
    special_var(name).is_some_and(|spec| spec.readable_at_startup_in(dialect))
}

#[cfg(test)]
fn is_initially_bound(name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
    special_var(name).is_some_and(|spec| spec.startup_existence(dialect, |_| false).initially_bound)
}

#[cfg(test)]
fn is_lazily_readable(name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
    special_var(name).is_some_and(|spec| spec.startup_existence(dialect, |_| false).lazily_readable)
}

#[cfg(test)]
fn is_externally_read(name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
    special_var_in_dialect(name, dialect).is_some_and(|spec| spec.externally_read)
}

/// The extra interpreter/runtime state a *write* to special variable `name`
/// mutates in `dialect`, or `None` if `name` is not a writable-with-effect
/// special variable there. Lets the side-effect analysis treat
/// `set auto_path …` as an [`SideEffectTarget::InterpState`] mutation.
#[must_use]
pub fn special_var_write_effect(
    name: &str,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<SideEffectTarget> {
    special_var_in_dialect(name, dialect).and_then(|v| v.write_effect)
}

/// The taint colour a *read* of special variable `name` produces in `dialect`,
/// or `None` if reading it is not a taint source there. `env` / `argv` /
/// `argv0` are attacker-influenced external input.
#[must_use]
pub fn special_var_read_taint(
    name: &str,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<TaintColour> {
    special_var_in_dialect(name, dialect).and_then(|v| v.read_taint)
}

/// Iterate the special variables available in `dialect`, in table order.
pub fn special_vars_for_dialect(
    dialect: Option<SurfaceQuery<'_>>,
) -> impl Iterator<Item = &'static SpecialVarSpec> {
    SPECIAL_VARS.iter().filter(move |v| v.available_in(dialect))
}

const TCL_AND_JIM: &[SpecSurface] = &[SpecSurface::ALL_TCL[0], SpecSurface::JIM[0]];
const TCL_IRULES_AND_JIM: &[SpecSurface] = &[
    SpecSurface::ALL_TCL_AND_IRULES[0],
    SpecSurface::ALL_TCL_AND_IRULES[1],
    SpecSurface::JIM[0],
];
const TCL85_PLUS_AND_JIM: &[SpecSurface] = &[SpecSurface::TCL85_PLUS[0], SpecSurface::JIM[0]];
const TCL86_PLUS_AND_JIM: &[SpecSurface] = &[SpecSurface::TCL86_PLUS[0], SpecSurface::JIM[0]];

/// `tcl_platform` array keys. Standard Tcl and F5 iRules diverge here: iRules
/// reports BIG-IP values (`os` = `"BIG-IP"`) and adds `tmmVersion`, while
/// dropping the desktop-only keys.
const TCL_PLATFORM_KEYS: &[SpecialVarKey] = &[
    SpecialVarKey {
        key: "platform",
        surface: TCL_IRULES_AND_JIM,
        summary: "OS family: `unix`, `windows`, or (older) `macintosh`.",
    },
    SpecialVarKey {
        key: "os",
        surface: TCL_IRULES_AND_JIM,
        summary: "Operating-system name (`Linux`, `Windows NT`; `BIG-IP` on iRules).",
    },
    SpecialVarKey {
        key: "osVersion",
        surface: SpecSurface::ALL_TCL_AND_IRULES,
        summary: "Operating-system version (the BIG-IP version on iRules).",
    },
    SpecialVarKey {
        key: "byteOrder",
        surface: TCL_IRULES_AND_JIM,
        summary: "`littleEndian` or `bigEndian`.",
    },
    SpecialVarKey {
        key: "wordSize",
        surface: TCL_IRULES_AND_JIM,
        summary: "Size in bytes of the native word (4 or 8).",
    },
    SpecialVarKey {
        key: "machine",
        surface: SpecSurface::ALL_TCL_AND_IRULES,
        summary: "Hardware architecture (`x86_64`, `arm64`; hostname on iRules).",
    },
    SpecialVarKey {
        key: "pointerSize",
        surface: TCL85_PLUS_AND_JIM,
        summary: "Size in bytes of a native pointer.",
    },
    SpecialVarKey {
        key: "user",
        surface: SpecSurface::ALL_TCL,
        summary: "Login name of the user running the process.",
    },
    SpecialVarKey {
        key: "engine",
        surface: TCL85_PLUS_AND_JIM,
        summary: "Interpreter engine name, `Tcl` or `Jim`.",
    },
    SpecialVarKey {
        key: "pathSeparator",
        surface: TCL86_PLUS_AND_JIM,
        summary: "Separator between paths in a search-path list.",
    },
    SpecialVarKey {
        key: "threaded",
        // Build conditional (`TCL_THREADS`), so no release-only profile can
        // promise this key to ordinary user code.
        surface: SpecSurface::JIM,
        summary: "Thread support indicator. Tcl 8.x provides it only in threaded builds.",
    },
    SpecialVarKey {
        key: "debug",
        // Windows debug-build conditional, likewise not a release fact.
        surface: &[],
        summary: "Present only on Tcl 8.x Windows debug builds.",
    },
    SpecialVarKey {
        key: "tmmVersion",
        surface: SpecSurface::IRULES,
        summary: "iRules only: full TMM version including build number.",
    },
    SpecialVarKey {
        key: "bootstrap",
        surface: SpecSurface::JIM,
        summary: "Whether this is the bootstrap Jim interpreter.",
    },
    SpecialVarKey {
        key: "stackFormat",
        surface: SpecSurface::JIM,
        summary: "Number of fields in a Jim stack frame.",
    },
];

const JIM_SHELL: &[SpecSurface] = &[SpecSurface::core_in(Family::Jim, &[("0.84", None)])];

const fn host_variable(
    name: &'static str,
    kind: SpecialVarKind,
    surface: &'static [SpecSurface],
    startup_binding: StartupBinding,
    summary: &'static str,
) -> SpecialVarSpec {
    SpecialVarSpec {
        name,
        kind,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Dialect,
        surface,
        initially_bound: if matches!(startup_binding, StartupBinding::None) {
            &[]
        } else {
            surface
        },
        lazily_readable: &[],
        startup_binding,
        runtime_hook: None,
        keys: &[],
        externally_read: false,
        cmp_unsafe: false,
        write_effect: None,
        read_taint: None,
        summary,
    }
}

/// The complete special-variable table, dialect-versioned.
///
/// Ordering groups related variables (auto-loader, error context, platform)
/// for readability; lookups are by name so order is not semantically
/// significant.
pub const SPECIAL_VARS: &[SpecialVarSpec] = &[
    // ── Command-line arguments (Tcl and Jim) ─────────
    SpecialVarSpec {
        name: "argc",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Interpreter,
        surface: TCL_AND_JIM,
        initially_bound: TCL_AND_JIM,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclMain,
        runtime_hook: None,
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
        surface: TCL_AND_JIM,
        initially_bound: TCL_AND_JIM,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclMain,
        runtime_hook: None,
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
        surface: TCL_AND_JIM,
        initially_bound: TCL_AND_JIM,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclMain,
        runtime_hook: None,
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
        surface: TCL_AND_JIM,
        initially_bound: TCL_AND_JIM,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclInit,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: TCL_AND_JIM,
        initially_bound: TCL_AND_JIM,
        lazily_readable: &[],
        startup_binding: StartupBinding::Interpreter,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL_AND_IRULES,
        initially_bound: SpecSurface::TCL84,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclInit,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL_AND_IRULES,
        initially_bound: SpecSurface::TCL84,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclInit,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL_AND_IRULES,
        initially_bound: SpecSurface::ALL_TCL_AND_IRULES,
        lazily_readable: &[],
        startup_binding: StartupBinding::Interpreter,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL_AND_IRULES,
        initially_bound: SpecSurface::ALL_TCL_AND_IRULES,
        lazily_readable: &[],
        startup_binding: StartupBinding::Interpreter,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: SpecSurface::ALL_TCL,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclInit,
        runtime_hook: None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Directory holding the standard Tcl script library (`init.tcl` et al.).",
    },
    SpecialVarSpec {
        name: "tcl_libPath",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        // Tcl 8.4 only: `Tcl_Init` set it on both Unix and Windows
        // (`tclUnixInit.c` / `tclWinInit.c`).  Tcl 8.5 marks it "OBSOLETE:
        // This variable is no longer set by Tcl" (`tclInterp.c`) and no
        // later release restores it.
        surface: SpecSurface::TCL84,
        initially_bound: SpecSurface::TCL84,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclInit,
        runtime_hook: None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Tcl 8.4 only: the library search path `Tcl_Init` seeded before \
                  sourcing `init.tcl`. Removed in Tcl 8.5.",
    },
    SpecialVarSpec {
        name: "tcl_pkgPath",
        kind: SpecialVarKind::Scalar,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Interpreter,
        surface: SpecSurface::ALL_TCL,
        initially_bound: SpecSurface::ALL_TCL,
        lazily_readable: &[],
        startup_binding: StartupBinding::Interpreter,
        runtime_hook: None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "List of directories where binary packages are installed.",
    },
    SpecialVarSpec {
        origin: VarOrigin::Interpreter,
        ..host_variable(
            "tclDefaultLibrary",
            SpecialVarKind::Scalar,
            SpecSurface::TCL84,
            StartupBinding::None,
            "Tcl 8.4's platform-dependent library bootstrap path, consumed and unset by `init.tcl`.",
        )
    },
    SpecialVarSpec {
        name: "tcl_platform",
        kind: SpecialVarKind::Array,
        access: VarAccess::ReadOnly,
        origin: VarOrigin::Platform,
        surface: TCL_IRULES_AND_JIM,
        initially_bound: TCL_IRULES_AND_JIM,
        lazily_readable: &[],
        startup_binding: StartupBinding::Interpreter,
        runtime_hook: None,
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
        surface: SpecSurface::TCL8X,
        initially_bound: &[],
        lazily_readable: SpecSurface::TCL8X,
        startup_binding: StartupBinding::ReadTrace,
        runtime_hook: Some(SpecialVariableHook::DoublePrecision),
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: SpecSurface::ALL_TCL,
        lazily_readable: &[],
        startup_binding: StartupBinding::AppInit,
        runtime_hook: None,
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
        surface: TCL_AND_JIM,
        initially_bound: TCL_AND_JIM,
        lazily_readable: &[],
        startup_binding: StartupBinding::TclMain,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
        surface: SpecSurface::ALL_TCL,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
        keys: &[],
        externally_read: true,
        cmp_unsafe: false,
        write_effect: Some(SideEffectTarget::InterpState),
        read_taint: None,
        summary: "Script evaluated to print the continuation prompt for an incomplete command.",
    },
    host_variable(
        "jim::argv0",
        SpecialVarKind::Scalar,
        JIM_SHELL,
        StartupBinding::TclMain,
        "Invocation name of the Jim shell executable.",
    ),
    host_variable(
        "jim::exe",
        SpecialVarKind::Scalar,
        JIM_SHELL,
        StartupBinding::None,
        "Path of the Jim shell executable when it can be resolved.",
    ),
    host_variable(
        "jim::lineedit",
        SpecialVarKind::Scalar,
        JIM_SHELL,
        StartupBinding::TclMain,
        "Whether the Jim shell has interactive line editing.",
    ),
    host_variable(
        "tcl::autocomplete_commands",
        SpecialVarKind::Scalar,
        JIM_SHELL,
        StartupBinding::AppInit,
        "Commands whose subcommands the Jim shell offers for completion.",
    ),
    host_variable(
        "tcl::stdhint_commands",
        SpecialVarKind::Scalar,
        JIM_SHELL,
        StartupBinding::AppInit,
        "Commands whose subcommands the Jim shell offers as hints.",
    ),
    host_variable(
        "tcl::stdhint_col",
        SpecialVarKind::Scalar,
        JIM_SHELL,
        StartupBinding::AppInit,
        "Colour and intensity of command hints in the Jim shell.",
    ),
    host_variable(
        "tcl::stdhint_cols",
        SpecialVarKind::Array,
        JIM_SHELL,
        StartupBinding::AppInit,
        "Colour and intensity names available for Jim shell hints.",
    ),
    host_variable(
        "binary::convtype",
        SpecialVarKind::Array,
        JIM_SHELL,
        StartupBinding::None,
        "Conversion codes supplied by Jim's optional `binary` extension.",
    ),
    host_variable(
        "binary::scalarinfo",
        SpecialVarKind::Array,
        JIM_SHELL,
        StartupBinding::None,
        "Scalar formats supplied by Jim's optional `binary` extension.",
    ),
    host_variable(
        "tk_version",
        SpecialVarKind::Scalar,
        SpecSurface::TK,
        StartupBinding::AppInit,
        "The major and minor version of the loaded Tk package.",
    ),
    host_variable(
        "tk_patchLevel",
        SpecialVarKind::Scalar,
        SpecSurface::TK,
        StartupBinding::AppInit,
        "The full patch level of the loaded Tk package.",
    ),
    SpecialVarSpec {
        access: VarAccess::ReadWrite,
        externally_read: true,
        ..host_variable(
            "tk_library",
            SpecialVarKind::Scalar,
            SpecSurface::TK,
            StartupBinding::AppInit,
            "Directory holding Tk's script library.",
        )
    },
    SpecialVarSpec {
        access: VarAccess::ReadWrite,
        externally_read: true,
        write_effect: Some(SideEffectTarget::InterpState),
        ..host_variable(
            "tk_strictMotif",
            SpecialVarKind::Scalar,
            SpecSurface::TK,
            StartupBinding::AppInit,
            "Controls whether Tk enforces strict Motif compatibility.",
        )
    },
    host_variable(
        "tk::Priv",
        SpecialVarKind::Array,
        SpecSurface::TK,
        StartupBinding::AppInit,
        "Tk's internal state for the current display.",
    ),
    // ── iRules-specific ────────────────────────────────────────────────────
    SpecialVarSpec {
        name: "static",
        kind: SpecialVarKind::Namespace,
        access: VarAccess::ReadWrite,
        origin: VarOrigin::Dialect,
        surface: SpecSurface::IRULES,
        initially_bound: &[],
        lazily_readable: &[],
        startup_binding: StartupBinding::None,
        runtime_hook: None,
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
mod tests;

/// The native entry point that produces a root-variable inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeBootstrapPurpose {
    /// Core constructor, before any script library or application arguments.
    CreateInterpreter,
    /// `Tcl_Main` argument publication, before its application initializer.
    MainArguments,
}

/// An operation-owned root variable; enumeration order is allocation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeBootstrapVariable {
    /// Undefined native error context trace cell.
    ErrorInfo,
    /// Undefined native error classification trace cell.
    ErrorCode,
    /// Process environment array.
    Environment,
    /// Native platform description array.
    Platform,
    /// Actual release patch level.
    PatchLevel,
    /// Actual release major/minor version.
    Version,
    /// Undefined precision read/write trace cell.
    Precision,
    /// C8.4 platform-provided default library path.
    DefaultLibrary,
    /// Build-provided package search path.
    PackagePath,
    /// Jim constructor's build-provided library search path.
    AutoPath,
    /// Application interactivity flag.
    Interactive,
    /// Application executable spelling.
    Argv0,
    /// Application argument count.
    Argc,
    /// Application original argument list.
    Argv,
}

impl NativeBootstrapVariable {
    /// Original global tail owned by this native producer.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ErrorInfo => "errorInfo",
            Self::ErrorCode => "errorCode",
            Self::Environment => "env",
            Self::Platform => "tcl_platform",
            Self::PatchLevel => "tcl_patchLevel",
            Self::Version => "tcl_version",
            Self::Precision => "tcl_precision",
            Self::DefaultLibrary => "tclDefaultLibrary",
            Self::PackagePath => "tcl_pkgPath",
            Self::AutoPath => "auto_path",
            Self::Interactive => "tcl_interactive",
            Self::Argv0 => "argv0",
            Self::Argc => "argc",
            Self::Argv => "argv",
        }
    }
    /// A constructor allocation with no defined guest value.
    #[must_use]
    pub const fn is_undefined(self) -> bool {
        matches!(self, Self::ErrorInfo | Self::ErrorCode | Self::Precision)
    }
}

/// Independently supplied host/build bytes, never inferred from a Tcl release.
#[derive(Clone, Debug, Default)]
pub struct NativeBootstrapInputs {
    /// Counted native list spelling of the build's package or Jim library path.
    pub package_path: Vec<u8>,
    /// Actual C8.4 platform initializer's default library, when that producer exists.
    pub default_library: Option<Vec<u8>>,
}

/// The original purpose of a copied Jim interpreter value. This selects only
/// a string extent; independently live source and target object owners are required.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimInterpreterCopyPurpose {
    /// `JimInterpCopyObj` copies all counted bytes of scripts, arguments and results.
    Object,
    /// `JimInterpCopyVariable` passes `Jim_String` to `NewStringObj` with length -1.
    ChildStartupVariable,
}

impl NativeJimInterpreterCopyPurpose {
    /// Select the original API's input extent without changing the source object.
    #[must_use]
    pub fn input(self, original: &[u8]) -> &[u8] {
        match self {
            Self::Object => original,
            Self::ChildStartupVariable => tcl_core_types::c_string_extent(original),
        }
    }
}

/// Provider-specific child initialisation, separate from the original core
/// constructor. The Jim recipe refers to the pinned static-extension roster;
/// a backend still admits only extensions it implements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeChildInitialisation {
    names: tcl_syntax::naming::NativeNameProtocol,
}

impl NativeChildInitialisation {
    /// `JimInterpCommand` explicitly initializes its compiled static extensions.
    /// C retains its independently selected child constructor path.
    #[must_use]
    pub const fn initialises_jim_static_extensions(self) -> bool {
        matches!(self.names, tcl_syntax::naming::NativeNameProtocol::Jim084)
    }

    /// `JimInterpCommand`'s exact `GLOBAL_ONLY` variable-copy operands. Missing
    /// parent values are not synthesized; C grants no Jim variable-copy plan.
    #[must_use]
    pub const fn copied_parent_variables(self) -> &'static [&'static [u8]] {
        match self.names {
            tcl_syntax::naming::NativeNameProtocol::Jim084 => &[
                b"argv",
                b"argc",
                b"argv0",
                b"jim::argv0",
                b"jim::exe",
                b"jim::lineedit",
            ],
            tcl_syntax::naming::NativeNameProtocol::C(_) => &[],
        }
    }
}

/// Actual audited constructor family; this grants no library initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBootstrapProtocol {
    names: tcl_syntax::naming::NativeNameProtocol,
}

impl NativeBootstrapProtocol {
    /// The authenticated name/storage recipe of the producing engine.
    #[must_use]
    pub const fn names(self) -> tcl_syntax::naming::NativeNameProtocol {
        self.names
    }

    /// The child API's initialisation purpose. No parent object, library
    /// command identity or current child state follows from this recipe.
    #[must_use]
    pub const fn child_initialisation(self) -> NativeChildInitialisation {
        NativeChildInitialisation { names: self.names }
    }

    /// Audited constructor math-function commands, before library initialisation.
    /// C8.6's `BuiltinFuncTable` in `tclBasic.c` registers these 29 commands;
    /// `min` and `max` are procedures supplied by `library/init.tcl`.
    /// Other constructor inventories are not admitted by this receipt.
    #[must_use]
    pub const fn core_mathfunc_roster(self) -> Option<&'static [&'static str]> {
        match self.names {
            tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_6) => Some(&[
                "abs", "acos", "asin", "atan", "atan2", "bool", "ceil", "cos", "cosh", "double",
                "entier", "exp", "floor", "fmod", "hypot", "int", "isqrt", "log", "log10", "pow",
                "rand", "round", "sin", "sinh", "sqrt", "srand", "tan", "tanh", "wide",
            ]),
            _ => None,
        }
    }

    /// Whether the original constructor installs math functions as commands.
    /// C8.4 uses its private function table; Jim uses its expression table.
    #[must_use]
    pub const fn registers_core_mathfunc(self) -> bool {
        matches!(
            self.names,
            tcl_syntax::naming::NativeNameProtocol::C(
                tcl_dialect::TclVersion::V8_5
                    | tcl_dialect::TclVersion::V8_6
                    | tcl_dialect::TclVersion::V9_0
                    | tcl_dialect::TclVersion::V9_1
            )
        )
    }

    /// Whether the core constructor registers the native binary command.
    /// Jim's binary command is a distribution script over an optional pack
    /// extension, outside `Jim_RegisterCoreCommands`.
    #[must_use]
    pub const fn registers_core_binary(self) -> bool {
        matches!(self.names, tcl_syntax::naming::NativeNameProtocol::C(_))
    }

    /// Whether the selected core registers Tcl's library-path initializer.
    #[must_use]
    pub const fn registers_core_init_auto_path(self) -> bool {
        matches!(
            self.names,
            tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V9_1)
        )
    }

    /// Whether the constructor registers and exports the original C operator
    /// namespace. Jim and C8.4 have no `::tcl::mathop` core command family.
    #[must_use]
    pub const fn registers_core_mathop(self) -> bool {
        matches!(
            self.names,
            tcl_syntax::naming::NativeNameProtocol::C(
                tcl_dialect::TclVersion::V8_5
                    | tcl_dialect::TclVersion::V8_6
                    | tcl_dialect::TclVersion::V9_0
                    | tcl_dialect::TclVersion::V9_1
            )
        )
    }

    /// Whether the native core registers its structured `try` handler.
    #[must_use]
    pub const fn registers_core_try(self) -> bool {
        match self.names {
            tcl_syntax::naming::NativeNameProtocol::Jim084 => true,
            tcl_syntax::naming::NativeNameProtocol::C(version) => {
                matches!(
                    version,
                    tcl_dialect::TclVersion::V8_6
                        | tcl_dialect::TclVersion::V9_0
                        | tcl_dialect::TclVersion::V9_1
                )
            }
        }
    }

    /// Whether the native core registers `throw`; Jim has no core worker.
    #[must_use]
    pub const fn registers_core_throw(self) -> bool {
        matches!(
            self.names,
            tcl_syntax::naming::NativeNameProtocol::C(
                tcl_dialect::TclVersion::V8_6
                    | tcl_dialect::TclVersion::V9_0
                    | tcl_dialect::TclVersion::V9_1
            )
        )
    }

    /// Whether the native core constructor itself initializes `TclOO`.
    /// Jim and C84/C85 require a separately loaded implementation; compatible
    /// source syntax or the host's analytical command set supplies no authority.
    #[must_use]
    pub const fn initializes_tcl_oo(self) -> bool {
        matches!(
            self.names,
            tcl_syntax::naming::NativeNameProtocol::C(
                tcl_dialect::TclVersion::V8_6
                    | tcl_dialect::TclVersion::V9_0
                    | tcl_dialect::TclVersion::V9_1
            )
        )
    }

    /// Ordered native root allocations. `default_library` is a separately
    /// supplied platform producer fact, not a release-based platform guess.
    #[must_use]
    pub fn allocations(
        self,
        purpose: NativeBootstrapPurpose,
        default_library: bool,
    ) -> Option<Vec<NativeBootstrapVariable>> {
        use NativeBootstrapVariable as V;
        use tcl_syntax::naming::NativeNameProtocol as N;
        if purpose == NativeBootstrapPurpose::MainArguments {
            return match self.names {
                N::C(_) => Some(vec![V::Argv0, V::Argc, V::Argv, V::Interactive]),
                N::Jim084 => None,
            };
        }
        match self.names {
            N::Jim084 => Some(vec![V::AutoPath, V::Interactive, V::Platform]),
            N::C(version) => {
                let mut rows = Vec::new();
                if version >= tcl_dialect::TclVersion::V8_5 {
                    rows.extend([V::ErrorInfo, V::ErrorCode]);
                }
                rows.extend([V::Environment, V::Platform, V::PatchLevel, V::Version]);
                if version < tcl_dialect::TclVersion::V9_0 {
                    rows.push(V::Precision);
                }
                if version == tcl_dialect::TclVersion::V8_4 && default_library {
                    rows.push(V::DefaultLibrary);
                }
                rows.push(V::PackagePath);
                Some(rows)
            }
        }
    }
}

/// [`InvocationDialect`](tcl_native_model::InvocationDialect) selectors for this module's types. The types
/// live in the registry core, so the selectors are an extension trait.
pub trait BootstrapDialect {
    /// Actual native constructor policy; vendor compatibility and authored
    /// simulations cannot issue a physical root birth inventory.
    #[must_use]
    fn native_bootstrap_protocol(self) -> Option<NativeBootstrapProtocol>;
}

impl BootstrapDialect for tcl_native_model::InvocationDialect {
    fn native_bootstrap_protocol(self) -> Option<NativeBootstrapProtocol> {
        Some(NativeBootstrapProtocol {
            names: self.native_name_protocol()?,
        })
    }
}

#[cfg(test)]
mod native_bootstrap_tests {
    use super::*;

    // Native proof: naming.bootstrap.original-root-cells
    // docs/design/analysis/name-resolution-proofs/bootstrap-original-root-cells.md
    #[test]
    fn constructor_root_order_and_undefined_cells_match_native_capture() {
        let fixture = include_str!("../tests/data/native_bootstrap/core-roots.tsv");
        for profile_name in tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(tcl_dialect::TclVersion::dialect_profile_name)
            .chain(["jim"])
        {
            let profile = crate::model::resolve_environment(profile_name).unit_profile();
            let dialect = tcl_native_model::InvocationDialect::of_profile(profile);
            let protocol = dialect.native_bootstrap_protocol().unwrap();
            let abi = tcl_core_types::NativeHashAbi {
                plain_char: tcl_core_types::NativeHashBytePromotion::Signed,
                unsigned_int: tcl_core_types::NativeHashWordWidth::Bits32,
                size_t: tcl_core_types::NativeHashWordWidth::Bits64,
                jim_seed: Some(0),
            };
            let hash = dialect.native_variable_table_protocol(abi).unwrap();
            let mut ledger = tcl_core_types::NativeEntryLedger::default();
            ledger.select_recipe(Some(hash.recipe()));
            let variables = protocol
                .allocations(NativeBootstrapPurpose::CreateInterpreter, true)
                .unwrap();
            for variable in &variables {
                ledger.insert(variable.name().as_bytes());
            }
            let expected: Vec<_> = fixture
                .lines()
                .filter_map(|line| {
                    let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(line).collect();
                    (fields[0] == profile_name).then_some((fields[1], fields[2] == "1"))
                })
                .collect();
            assert_ne!(expected.len(), 0);
            let actual: Vec<_> = ledger
                .keys()
                .unwrap()
                .into_iter()
                .map(|key| {
                    let variable = variables
                        .iter()
                        .find(|variable| variable.name().as_bytes() == key)
                        .unwrap();
                    (variable.name(), !variable.is_undefined())
                })
                .collect();
            assert_eq!(actual, expected, "{profile_name}");
        }
    }

    #[test]
    fn application_arguments_and_unknown_engines_have_separate_authority() {
        let c = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
            .native_bootstrap_protocol()
            .unwrap();
        let names: Vec<_> = c
            .allocations(NativeBootstrapPurpose::MainArguments, false)
            .unwrap()
            .into_iter()
            .map(NativeBootstrapVariable::name)
            .collect();
        assert_eq!(names, ["argv0", "argc", "argv", "tcl_interactive"]);
        let jim = tcl_native_model::InvocationDialect::of_profile(
            crate::model::resolve_environment("jim").unit_profile(),
        )
        .native_bootstrap_protocol()
        .unwrap();
        assert!(
            jim.allocations(NativeBootstrapPurpose::MainArguments, false)
                .is_none()
        );
        assert!(
            tcl_native_model::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find("f5-irules").unwrap()
            )
            .native_bootstrap_protocol()
            .is_none()
        );
    }
}

#[cfg(test)]
mod native_error_variable_tests {
    use super::*;

    #[test]
    fn hidden_error_read_policy_matches_five_original_c_releases() {
        let fixture = include_str!("../tests/data/native_error_variables/observations.tsv");
        assert_eq!(fixture.lines().count(), 50);
        for (name, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let profile = crate::model::resolve_environment(name).unit_profile();
            let protocol = tcl_native_model::InvocationDialect::of_profile(profile)
                .native_error_variable_protocol();
            assert_eq!(protocol.is_some(), name != "tcl8.4");
            let rows: Vec<_> = fixture
                .lines()
                .filter(|line| line.starts_with(version))
                .collect();
            assert_eq!(rows.len(), 10);
            if let Some(protocol) = protocol {
                assert!(rows[3].contains("errorInfo_same=1"));
                assert!(rows[4].contains("errorInfo_present=1\terrorInfo_defined=0"));
                assert!(rows[7].contains("errorCode_present=1\terrorCode_defined=0"));
                assert!(rows[8].contains("errorCode_value=\"\""));
                assert_eq!(
                    protocol.read(false, true, true),
                    NativeErrorVariableRead::Preserve
                );
                assert_eq!(
                    protocol.read(true, true, true),
                    NativeErrorVariableRead::CopyPrivate
                );
                assert_eq!(
                    protocol.read(true, false, false),
                    NativeErrorVariableRead::DefineEmpty
                );
                assert_eq!(
                    protocol.read(true, false, true),
                    NativeErrorVariableRead::Preserve
                );
            }
        }
        for name in ["jim", "f5-irules"] {
            let profile = crate::model::resolve_environment(name).unit_profile();
            assert!(
                tcl_native_model::InvocationDialect::of_profile(profile)
                    .native_error_variable_protocol()
                    .is_none()
            );
        }
    }
}
