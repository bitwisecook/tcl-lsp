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

//! The `DialectProfile` catalogue — one interned profile per canonical dialect.
//!
//! A profile is resolved from a dialect-name string **once at ingest**
//! (LSP `dialect_for_open`, CLI `effective_dialect`, `detect_dialect`) and
//! the `&'static DialectProfile` is threaded from there, so consumers stop
//! re-parsing dialect strings per query.
//!
//! This is the compositional model of `docs/design/registry/dialect-profile-model.md`:
//! identity, the availability axis — masks, load layers, grammar unions —
//! the behaviour/runtime axis — base versions, octal policy, expr grammar,
//! lexer grammar, the per-dialect predicates — and the versioned-library
//! axis.

use crate::grammar::LexerGrammar;
use crate::library::{LibraryPin, LibraryVersion, LibraryVersionOverrides};
use crate::model::{Family, PackageFloor, SpecProvider, SurfaceLayer};
use crate::version::{TclVersion, Ternary};

mod catalog;
mod engine_policy;

use catalog::{CATALOG, PLAIN_TCL, TK_PROFILE, core_provider, tcl_version_of};

/// What stands behind a profile's evaluation semantics: the evidence gate a
/// compile-time fold passes before it evaluates under the profile's release.
///
/// [`DialectProfile::runtime_base`] says which Tcl release a profile's runtime
/// was *modelled* on, and nothing in it says the model was ever compared with the
/// real thing. A fold bakes a value into a program, and is only as sound as that
/// comparison, so [`DialectProfile::evaluation_point`] answers a release only
/// where one of these says there was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvaluationEvidence {
    /// The profile is a Tcl release itself, and `data/reference-toolchains.tsv`
    /// pins an interpreter for it that the differential suites run.
    ReferenceToolchain,
    /// A vendor fork of a Tcl release, measured on the real environment.
    MeasuredFork {
        /// Where the measurement is recorded.
        note: &'static str,
    },
    /// Nothing measured: the runtime base is a claim and not evidence.
    Unmeasured,
}

/// One filename extension a dialect owns, with its human-facing name —
/// the catalogue analogue of a `SpecTcl` pack's
/// `file_extension upf -name {Unified Power Format}` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DialectFileExtension {
    /// Lower-case extension without the leading dot (`"xdc"`).
    pub extension: &'static str,
    /// What the file type is called (`"Xilinx Design Constraints"`).
    pub display_name: &'static str,
}

/// One resolved dialect. `'static`, interned in [`DialectProfile::all`],
/// keyed by canonical name.
///
/// Equality of profiles is pointer identity — there is exactly one profile
/// per canonical dialect, plus the [`DialectProfile::plain_tcl`] fallback
/// every unknown name resolves to.
///
/// The behaviour fields are **derived data fixed at catalogue-construction
/// time** from the §7 table of the design doc (`signature_base` /
/// `runtime_base` / the vendor surface drive the rest); the derivation rules
/// (§7.1) are enforced by this module's invariant tests so the hand-laid
/// values can never drift from the model.
#[derive(Debug, Clone)]
pub struct DialectProfile {
    /// Explicit execution release and build retained by a projected environment.
    /// Catalogue profiles can express an unpinned family without inventing a point.
    pub core_point: Option<crate::model::DialectPoint>,
    /// The canonical dialect name (`"tcl8.6"`, `"f5-irules"`, …). Stable:
    /// this is the string that round-trips through configuration
    /// (`tclLsp.selectDialect`, `folderDialects`), the registry-dump JSON
    /// schema, and the editor catalogues.
    pub name: &'static str,
    /// Legacy / editor spellings that resolve to this profile
    /// (`"irules"` → `f5-irules`). Resolution through [`Self::find`] (and
    /// the environment seam built on it) canonicalises them, so profile
    /// predicates can never disagree with the canonical spelling the way
    /// separate string-keyed tables would (design doc §2.4).
    pub aliases: &'static [&'static str],
    /// The full human-facing name shown in settings menus and pickers
    /// (`"Xilinx Vivado"`, `"Tcl 8.6"`). The catalogue is the single
    /// source for editor presentation: `cargo xtask gen-editor-dialects`
    /// projects this into every editor's dialect list, so adding a
    /// profile ships its label everywhere at once.
    pub display_name: &'static str,
    /// A compact label for tight UI (the compiler-explorer dropdown,
    /// status bars): `"Vivado"`, `"iRules"`. Never empty — repeats
    /// [`Self::display_name`] where no shorter form exists.
    pub short_name: &'static str,
    /// The editor language id this dialect's files open under, where the
    /// editors keep a dedicated language (`"tcl-synopsys"`, `"tcl84"`).
    /// Undotted by contract — VS Code splits `configurationDefaults`
    /// override keys on `.`. `None` = no dedicated editor
    /// language; the dialect's files (if any) ride the plain `tcl`
    /// language and server-side detection routes them.
    pub editor_language_id: Option<&'static str>,
    /// Whole *basenames* this dialect owns, matched instead of an extension
    /// (`bigip.conf`, `bigip_base.conf`). Lower-case, compared
    /// case-insensitively against a path's last component, unique across the
    /// catalogue.
    ///
    /// The second axis of file recognition, and it needs its own field
    /// because the file it names has no useful extension: a bare `.conf`
    /// suffix belongs to every unrelated config file on the machine, so
    /// `f5-bigip` can only claim `bigip.conf` by *name*. Keeping this set
    /// inside `tcl_lsp_core::bigip` instead, where the editors cannot read
    /// it, would leave VS Code contributing no `filenames` at all so a
    /// `bigip.conf` never associates — a hazard the Sublime grammar's own
    /// comment gets backwards.
    ///
    /// Consumed exactly where `file_extensions` is: `dialect_from_extension`
    /// checks it *before* the extension tier (a basename match is the more
    /// specific claim), and `cargo xtask gen-editor-extensions` projects it
    /// into each editor's per-language `filenames` list.
    pub filenames: &'static [&'static str],
    /// The filename extensions this dialect owns, each with its
    /// human-facing name (`xdc` / "Xilinx Design Constraints"). This is
    /// the source of truth its consumers project: extension→dialect
    /// routing (`tcl-registry`'s `dialect_from_extension` fallback), the
    /// editors' registered extension lists (`cargo xtask
    /// gen-editor-extensions`), and any UI that names a file type.
    /// Lower-case, no leading dot, unique across the catalogue. `SpecTcl`
    /// packs can register further extensions at load time
    /// (`file_extension` rows) — those layer on top of, and are consulted
    /// before, this static set.
    pub file_extensions: &'static [DialectFileExtension],
    /// Native tag of this dialect's own command surface, if any (`IRULES`,
    /// `IAPPS`, `EXPECT`, an EDA vendor, `BPF`). `None` for the plain
    /// Tcl-version profiles, the config-only dialects, and the permissive
    /// fallback. A vendor shell is a *closed world*: desktop libraries
    /// (Tk) can never be `package require`d into it, which consumers gate
    /// on via this field until the versioned-library axis models library
    /// hosting per profile (§7.2).
    pub vendor_surface: Option<SpecProvider>,
    /// The packages this profile's **own point** carries.
    ///
    /// For a vendor shell this is its vendor package; for the `tk` ingress
    /// profile it is `Tk`, which is why the field exists at all: Tk is a
    /// library, not a closed-world vendor surface, so
    /// [`Self::vendor_surface`] cannot carry it without making `tk` a closed
    /// world. Empty for every plain Tcl version — Tk there needs a
    /// `package require`.
    ///
    /// Each is named with no floor: the profile says which package its point
    /// carries, not which release of it. A registry that knows a release —
    /// a pack's `ambient_package` row — states it on its own query.
    ///
    /// Pinned against [`Self::vendor_surface`] by
    /// `surface_packages_carry_the_vendor_surface`, so the two cannot drift.
    pub surface_packages: &'static [PackageFloor<'static>],

    // AXIS A: availability.
    /// The registry command packs `load_dialect` applies for this profile,
    /// in order. A plain Tcl version's layer carries no specs — it only
    /// records which release the registry is — and the permissive fallback
    /// profile loads nothing at all.
    pub base_layers: &'static [SurfaceLayer],
    /// Coarse over-approximating union for **static** grammars only
    /// (tree-sitter / tmLanguage first-paint highlighting). Deliberately
    /// wider than [`Self::surface_query`] — precise per-version
    /// correctness is the LSP semantic-token layer's job (§10). iRules is
    /// the exception: its static grammar names only its own family, which
    /// is the shipped highlight fix this model preserves.
    pub grammar_union: &'static [SpecProvider],
    /// UPPER-BOUND version guard for option gating (design doc §5.2): the
    /// highest Tcl version whose options may appear under this profile. A
    /// version-gated option resolves only when its gate's
    /// [`core_tcl_floor`] is at or below this ceiling, so a
    /// tcl9.0-only option can never leak into an 8.5-superset profile whose
    /// point happens to sit inside its gate. `None` = no ceiling (the
    /// permissive fallback and the interim config-only dialects).
    pub version_ceiling: Option<TclVersion>,

    // AXIS B: behaviour / runtime.
    /// The Tcl version whose command/subcommand/option *signatures* this
    /// dialect exposes — the release half of [`Self::surface_query`].
    /// `None` for a non-Tcl surface (`f5-bigip`) and the permissive
    /// fallback.
    pub signature_base: Option<TclVersion>,
    /// The Tcl version whose *evaluation semantics* apply: octal, expr
    /// grammar (TIP 201/461), mathfunc ceiling, number parsing, const-fold.
    /// Equal to [`Self::signature_base`] for every ordinary Tcl version;
    /// both are `V8_4` for iRules (D3: a genuine embedded Tcl 8.4.6,
    /// nothing backported at any BIG-IP version). Kept structurally
    /// distinct from `signature_base` because the const-fold and
    /// expr-grammar paths key off different projections (§2.1).
    pub runtime_base: Option<TclVersion>,
    /// Whether a bare leading-zero integer literal reads as octal.
    /// Derived: `runtime_base < V9_0` (Tcl 9.0 dropped the rule, TIP
    /// 114/472); [`Ternary::Inert`] — not a silent default — when there is
    /// no Tcl runtime to have an opinion (§11.1).
    pub leading_zero_is_octal: Ternary,
    /// The Tcl core-grammar version the dialect's `expr` evaluator behaves
    /// like — gates TIP 201 (`in`/`ni`, 8.5+) and TIP 461
    /// (`lt`/`le`/`gt`/`ge`, 9.0+) and the mathfunc tiers. Always equal to
    /// [`Self::runtime_base`] (§7.1); `None` means the validators return
    /// only the dialect-invariant subset.
    pub expr_grammar_base: Option<TclVersion>,
    /// The dialect-derived lexing grammar (`{*}` expansion, the iRules
    /// `}{` separator, the `${…}` delimiting rule). The single source
    /// `LexerConfig::for_dialect` reads — derived from
    /// `runtime_base` (8.x → first-close, 9.x → nesting) plus the
    /// per-dialect quirks.
    pub grammar: LexerGrammar,
    /// Whether the math-operator command heads (`+`, `eq`,
    /// `tcl::mathop::*`) exist as callable commands. False for iRules —
    /// operators there live only inside `expr` (§9's second conflated
    /// fact; `tk` is modelled as a library, not a profile).
    pub operators_as_commands: bool,
    /// Whether `TclOO` (`oo::*`) is part of this dialect's surface. Explicit
    /// per profile and invariant-tested against what the profile's point
    /// actually resolves (§11.2), so the flag cannot drift from it.
    pub tcloo: bool,
    /// Whether the dialect's ensemble commands are *fixed* — a closed
    /// subcommand set with no user-extensible ensembles — so the minifier
    /// may shorten subcommands to unambiguous prefixes. Exactly the F5
    /// family `{f5-irules, f5-iapps, f5-bigip}` — NOT f5-tmsh (§7.1).
    pub has_fixed_ensembles: bool,
    /// The Tcl release the bytecode VM emulates when executing this dialect.
    /// This stays aligned with [`Self::runtime_base`] for every Tcl profile;
    /// the inert config-only and permissive profiles retain the V9.0 default.
    /// Keeping the selected release on the profile lets all runtime consumers
    /// share one version decision rather than interpreting a dialect name.
    pub vm_runtime_version: TclVersion,
    /// Whether anything measured [`Self::runtime_base`], which is what lets a
    /// compile-time fold evaluate under it ([`Self::evaluation_point`]).
    pub evaluation_evidence: EvaluationEvidence,

    // AXIS C: versioned libraries (§7.1, D5).
    /// The library packages this profile models, each with its version pin
    /// (§7 "Libraries" column). An **ambient** pin is part of the modelled
    /// runtime (the F5 surfaces, an EDA shell's tool commands): no
    /// `package require` needed, and the pin supplies the version floor
    /// spec `min_version` gates compare against. A **hosted** pin (Tk /
    /// Itcl on plain Tcl) still needs its `package require` and only
    /// refines the floor when the require names no version. Explicit
    /// versioned requires raise floors, never lower them below the pin.
    pub libraries: &'static [LibraryPin],

    // AXIS D: out-of-registry vendor knowledge (§5.4).
    /// Lower-case substring terms that select this dialect's entries in
    /// the KCS help index (`tcl help --dialect`). Empty = no filtering
    /// (the permissive fallback). Resolution through the catalogue means
    /// alias spellings (`irules`) filter exactly like the canonical name
    /// — a plain string-keyed table would silently apply no filter to them.
    pub help_terms: &'static [&'static str],
}

/// A `'static` profile axis compared by value, with pointer identity as a
/// fast path: keys projected from one interned profile share their slices.
struct StaticAxis<T: ?Sized + 'static>(&'static T);

impl<T: ?Sized> Clone for StaticAxis<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for StaticAxis<T> {}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for StaticAxis<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: ?Sized + Eq> Eq for StaticAxis<T> {}

impl<T: ?Sized + PartialEq> PartialEq for StaticAxis<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0) || self.0 == other.0
    }
}

impl<T: ?Sized + std::hash::Hash> std::hash::Hash for StaticAxis<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// A value identity for every profile axis, including the execution point,
/// build, numeric and lexer policies, availability, and library pins.
///
/// Use this for caches receiving an already selected profile. A display name
/// or a Tcl version cannot distinguish projected engines or policy overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DialectProfileKey {
    core_point: Option<crate::model::DialectPoint>,
    name: StaticAxis<str>,
    aliases: StaticAxis<[&'static str]>,
    display_name: StaticAxis<str>,
    short_name: StaticAxis<str>,
    editor_language_id: Option<&'static str>,
    filenames: StaticAxis<[&'static str]>,
    file_extensions: StaticAxis<[DialectFileExtension]>,
    vendor_surface: Option<SpecProvider>,
    surface_packages: StaticAxis<[PackageFloor<'static>]>,
    base_layers: StaticAxis<[SurfaceLayer]>,
    grammar_union: StaticAxis<[SpecProvider]>,
    version_ceiling: Option<TclVersion>,
    signature_base: Option<TclVersion>,
    runtime_base: Option<TclVersion>,
    leading_zero_is_octal: Ternary,
    expr_grammar_base: Option<TclVersion>,
    grammar: LexerGrammar,
    operators_as_commands: bool,
    tcloo: bool,
    has_fixed_ensembles: bool,
    vm_runtime_version: TclVersion,
    evaluation_evidence: EvaluationEvidence,
    libraries: StaticAxis<[LibraryPin]>,
    help_terms: StaticAxis<[&'static str]>,
}

impl DialectProfileKey {
    /// Recover this exact frozen snapshot without interpreting its name.
    /// Useful when a persisted incremental key must restore execution policy.
    #[must_use]
    pub fn profile(&self) -> &'static DialectProfile {
        DialectProfile {
            core_point: self.core_point,
            name: self.name.0,
            aliases: self.aliases.0,
            display_name: self.display_name.0,
            short_name: self.short_name.0,
            editor_language_id: self.editor_language_id,
            filenames: self.filenames.0,
            file_extensions: self.file_extensions.0,
            vendor_surface: self.vendor_surface,
            surface_packages: self.surface_packages.0,
            base_layers: self.base_layers.0,
            grammar_union: self.grammar_union.0,
            version_ceiling: self.version_ceiling,
            signature_base: self.signature_base,
            runtime_base: self.runtime_base,
            leading_zero_is_octal: self.leading_zero_is_octal,
            expr_grammar_base: self.expr_grammar_base,
            grammar: self.grammar,
            operators_as_commands: self.operators_as_commands,
            tcloo: self.tcloo,
            has_fixed_ensembles: self.has_fixed_ensembles,
            vm_runtime_version: self.vm_runtime_version,
            evaluation_evidence: self.evaluation_evidence,
            libraries: self.libraries.0,
            help_terms: self.help_terms.0,
        }
        .intern()
    }
}

/// Profile equality **is** pointer identity, as the type's contract states:
/// every profile a consumer holds came from the interned catalogue (or the
/// [`DialectProfile::plain_tcl`] sink), so two handles name the same dialect
/// exactly when they are the same allocation. Spelling it as a trait impl
/// lets a config type that carries a resolved profile keep deriving
/// `PartialEq`.
impl PartialEq for DialectProfile {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for DialectProfile {}

impl DialectProfile {
    /// Freeze every axis used by a profile consumer into a cache key.
    ///
    /// ```
    /// use tcl_dialect::DialectProfile;
    /// assert_ne!(DialectProfile::find("tcl8.4").unwrap().cache_key(),
    ///            DialectProfile::find("tcl9.1").unwrap().cache_key());
    /// ```
    #[must_use]
    pub fn cache_key(&self) -> DialectProfileKey {
        // Exhaustive destructuring makes a new profile axis require a deliberate
        // cache identity update; it cannot silently bypass this shared owner.
        let Self {
            core_point,
            name,
            aliases,
            display_name,
            short_name,
            editor_language_id,
            filenames,
            file_extensions,
            vendor_surface,
            surface_packages,
            base_layers,
            grammar_union,
            version_ceiling,
            signature_base,
            runtime_base,
            leading_zero_is_octal,
            expr_grammar_base,
            grammar,
            operators_as_commands,
            tcloo,
            has_fixed_ensembles,
            vm_runtime_version,
            evaluation_evidence,
            libraries,
            help_terms,
        } = self;
        DialectProfileKey {
            core_point: *core_point,
            name: StaticAxis(name),
            aliases: StaticAxis(aliases),
            display_name: StaticAxis(display_name),
            short_name: StaticAxis(short_name),
            editor_language_id: *editor_language_id,
            filenames: StaticAxis(filenames),
            file_extensions: StaticAxis(file_extensions),
            vendor_surface: *vendor_surface,
            surface_packages: StaticAxis(surface_packages),
            base_layers: StaticAxis(base_layers),
            grammar_union: StaticAxis(grammar_union),
            version_ceiling: *version_ceiling,
            signature_base: *signature_base,
            runtime_base: *runtime_base,
            leading_zero_is_octal: *leading_zero_is_octal,
            expr_grammar_base: *expr_grammar_base,
            grammar: *grammar,
            operators_as_commands: *operators_as_commands,
            tcloo: *tcloo,
            has_fixed_ensembles: *has_fixed_ensembles,
            vm_runtime_version: *vm_runtime_version,
            evaluation_evidence: *evaluation_evidence,
            libraries: StaticAxis(libraries),
            help_terms: StaticAxis(help_terms),
        }
    }

    /// Intern an exact profile snapshot for APIs that retain a static handle.
    /// Catalogue handles keep their existing pointer identity. Projected
    /// snapshots share an allocation only when every profile axis agrees.
    #[must_use]
    pub fn intern(&self) -> &'static Self {
        static PROFILES: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashMap<DialectProfileKey, &'static DialectProfile>>,
        > = std::sync::OnceLock::new();
        let key = self.cache_key();
        if let Some(profile) = Self::all()
            .iter()
            .chain([Self::plain_tcl(), Self::tk()])
            .find(|profile| profile.cache_key() == key)
        {
            return profile;
        }
        let mut profiles = PROFILES
            .get_or_init(Default::default)
            .lock()
            .expect("profile snapshot mutex");
        profiles
            .entry(key)
            .or_insert_with(|| Box::leak(Box::new(self.clone())))
    }

    /// A profile **projected from a resolved point** — the interop handed to
    /// consumers that still carry `&'static DialectProfile` for an
    /// environment the catalogue has no row for.
    ///
    /// `jim` is such an environment: its grammar depends on
    /// `(family, release, build)`, so the environment names its family and
    /// release ladder rather than owning a resolved-grammar row. The compiler,
    /// analyser, LSP providers and VM receive a `&'static DialectProfile`
    /// projected from that resolved grammar.
    ///
    /// Identity (name, aliases, display name) is the environment's; the
    /// grammar and the version bases are the point's; every *policy* field
    /// keeps the permissive fallback's value, which is exactly what such an
    /// environment received before. Nothing downstream gains a new opinion
    /// — it gains the right grammar, under the right name, so a consumer
    /// that later resolves by `profile.name` reaches the same point.
    #[must_use]
    pub fn projected_from_point(
        name: &'static str,
        aliases: &'static [&'static str],
        display_name: &'static str,
        point: crate::model::DialectPoint,
    ) -> Self {
        let tcl_version = tcl_version_of(point.release());
        Self {
            core_point: Some(point),
            name,
            aliases,
            display_name,
            short_name: display_name,
            editor_language_id: None,
            filenames: &[],
            file_extensions: &[],
            vendor_surface: None,
            surface_packages: &[],
            base_layers: &[],
            grammar_union: core_provider(point.family()),
            version_ceiling: None,
            signature_base: None,
            runtime_base: tcl_version,
            leading_zero_is_octal: Ternary::Inert,
            expr_grammar_base: tcl_version,
            grammar: point.grammar(),
            operators_as_commands: PLAIN_TCL.operators_as_commands,
            tcloo: PLAIN_TCL.tcloo,
            has_fixed_ensembles: PLAIN_TCL.has_fixed_ensembles,
            vm_runtime_version: tcl_version.unwrap_or(PLAIN_TCL.vm_runtime_version),
            evaluation_evidence: EvaluationEvidence::Unmeasured,
            libraries: &[],
            help_terms: &[],
        }
    }

    /// Whether `name` denotes the F5 iRules dialect — resolved through the
    /// profile catalogue, so the canonical `f5-irules` and every registered
    /// alias (`irules`, `tcl-irule`) agree with the profile predicates by
    /// construction (dialect-profile-model.md §2.4). The single source of
    /// truth for the "is this iRules?" check compiler and LSP passes need.
    #[must_use]
    pub fn name_is_irules(name: Option<&str>) -> bool {
        name.and_then(Self::find).is_some_and(Self::is_irules)
    }

    /// Whether `name`'s ensemble commands are *fixed* — the dialect ships a
    /// closed set of subcommands with no user-extensible ensembles — so the
    /// minifier may safely shorten subcommands to their unambiguous prefix.
    /// True for the F5 dialect family, resolved through the catalogue so
    /// alias spellings agree with the canonical name (§2.4).
    #[must_use]
    pub fn name_has_fixed_ensembles(name: Option<&str>) -> bool {
        name.and_then(Self::find)
            .is_some_and(|profile| profile.has_fixed_ensembles)
    }

    /// The full catalogue of canonical dialect profiles, in sorted-name order.
    /// Excludes the [`Self::plain_tcl`] fallback — it is a resolution sink, not
    /// a selectable dialect.
    ///
    /// The lexer's grammar rows and the editors' identity key, never a list of
    /// names shown to a user: that reads
    /// [`EnvironmentRegistry::selectable`](crate::model::EnvironmentRegistry::selectable).
    #[must_use]
    pub fn all() -> &'static [DialectProfile] {
        &CATALOG
    }

    /// Look up a **catalogue** profile by canonical name or registered
    /// alias — `None` for anything else, the `tk` ingress and the lenient
    /// sink included.
    ///
    /// This catalogue lookup is
    /// what the environment-model seam (`tcl_registry::model::ingress`)
    /// and the documented per-crate interop twins are built on, and it
    /// resolves an **environment id**, never a user-written dialect
    /// string. Every user-written name resolves through the seam's
    /// `resolve_environment` instead — the retired name validators
    /// (`by_name`, `by_opt_name`, `resolve_known`,
    /// `availability_for_name`) are deleted, not wrapped.
    #[must_use]
    pub fn find(name: &str) -> Option<&'static DialectProfile> {
        CATALOG
            .iter()
            .find(|p| p.name == name || p.aliases.contains(&name))
    }

    /// The `f5-irules` profile — an explicit handle for the hardcoded
    /// iRules lookups (event checks, taint, the iRules test framework).
    #[must_use]
    pub fn irules() -> &'static DialectProfile {
        Self::find("f5-irules").expect("f5-irules is in the catalog")
    }

    /// The permissive fallback profile every unknown / unset dialect
    /// resolves to (§8 of the design doc: one sink, `ALL_TCL`-permissive).
    #[must_use]
    pub fn plain_tcl() -> &'static DialectProfile {
        &PLAIN_TCL
    }

    /// The additive Tk-only ingress profile.
    ///
    /// Tk is intentionally absent from the selectable profile catalogue: it
    /// is a library surface layered onto Tcl rather than a runtime with its
    /// own release semantics. CLI/LSP compatibility inputs still need a
    /// resolved identity, though, so this profile carries `Tk` in its point
    /// while retaining the permissive Tcl behaviour of that ingress.
    #[must_use]
    pub fn tk() -> &'static DialectProfile {
        &TK_PROFILE
    }

    /// Whether this profile is the permissive unknown-dialect fallback
    /// ([`Self::plain_tcl`]) — i.e. the ingest string named no real
    /// dialect. Consumers needing to distinguish this case
    /// (diagnostic labels, "no specific dialect" paths) key off
    /// this instead of special-casing an empty dialect string.
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        std::ptr::eq(self, Self::plain_tcl())
    }

    /// Whether this profile IS the iRules profile — the canonical
    /// "is this iRules?" predicate (pointer identity against the interned
    /// catalogue entry, so aliases are already folded in by resolution).
    #[must_use]
    pub fn is_irules(&self) -> bool {
        std::ptr::eq(self, Self::irules())
    }

    /// Core family selected by this profile's providers. Package surfaces
    /// hosted by the F5 trunk retain that family independently of their
    /// compatible Tcl signature release. This supplies no execution point.
    #[must_use]
    pub fn runtime_core_family(&self) -> Option<Family> {
        match self.vendor_surface {
            Some(SpecProvider::Core(family)) => Some(family),
            Some(SpecProvider::Package("tmsh" | "iapps")) => Some(Family::F5Tcl),
            _ => self
                .grammar_union
                .iter()
                .find_map(|provider| match provider {
                    SpecProvider::Core(family) => Some(*family),
                    SpecProvider::Package(_) => None,
                }),
        }
    }

    /// The F5-family core `expr` grammar behind this catalogue profile, or
    /// `None` for a profile whose runtime core is not on the F5 tree.
    ///
    /// The nine word-form `expr` operators (`and`/`or`/`not`/`contains`/
    /// `starts_with`/`ends_with`/`equals`/`matches_glob`/`matches_regex`)
    /// are an **`f5-tcl` trunk fact**, measured byte-identical in tmsh and
    /// iApp contexts too, not iRules-only
    /// (`docs/design/f5/bigip-irule-parser-measurements.md` §4a) — so every
    /// F5Tcl-cored catalogue profile answers with the family's own
    /// [`ExprGrammar`](crate::model::expr_grammar::ExprGrammar) here and
    /// consumers read the word-operator surface off that table instead of
    /// duplicating rows. The catalogue's own `f5-tmsh` /
    /// `f5-iapps` grammar rows are deliberately retained: only the expr
    /// word-operator acceptance follows the family fact.
    ///
    /// `f5-bigip` is excluded by design: it is a config-schema identity
    /// with no Tcl runtime or expr grammar of its own (its embedded iRules
    /// route through `f5-irules`).
    #[must_use]
    pub fn f5_core_expr_grammar(&self) -> Option<&'static crate::model::ExprGrammar> {
        use crate::model::family::Release;
        match self.runtime_core_family() {
            // The iRules offshoot overrides no expr axis — it answers with
            // the trunk grammar along the fork edge (measurements §4a).
            Some(Family::F5Irules) => {
                Some(crate::model::expr(Family::F5Irules, Release::F5_IRULES_TMM))
            }
            Some(Family::F5Tcl) => Some(crate::model::expr(Family::F5Tcl, Release::F5_TCL_TMOS)),
            _ => None,
        }
    }

    /// The release a compile-time fold evaluates under for this profile: its
    /// [`Self::runtime_base`] where [`Self::evaluation_evidence`] says the base
    /// was measured, and `None` where it was not, so a fold keeps to the answer
    /// every modelled release gives.
    ///
    /// Decided per measured row and never by the profile's name: a Tcl release
    /// has its row in `data/reference-toolchains.tsv`, and a vendor fork has its
    /// own measured-fork note. The release a profile *models* for the
    /// value-transfer routes stays [`Self::runtime_version`].
    #[must_use]
    pub fn evaluation_point(&self) -> Option<TclVersion> {
        match self.evaluation_evidence {
            EvaluationEvidence::ReferenceToolchain | EvaluationEvidence::MeasuredFork { .. } => {
                self.runtime_base
            }
            EvaluationEvidence::Unmeasured => None,
        }
    }

    /// Whether this profile is a Tcl release itself, with an interpreter pinned
    /// for it in `data/reference-toolchains.tsv`, as against a vendor fork or an
    /// environment that models a release.
    #[must_use]
    pub fn is_tcl_release(&self) -> bool {
        self.evaluation_evidence == EvaluationEvidence::ReferenceToolchain
    }

    /// The version-aware *compile-time fold* projection: the profile's
    /// [`Self::evaluation_point`].
    #[must_use]
    pub fn const_fold_version(&self) -> Option<TclVersion> {
        self.evaluation_point()
    }

    /// The [`LibraryPin`] this profile declares for `package`, if any
    /// (§7.1 axis C).
    #[must_use]
    pub fn library_pin(&self, package: &str) -> Option<&'static LibraryPin> {
        self.libraries.iter().find(|pin| pin.package == package)
    }

    /// Whether `package` is **ambient** in this profile's runtime — part of
    /// the modelled interpreter, needing no `package require` (the F5
    /// surfaces, an EDA shell's tool commands). Ambient packages are exempt
    /// from missing-require diagnostics and stay in the ambient completion
    /// / static-highlight surfaces.
    #[must_use]
    pub fn is_ambient_package(&self, package: &str) -> bool {
        self.library_pin(package).is_some_and(|pin| pin.ambient)
    }

    /// Whether this profile can host the Tk desktop library
    /// (`package require Tk`): the plain Tcl versions (which pin Tk) and the
    /// permissive fallback (an unlabelled `tk` / `wish` shell). A closed-world
    /// vendor shell — the F5 surfaces, the EDA shells (packaged vendors with no
    /// Tk pin), bpf — cannot (dialect-profile-model.md §7.2;
    /// eda-library-packages.md). Consumers key Tk offering/acceptance off this
    /// rather than `vendor_surface`, since the EDA shells carry no vendor
    /// surface of their own.
    #[must_use]
    pub fn hosts_tk(&self) -> bool {
        self.is_fallback() || self.library_pin("Tk").is_some()
    }

    /// The version floor this profile guarantees for `package` before any
    /// `package require` is seen, honouring `overrides` for the keyed axes
    /// (D5: keyed pins default to the **oldest supported** version).
    ///
    /// `None` when the package is unpinned (floors come only from explicit
    /// requires, as before) or when a keyed axis has neither an override
    /// nor a default (no data authority yet — permissive).
    #[must_use]
    pub fn library_floor<'a>(
        &self,
        package: &str,
        overrides: &'a LibraryVersionOverrides,
    ) -> Option<&'a str> {
        let pin = self.library_pin(package)?;
        if let LibraryVersion::Keyed(key) = pin.version
            && let Some(pinned) = overrides.get(key)
        {
            return Some(pinned);
        }
        self.library_floor_default(package)
    }

    /// [`Self::library_floor`] with no session overrides — the statically
    /// resolvable floor (`TracksBase` → the runtime base, `Pinned` → the
    /// shipped version, `Keyed` → the oldest-supported default), for
    /// consumers with no override channel (completion, hover, the CLI
    /// snapshot).
    #[must_use]
    pub fn library_floor_default(&self, package: &str) -> Option<&'static str> {
        let pin = self.library_pin(package)?;
        match pin.version {
            LibraryVersion::TracksBase => self.runtime_base.map(TclVersion::as_package_version),
            LibraryVersion::Pinned(version) => Some(version),
            LibraryVersion::Keyed(key) => key.default_version(),
        }
    }

    /// The Tcl version an argument mini-language (`format`/`scan`
    /// conversions, `string is` classes, …) must validate against — the
    /// argument-DSL rung of the granularity ladder (design doc §6.1).
    ///
    /// This is the [`Self::runtime_base`], **raised** to any
    /// `package require Tcl` floor the caller resolved from the file (a
    /// file demanding a newer core than the ambient dialect validates
    /// against what it demands). Permissive (`None`) for the unknown-
    /// dialect fallback and non-Tcl profiles — the validators abstain
    /// rather than guess.
    #[must_use]
    pub fn effective_tcl_version(&self, package_floor: Option<TclVersion>) -> Option<TclVersion> {
        let base = self.runtime_base?;
        Some(package_floor.map_or(base, |floor| base.max(floor)))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod snapshot_identity_tests;
