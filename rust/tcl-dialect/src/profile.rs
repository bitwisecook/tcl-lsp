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

//! The `DialectProfile` catalog — one interned profile per canonical dialect.
//!
//! A profile is resolved from a dialect-name string **once at ingest**
//! (LSP `dialect_for_open`, CLI `effective_dialect`, `detect_dialect`) and
//! the `&'static DialectProfile` is threaded from there, so consumers stop
//! re-parsing dialect strings per query.
//!
//! This is the compositional model of `docs/design/dialect-profile-model.md`:
//! identity, the availability axis — masks, load layers, grammar unions —
//! the behaviour/runtime axis — base versions, octal policy, expr grammar,
//! lexer grammar, the per-dialect predicates — and the versioned-library
//! axis.

use crate::grammar::{
    BraceLineContinuation, BracedVarStyle, EscapeSyntax, ExprCommentStyle, LexerGrammar,
    NumberSyntax,
};
use crate::library::{LibraryPin, LibraryVersion, LibraryVersionOverrides, VersionKey};
use crate::model::{Family, SpecProvider, SurfaceLayer, SurfaceQuery};
use crate::version::{StringCharacterModel, TclVersion, Ternary};

/// Library pins for the 8.4/8.5-era plain Tcl profiles: Tk tracks the
/// embedded base (`wish` 8.5 ships Tk 8.5), Itcl ships the 3.x line.
///
/// tcllib is deliberately NOT pinned here (despite the §7 table's loose
/// "tcllib `TracksBase`"): tcllib packages version on their own axes
/// (`struct 1.4`, not the Tcl release), so a Tcl-base floor would compare
/// apples to oranges — hostability for tcllib is already permissive, and
/// its version floors keep coming from explicit `package require` alone.
const LIBS_TCL84_85: &[LibraryPin] = &[
    LibraryPin {
        package: "Tk",
        version: LibraryVersion::TracksBase,
        ambient: false,
    },
    LibraryPin {
        package: "Itcl",
        version: LibraryVersion::Pinned("3.4"),
        ambient: false,
    },
];

/// Library pins for the 8.6+/9.x plain Tcl profiles (Itcl moves to the
/// 4.x line bundled from 8.6).
const LIBS_TCL86_PLUS: &[LibraryPin] = &[
    LibraryPin {
        package: "Tk",
        version: LibraryVersion::TracksBase,
        ambient: false,
    },
    LibraryPin {
        package: "Itcl",
        version: LibraryVersion::Pinned("4.2"),
        ambient: false,
    },
];

/// The Tcl 8.4 lexing grammar: no `{*}` expansion (TIP 157 is 8.5), the
/// 8.x first-close `${…}` rule, no `expr` comments (TIP 582 is 9.0).
const GRAMMAR_TCL84: LexerGrammar = LexerGrammar {
    expand_syntax: false,
    irules_brace_separator: false,
    brace_line_continuation: BraceLineContinuation::Terminates,
    braced_var: BracedVarStyle::FirstClose,
    array_index: crate::ArrayIndexSyntax::Tcl8,
    script_skips_leading_bom: false,
    expr_comments: ExprCommentStyle::None,
    numbers: NumberSyntax::Tcl84,
    escapes: EscapeSyntax::Tcl84,
};

/// The Tcl 8.5 lexing grammar (plain 8.5, iApps, tmsh, the 8.5-based EDA
/// shells): `{*}` expansion, the 8.x first-close `${…}` rule, no `expr`
/// comments (TIP 582 is 9.0), and 8.4's pre-TIP-388 escape grammar.
const GRAMMAR_TCL85: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    irules_brace_separator: false,
    brace_line_continuation: BraceLineContinuation::Terminates,
    braced_var: BracedVarStyle::FirstClose,
    array_index: crate::ArrayIndexSyntax::Tcl8,
    script_skips_leading_bom: false,
    expr_comments: ExprCommentStyle::None,
    numbers: NumberSyntax::Tcl85,
    escapes: EscapeSyntax::Tcl84,
};

/// The Tcl 8.6 lexing grammar (plain 8.6, Expect, the 8.6-based EDA shells):
/// 8.5's, except that TIP 388 caps `\x` at two digits and adds `\U` — the one
/// axis on which 8.5 and 8.6 differ.
const GRAMMAR_TCL86: LexerGrammar = LexerGrammar {
    escapes: EscapeSyntax::Tcl86,
    ..GRAMMAR_TCL85
};

/// The modern 9.x grammar (also the permissive default): `{*}` expansion,
/// Tcl 9's nesting `${…}` rule, TIP 582 `expr` comments.
const GRAMMAR_TCL9X: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    irules_brace_separator: false,
    brace_line_continuation: BraceLineContinuation::Terminates,
    braced_var: BracedVarStyle::Tcl9Nesting,
    array_index: crate::ArrayIndexSyntax::Tcl9,
    script_skips_leading_bom: true,
    expr_comments: ExprCommentStyle::Hash,
    numbers: NumberSyntax::Tcl90,
    escapes: EscapeSyntax::Tcl90,
};

/// The `f5-tcl` **trunk** lexing grammar: a Tcl 8.4 base (no `{*}`, no
/// `expr` comments) plus the two measured fork axes — the implicit word
/// break (R-rules) and the brace-line continuation (N-rules), both
/// live-measured on TMM 21.1.0.1 with same-host stock controls
/// (`docs/design/bigip-irule-parser-measurements.md` §1-§3). Measured
/// **byte-identical in all three BIG-IP execution contexts** (§4a):
/// TMM iRules, `IAppImplementation`, and tmsh `cli script` all reproduce
/// the R-rules, the N-rules, and the inert `{*}`, so this one grammar
/// serves `f5-irules`, `f5-iapps`, and `f5-tmsh` alike — the iRules
/// offshoot overrides no lexical axis.
const GRAMMAR_F5_TCL: LexerGrammar = LexerGrammar {
    expand_syntax: false,
    irules_brace_separator: true,
    brace_line_continuation: BraceLineContinuation::Continues,
    braced_var: BracedVarStyle::FirstClose,
    array_index: crate::ArrayIndexSyntax::Tcl8,
    script_skips_leading_bom: false,
    expr_comments: ExprCommentStyle::None,
    numbers: NumberSyntax::Tcl84,
    escapes: EscapeSyntax::Tcl84,
};

/// One filename extension a dialect owns, with its human-facing name —
/// the catalog analogue of a `SpecTcl` pack's
/// `file_extension upf -name {Unified Power Format}` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
/// The behaviour fields are **derived data fixed at catalog-construction
/// time** from the §7 table of the design doc (`signature_base` /
/// `runtime_base` / the vendor surface drive the rest); the derivation rules
/// (§7.1) are enforced by this module's invariant tests so the hand-laid
/// values can never drift from the model.
#[derive(Debug)]
pub struct DialectProfile {
    /// The canonical dialect name (`"tcl8.6"`, `"f5-irules"`, …). Stable:
    /// this is the string that round-trips through configuration
    /// (`tclLsp.selectDialect`, `folderDialects`), the registry-dump JSON
    /// schema, and the editor catalogues.
    pub name: &'static str,
    /// Legacy / editor spellings that resolve to this profile
    /// (`"irules"` → `f5-irules`). Resolution through [`Self::find`] (and
    /// the environment seam built on it) canonicalises them, so profile
    /// predicates can never disagree with the canonical spelling the way
    /// the string-keyed tables used to (design doc §2.4).
    pub aliases: &'static [&'static str],
    /// The full human-facing name shown in settings menus and pickers
    /// (`"Synopsys EDA Tcl"`, `"Tcl 8.6"`). The catalog is the single
    /// source for editor presentation: `cargo xtask gen-editor-dialects`
    /// projects this into every editor's dialect list, so adding a
    /// profile ships its label everywhere at once.
    pub display_name: &'static str,
    /// A compact label for tight UI (the compiler-explorer dropdown,
    /// status bars): `"Synopsys EDA"`, `"iRules"`. Never empty — repeats
    /// [`Self::display_name`] where no shorter form exists.
    pub short_name: &'static str,
    /// The editor language id this dialect's files open under, where the
    /// editors keep a dedicated language (`"tcl-synopsys"`, `"tcl84"`).
    /// Undotted by contract — VS Code splits `configurationDefaults`
    /// override keys on `.` (issue #1122). `None` = no dedicated editor
    /// language; the dialect's files (if any) ride the plain `tcl`
    /// language and server-side detection routes them.
    pub editor_language_id: Option<&'static str>,
    /// Whole *basenames* this dialect owns, matched instead of an extension
    /// (`bigip.conf`, `bigip_base.conf`). Lower-case, compared
    /// case-insensitively against a path's last component, unique across the
    /// catalog.
    ///
    /// The second axis of file recognition, and it needs its own field
    /// because the file it names has no useful extension: a bare `.conf`
    /// suffix belongs to every unrelated config file on the machine, so
    /// `f5-bigip` can only claim `bigip.conf` by *name*. Before this existed
    /// the set lived in `tcl_lsp_core::bigip`, which the editors could not
    /// read — so VS Code contributed no `filenames` at all and a `bigip.conf`
    /// never associated, while the Sublime grammar's comment claimed the
    /// opposite (issue #1625).
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
    /// Lower-case, no leading dot, unique across the catalog. `SpecTcl`
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
    /// Pinned against [`Self::vendor_surface`] by
    /// `surface_packages_carry_the_vendor_surface`, so the two cannot drift.
    pub surface_packages: &'static [&'static str],

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
    /// (the permissive fallback). Resolution through the catalog means
    /// alias spellings (`irules`) filter exactly like the canonical name
    /// — the old string-keyed table silently applied no filter to them.
    pub help_terms: &'static [&'static str],
}

/// Profile equality **is** pointer identity, as the type's contract states:
/// every profile a consumer holds came from the interned catalog (or the
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

/// The catalog: one profile per canonical dialect, in
/// [`KNOWN_DIALECTS`](crate::KNOWN_DIALECTS) (sorted) order.
///
/// Surface and behaviour values follow the per-dialect table in
/// `docs/design/dialect-profile-model.md` §7.
static CATALOG: [DialectProfile; 19] = [
    // bpf embeds a genuine Tcl 9.0 (design doc D7): 9.0 runtime semantics —
    // decimal leading zeros, 9.0 expr grammar, the nesting `${…}` rule —
    // and a precise point: 9.0 core plus the bpf surface resolve, while
    // 8.x-only relics (removed at the 9.0 boundary) are correctly unknown.
    DialectProfile {
        name: "bpf",
        aliases: &[],
        display_name: "BPF",
        short_name: "BPF",
        editor_language_id: None,
        filenames: &[],
        file_extensions: &[],
        vendor_surface: Some(SpecProvider::Package("bpf")),
        surface_packages: &["bpf"],
        base_layers: &[SurfaceLayer::Package("bpf")],
        grammar_union: &[
            SpecProvider::Core(Family::Tcl),
            SpecProvider::Package("bpf"),
        ],
        version_ceiling: Some(TclVersion::V9_0),
        signature_base: Some(TclVersion::V9_0),
        runtime_base: Some(TclVersion::V9_0),
        leading_zero_is_octal: Ternary::No,
        expr_grammar_base: Some(TclVersion::V9_0),
        grammar: GRAMMAR_TCL9X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
        libraries: &[],
        help_terms: &["bpf", "ebpf"],
    },
    DialectProfile {
        name: "cadence-eda-tcl",
        aliases: &[],
        display_name: "Cadence EDA Tcl",
        short_name: "Cadence EDA",
        editor_language_id: Some("tcl-cadence"),
        filenames: &[],
        file_extensions: &[DialectFileExtension {
            extension: "globals",
            display_name: "Innovus/Genus Globals",
        }],
        vendor_surface: None,
        surface_packages: &[],
        // Innovus/Genus embed an 8.4-safe Tcl core: real Cadence scripts
        // systematically avoid dict/lassign/`{*}` (the 8.5 additions), and no
        // public source pins a newer interpreter (owner decision; the July-2026
        // EDA study). So no `{*}` expansion, no `::tcl::mathop` heads, no TclOO.
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.4")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_4),
        signature_base: Some(TclVersion::V8_4),
        runtime_base: Some(TclVersion::V8_4),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_4),
        grammar: GRAMMAR_TCL84,
        operators_as_commands: false,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_4,
        libraries: &[
            LibraryPin {
                package: "sdc",
                version: LibraryVersion::Keyed(VersionKey::SdcVersion),
                ambient: true,
            },
            LibraryPin {
                package: "upf",
                version: LibraryVersion::Keyed(VersionKey::UpfVersion),
                ambient: true,
            },
            LibraryPin {
                package: "cadence-genus",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "cadence-common",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "cadence-innovus",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "cadence-xcelium",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
        ],
        help_terms: &[
            "cadence",
            "genus",
            "innovus",
            "tempus",
            "xcelium",
            "encounter",
        ],
    },
    // Expect embeds Tcl 8.6 — including the 8.x first-close `${…}` rule,
    // which the old string-keyed lexer table missed (it fell through to
    // the modern-9.x default).
    DialectProfile {
        name: "expect",
        aliases: &[],
        display_name: "Expect",
        short_name: "Expect",
        editor_language_id: Some("tcl-expect"),
        filenames: &[],
        file_extensions: &[
            DialectFileExtension {
                extension: "exp",
                display_name: "Expect Script",
            },
            DialectFileExtension {
                extension: "expect",
                display_name: "Expect Script",
            },
        ],
        vendor_surface: Some(SpecProvider::Package("expect")),
        surface_packages: &["expect"],
        base_layers: &[SurfaceLayer::Package("expect")],
        grammar_union: &[
            SpecProvider::Core(Family::Tcl),
            SpecProvider::Package("expect"),
        ],
        version_ceiling: Some(TclVersion::V8_6),
        signature_base: Some(TclVersion::V8_6),
        runtime_base: Some(TclVersion::V8_6),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_6),
        grammar: GRAMMAR_TCL86,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_6,
        libraries: &[LibraryPin {
            package: "Expect",
            version: LibraryVersion::Pinned("5.45.4"),
            ambient: true,
        }],
        help_terms: &["expect", "spawn", "interact"],
    },
    // f5-bigip is a config parser, not a Tcl surface; it has no command
    // pack, no Tcl runtime (behaviour axis inert — §11.1), and no expr
    // grammar. It is first-class as *identity only* (D8): the bare
    // the `bigip` surface keys the profile and its versioned schema
    // library — BIG-IP config documents route to the tcl-bigip validator,
    // never the Tcl analyser, so this is not a Tcl-availability surface.
    DialectProfile {
        name: "f5-bigip",
        aliases: &[],
        display_name: "F5 BIG-IP",
        short_name: "BIG-IP",
        editor_language_id: Some("tcl-bigip"),
        // The canonical BIG-IP configuration basenames. Deliberately *not*
        // the `.conf` extension: that suffix belongs to every unrelated
        // config file, so these files can only be claimed by name.
        filenames: &[
            "bigip.conf",
            "bigip_base.conf",
            "bigip_gtm.conf",
            "bigip_script.conf",
            "bigip_user.conf",
        ],
        file_extensions: &[DialectFileExtension {
            extension: "scf",
            display_name: "BIG-IP Single Configuration File",
        }],
        vendor_surface: Some(SpecProvider::Package("bigip")),
        surface_packages: &["bigip"],
        base_layers: &[SurfaceLayer::Package("bigip")],
        grammar_union: &[SpecProvider::Package("bigip")],
        version_ceiling: None,
        signature_base: None,
        runtime_base: None,
        leading_zero_is_octal: Ternary::Inert,
        expr_grammar_base: None,
        grammar: GRAMMAR_TCL9X,
        operators_as_commands: false,
        tcloo: false,
        has_fixed_ensembles: true,
        vm_runtime_version: TclVersion::V9_0,
        libraries: &[LibraryPin {
            package: "f5-bigip-schema",
            version: LibraryVersion::Keyed(VersionKey::BigipVersion),
            ambient: true,
        }],
        help_terms: &["bigip", "big-ip", "bigip.conf", "f5", "ltm", "gtm"],
    },
    // iApps ride the `f5-tcl` trunk (fork of Tcl at 8.4.6), NOT a real
    // 8.5 host: the 8.5 hypothesis is measured and falsified
    // (`docs/design/bigip-irule-parser-measurements.md` §4a) —
    // `IAppImplementation` reports patchlevel 8.4.6, fails every 8.5
    // discriminator (`dict`, `lassign`, `apply`, `0b101`), and carries
    // the full trunk grammar (R-rules, N-rules, inert `{*}`, expr word
    // operators) byte-identical to TMM. `::tcl::mathop` is measured
    // absent, so operator heads are not commands. Environment deltas
    // (working `exec`, large `package names`, 32-bit `tcl_platform`) are
    // non-grammatical and live on the environment, not here.
    DialectProfile {
        name: "f5-iapps",
        aliases: &[],
        display_name: "F5 iApps",
        short_name: "iApps",
        editor_language_id: Some("tcl-iapp"),
        filenames: &[],
        file_extensions: &[
            DialectFileExtension {
                extension: "iapp",
                display_name: "F5 iApp Template",
            },
            DialectFileExtension {
                extension: "iappimpl",
                display_name: "F5 iApp Implementation",
            },
            DialectFileExtension {
                extension: "impl",
                display_name: "F5 iApp Implementation",
            },
        ],
        vendor_surface: Some(SpecProvider::Package("iapps")),
        surface_packages: &["iapps"],
        base_layers: &[SurfaceLayer::Package("iapps")],
        grammar_union: &[
            SpecProvider::Core(Family::Tcl),
            SpecProvider::Package("iapps"),
        ],
        version_ceiling: Some(TclVersion::V8_4),
        signature_base: Some(TclVersion::V8_4),
        runtime_base: Some(TclVersion::V8_4),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_4),
        grammar: GRAMMAR_F5_TCL,
        operators_as_commands: false,
        tcloo: false,
        has_fixed_ensembles: true,
        vm_runtime_version: TclVersion::V8_4,
        libraries: &[LibraryPin {
            package: "f5-iapps-cmds",
            version: LibraryVersion::Keyed(VersionKey::BigipVersion),
            ambient: true,
        }],
        help_terms: &["iapps", "iapp", "f5", "big-ip"],
    },
    // iRules is a genuine embedded Tcl 8.4.6 whose availability is stated
    // per spec: the point names the family and no release, so a command is
    // available iff its own surface names iRules. The F5 command surface
    // does; the iRules-enabled Tcl core names both Tcl and iRules; a
    // K36322151 sandbox-banned command names only Tcl and so is simply
    // absent — no subtractive disable list. Naming a release here would
    // only re-admit 8.x-only specs the TMM build never had. Signature and
    // runtime base are both 8.4 (D3) and math operators are not command
    // heads.
    DialectProfile {
        name: "f5-irules",
        aliases: &["irules", "tcl-irule"],
        display_name: "F5 iRules",
        short_name: "iRules",
        editor_language_id: Some("tcl-irule"),
        filenames: &[],
        file_extensions: &[
            DialectFileExtension {
                extension: "irul",
                display_name: "F5 iRule",
            },
            DialectFileExtension {
                extension: "irule",
                display_name: "F5 iRule",
            },
            DialectFileExtension {
                extension: "irules",
                display_name: "F5 iRule",
            },
        ],
        vendor_surface: Some(SpecProvider::Core(Family::F5Irules)),
        surface_packages: &[],
        base_layers: &[SurfaceLayer::Core(Family::F5Irules, "")],
        grammar_union: &[SpecProvider::Core(Family::F5Irules)],
        version_ceiling: Some(TclVersion::V8_4),
        signature_base: Some(TclVersion::V8_4),
        runtime_base: Some(TclVersion::V8_4),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_4),
        grammar: GRAMMAR_F5_TCL,
        operators_as_commands: false,
        tcloo: false,
        has_fixed_ensembles: true,
        vm_runtime_version: TclVersion::V8_4,
        libraries: &[LibraryPin {
            package: "f5-irules-cmds",
            version: LibraryVersion::Keyed(VersionKey::BigipVersion),
            ambient: true,
        }],
        help_terms: &["irules", "irule", "f5", "big-ip", "tmm", "event"],
    },
    // f5-tmsh rides the `f5-tcl` trunk (fork of Tcl at 8.4.6): the
    // previous 8.5/8.5.13 claims are measured and falsified
    // (`docs/design/bigip-irule-parser-measurements.md` §4a) — a
    // `TmshCliScript` reports patchlevel 8.4.6 and reproduces the entire
    // trunk grammar (R-rules, N-rules, inert `{*}`, expr word operators)
    // identically to TMM, and `::tcl::mathop` is measured absent. It is
    // first-class (D8): the tmsh shell hosts the trunk interpreter plus
    // the `tmsh::` surface (shared spec data with iApps, tagged
    // `IAPPS|TMSH`). Environment deltas (working `exec`, empty
    // `tcl_platform`, no `tcl_patchLevel`, `info vartype`) live on the
    // environment, not here.
    DialectProfile {
        name: "f5-tmsh",
        aliases: &[],
        display_name: "F5 tmsh Scripts",
        short_name: "tmsh",
        editor_language_id: Some("tcl-tmsh"),
        filenames: &[],
        file_extensions: &[DialectFileExtension {
            extension: "tmsh",
            display_name: "F5 tmsh Script",
        }],
        vendor_surface: Some(SpecProvider::Package("tmsh")),
        surface_packages: &["tmsh"],
        base_layers: &[SurfaceLayer::Package("tmsh")],
        grammar_union: &[
            SpecProvider::Core(Family::Tcl),
            SpecProvider::Package("tmsh"),
        ],
        version_ceiling: Some(TclVersion::V8_4),
        signature_base: Some(TclVersion::V8_4),
        runtime_base: Some(TclVersion::V8_4),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_4),
        grammar: GRAMMAR_F5_TCL,
        operators_as_commands: false,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_4,
        libraries: &[LibraryPin {
            package: "f5-tmsh-cmds",
            version: LibraryVersion::Keyed(VersionKey::BigipVersion),
            ambient: true,
        }],
        help_terms: &["tmsh", "f5", "big-ip", "bigip"],
    },
    DialectProfile {
        name: "intel-quartus-eda-tcl",
        aliases: &[],
        display_name: "Intel Quartus EDA Tcl",
        short_name: "Intel Quartus",
        editor_language_id: Some("tcl-quartus"),
        filenames: &[],
        file_extensions: &[
            DialectFileExtension {
                extension: "qsf",
                display_name: "Quartus Settings File",
            },
            DialectFileExtension {
                extension: "qpf",
                display_name: "Quartus Project File",
            },
            DialectFileExtension {
                extension: "qip",
                display_name: "Quartus IP File",
            },
        ],
        vendor_surface: None,
        surface_packages: &[],
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.5")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL85,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_5,
        libraries: &[
            LibraryPin {
                package: "sdc",
                version: LibraryVersion::Keyed(VersionKey::SdcVersion),
                ambient: true,
            },
            LibraryPin {
                package: "upf",
                version: LibraryVersion::Keyed(VersionKey::UpfVersion),
                ambient: true,
            },
            LibraryPin {
                package: "quartus-project",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "quartus-flow",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "quartus-sta",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "quartus-sdc-ext",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "quartus-report",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "quartus-device",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "quartus-misc",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
        ],
        help_terms: &["quartus", "intel", "altera", "fpga", "quartus_sh"],
    },
    DialectProfile {
        name: "mentor-eda-tcl",
        aliases: &[],
        display_name: "Mentor EDA Tcl",
        short_name: "Mentor EDA",
        editor_language_id: Some("tcl-mentor"),
        filenames: &[],
        file_extensions: &[DialectFileExtension {
            extension: "do",
            display_name: "ModelSim/Questa Do Script",
        }],
        vendor_surface: None,
        surface_packages: &[],
        // Modern Questa/ModelSim embeds Tcl 8.6 (owner decision; the July-2026
        // EDA study — bundled `tcl8.6` library paths). Older ModelSim shipped
        // 8.4/8.5, but the current-tool default is 8.6: TclOO + the 8.6 core.
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.6")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_6),
        signature_base: Some(TclVersion::V8_6),
        runtime_base: Some(TclVersion::V8_6),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_6),
        grammar: GRAMMAR_TCL86,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_6,
        libraries: &[
            LibraryPin {
                package: "sdc",
                version: LibraryVersion::Keyed(VersionKey::SdcVersion),
                ambient: true,
            },
            LibraryPin {
                package: "upf",
                version: LibraryVersion::Keyed(VersionKey::UpfVersion),
                ambient: true,
            },
            LibraryPin {
                package: "questa",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "questa-formal",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "calibre",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
        ],
        help_terms: &["mentor", "siemens", "modelsim", "questa", "calibre", "vsim"],
    },
    DialectProfile {
        name: "microchip-libero-eda-tcl",
        aliases: &[],
        display_name: "Microchip Libero EDA Tcl",
        short_name: "Microchip Libero",
        editor_language_id: Some("tcl-microchip"),
        filenames: &[],
        file_extensions: &[],
        vendor_surface: None,
        surface_packages: &[],
        // Libero SoC's embedded interpreter is an 8.5-era core (the v11.x
        // reference documents plain-8.5 idiom and none of the 8.6 additions;
        // no public source pins a newer interpreter). Judgement call pending
        // owner confirmation against a live install — the conservative choice
        // mirrors the Quartus/Xilinx 8.5 base.
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.5")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL85,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_5,
        libraries: &[
            LibraryPin {
                package: "sdc",
                version: LibraryVersion::Keyed(VersionKey::SdcVersion),
                ambient: true,
            },
            LibraryPin {
                package: "upf",
                version: LibraryVersion::Keyed(VersionKey::UpfVersion),
                ambient: true,
            },
            LibraryPin {
                package: "libero",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
        ],
        help_terms: &[
            "microchip",
            "microsemi",
            "actel",
            "libero",
            "smartfusion",
            "igloo",
            "proasic",
        ],
    },
    // SpecTcl — the `.tclspec` spec-pack authoring DSL (`spec-packs.md`).
    // A pack is an ordinary Tcl script read from the CST and never executed;
    // the only Tcl that is ever *evaluated* is a hook body, on our own VM, so
    // the runtime half of this profile is plain Tcl 9.0. What makes it a
    // dialect at all is the availability half: the statement words
    // (`speclib` / `command` / `option` / `arg` / …) are a command surface
    // that must exist inside a pack and nowhere else.
    DialectProfile {
        name: "spectcl",
        aliases: &["tcl-spec", "tclspec"],
        display_name: "SpecTcl",
        short_name: "SpecTcl",
        editor_language_id: Some("tclspec"),
        filenames: &[],
        file_extensions: &[DialectFileExtension {
            extension: "tclspec",
            display_name: "SpecTcl Command Pack",
        }],
        vendor_surface: Some(SpecProvider::Package("spectcl")),
        surface_packages: &["spectcl"],
        base_layers: &[SurfaceLayer::Package("spectcl")],
        grammar_union: &[
            SpecProvider::Core(Family::Tcl),
            SpecProvider::Package("spectcl"),
        ],
        version_ceiling: Some(TclVersion::V9_0),
        signature_base: Some(TclVersion::V9_0),
        runtime_base: Some(TclVersion::V9_0),
        leading_zero_is_octal: Ternary::No,
        expr_grammar_base: Some(TclVersion::V9_0),
        grammar: GRAMMAR_TCL9X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
        libraries: &[],
        help_terms: &["spectcl", "speclib", "tclspec"],
    },
    // SslicTcl — the `.sslictcl` TLS-assurance declaration DSL (#1543).
    // A document is an ordinary Tcl script read from the CST and never
    // executed: the loader evaluates nothing, not even a `predicate` body,
    // which it retains verbatim. Like SpecTcl this is an *environment* over
    // Tcl 9.0 rather than a grammar axis — what makes it a dialect is the
    // availability half, the declaration vocabulary (`certificate` /
    // `endpoint` / `policy` / …) that must exist inside a `.sslictcl`
    // document and nowhere else.
    DialectProfile {
        name: "sslictcl",
        aliases: &["sslic-tcl", "tls-sslictcl"],
        display_name: "SslicTcl",
        short_name: "SslicTcl",
        editor_language_id: Some("sslictcl"),
        filenames: &[],
        file_extensions: &[DialectFileExtension {
            extension: "sslictcl",
            display_name: "SslicTcl TLS Declaration",
        }],
        vendor_surface: Some(SpecProvider::Package("sslictcl")),
        surface_packages: &["sslictcl"],
        base_layers: &[SurfaceLayer::Package("sslictcl")],
        grammar_union: &[
            SpecProvider::Core(Family::Tcl),
            SpecProvider::Package("sslictcl"),
        ],
        version_ceiling: Some(TclVersion::V9_0),
        signature_base: Some(TclVersion::V9_0),
        runtime_base: Some(TclVersion::V9_0),
        leading_zero_is_octal: Ternary::No,
        // The declaration vocabulary evaluates nothing, so no `expr` is ever
        // run from a `.sslictcl` document. The field is still V9_0 because it
        // is derived, not chosen: `expr_grammar_base == runtime_base` is a
        // profile invariant, and base Tcl stays loaded underneath the
        // declaration surface.
        expr_grammar_base: Some(TclVersion::V9_0),
        grammar: GRAMMAR_TCL9X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
        libraries: &[],
        help_terms: &["sslictcl", "tls", "certificate", "endpoint"],
    },
    DialectProfile {
        name: "synopsys-eda-tcl",
        aliases: &[],
        display_name: "Synopsys EDA Tcl",
        short_name: "Synopsys EDA",
        editor_language_id: Some("tcl-synopsys"),
        filenames: &[],
        file_extensions: &[
            DialectFileExtension {
                extension: "sdc",
                display_name: "Synopsys Design Constraints",
            },
            DialectFileExtension {
                extension: "upf",
                display_name: "Unified Power Format",
            },
        ],
        vendor_surface: None,
        surface_packages: &[],
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.6")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_6),
        signature_base: Some(TclVersion::V8_6),
        runtime_base: Some(TclVersion::V8_6),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_6),
        grammar: GRAMMAR_TCL86,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_6,
        libraries: &[
            LibraryPin {
                package: "sdc",
                version: LibraryVersion::Keyed(VersionKey::SdcVersion),
                ambient: true,
            },
            LibraryPin {
                package: "upf",
                version: LibraryVersion::Keyed(VersionKey::UpfVersion),
                ambient: true,
            },
            LibraryPin {
                package: "synopsys-dc",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "synopsys-pt",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "synopsys-icc2",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "synopsys-fm",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
            LibraryPin {
                package: "synopsys",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
        ],
        help_terms: &[
            "synopsys",
            "dc_shell",
            "design_compiler",
            "primetime",
            "icc2",
            "formality",
        ],
    },
    DialectProfile {
        name: "tcl8.4",
        aliases: &[],
        display_name: "Tcl 8.4",
        short_name: "Tcl 8.4",
        editor_language_id: Some("tcl84"),
        filenames: &[],
        file_extensions: &[],
        vendor_surface: None,
        surface_packages: &[],
        // The version layer loads no specs; it records which release the
        // registry is, which its own introspection reads
        // (`CommandRegistry::leading_zero_is_octal`).
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.4")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_4),
        signature_base: Some(TclVersion::V8_4),
        runtime_base: Some(TclVersion::V8_4),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_4),
        grammar: GRAMMAR_TCL84,
        // The `::tcl::mathop` operator-command heads (and the whole `::tcl::`
        // namespace they live in) are TIP 174, added in Tcl 8.5 — plain 8.4
        // has no `::tcl` namespace at all, so operators are never command
        // heads there, matching iRules' embedded-8.4 reasoning (§9).
        operators_as_commands: false,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_4,
        libraries: LIBS_TCL84_85,
        help_terms: &["tcl", "tk"],
    },
    DialectProfile {
        name: "tcl8.5",
        aliases: &[],
        display_name: "Tcl 8.5",
        short_name: "Tcl 8.5",
        editor_language_id: Some("tcl85"),
        filenames: &[],
        file_extensions: &[],
        vendor_surface: None,
        surface_packages: &[],
        // The version layer loads no specs; it records which release the
        // registry is, which its own introspection reads
        // (`CommandRegistry::leading_zero_is_octal`).
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.5")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL85,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_5,
        libraries: LIBS_TCL84_85,
        help_terms: &["tcl", "tk"],
    },
    DialectProfile {
        name: "tcl8.6",
        aliases: &[],
        display_name: "Tcl 8.6",
        short_name: "Tcl 8.6",
        editor_language_id: Some("tcl86"),
        filenames: &[],
        file_extensions: &[],
        vendor_surface: None,
        surface_packages: &[],
        // The version layer loads no specs; it records which release the
        // registry is, which its own introspection reads
        // (`CommandRegistry::leading_zero_is_octal`).
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.6")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_6),
        signature_base: Some(TclVersion::V8_6),
        runtime_base: Some(TclVersion::V8_6),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_6),
        grammar: GRAMMAR_TCL86,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_6,
        libraries: LIBS_TCL86_PLUS,
        help_terms: &["tcl", "tk"],
    },
    DialectProfile {
        name: "tcl9.0",
        aliases: &[],
        display_name: "Tcl 9.0",
        short_name: "Tcl 9.0",
        editor_language_id: Some("tcl90"),
        filenames: &[],
        file_extensions: &[],
        vendor_surface: None,
        surface_packages: &[],
        // The version layer loads no specs; it records which release the
        // registry is, which its own introspection reads
        // (`CommandRegistry::leading_zero_is_octal`).
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "9.0")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V9_0),
        signature_base: Some(TclVersion::V9_0),
        runtime_base: Some(TclVersion::V9_0),
        leading_zero_is_octal: Ternary::No,
        expr_grammar_base: Some(TclVersion::V9_0),
        grammar: GRAMMAR_TCL9X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
        libraries: LIBS_TCL86_PLUS,
        help_terms: &["tcl", "tk"],
    },
    // The 9.0-and-later windows already give 9.1 its 9.0 inheritance, so
    // naming the exact release here keeps per-version gating precise.
    DialectProfile {
        name: "tcl9.1",
        aliases: &[],
        display_name: "Tcl 9.1",
        short_name: "Tcl 9.1",
        editor_language_id: Some("tcl91"),
        filenames: &[],
        file_extensions: &[],
        vendor_surface: None,
        surface_packages: &[],
        // The version layer loads no specs; it records which release the
        // registry is, which its own introspection reads
        // (`CommandRegistry::leading_zero_is_octal`).
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "9.1")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V9_1),
        signature_base: Some(TclVersion::V9_1),
        runtime_base: Some(TclVersion::V9_1),
        leading_zero_is_octal: Ternary::No,
        expr_grammar_base: Some(TclVersion::V9_1),
        grammar: GRAMMAR_TCL9X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_1,
        libraries: LIBS_TCL86_PLUS,
        help_terms: &["tcl", "tk"],
    },
    DialectProfile {
        name: "xilinx-eda-tcl",
        aliases: &[],
        display_name: "Xilinx EDA Tcl",
        short_name: "Xilinx EDA",
        editor_language_id: Some("tcl-xilinx"),
        filenames: &[],
        file_extensions: &[DialectFileExtension {
            extension: "xdc",
            display_name: "Xilinx Design Constraints",
        }],
        vendor_surface: None,
        surface_packages: &[],
        base_layers: &[SurfaceLayer::Core(Family::Tcl, "8.5")],
        grammar_union: &[SpecProvider::Core(Family::Tcl)],
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL85,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V8_5,
        libraries: &[
            LibraryPin {
                package: "sdc",
                version: LibraryVersion::Keyed(VersionKey::SdcVersion),
                ambient: true,
            },
            LibraryPin {
                package: "upf",
                version: LibraryVersion::Keyed(VersionKey::UpfVersion),
                ambient: true,
            },
            LibraryPin {
                package: "vivado",
                version: LibraryVersion::Keyed(VersionKey::ToolVersion),
                ambient: true,
            },
        ],
        help_terms: &["xilinx", "vivado", "vitis", "amd", "fpga", "ise"],
    },
];

/// The single sink for every unparseable / typo / unset dialect string.
/// Deliberately permissive so an unknown dialect never flags valid code
/// (design doc §8): full `ALL_TCL` availability, nothing disabled, no pack,
/// inert octal policy, no expr-grammar opinion, modern lexing grammar.
static PLAIN_TCL: DialectProfile = DialectProfile {
    name: "tcl",
    aliases: &[],
    display_name: "Tcl",
    short_name: "Tcl",
    editor_language_id: None,
    filenames: &[],
    file_extensions: &[],
    vendor_surface: None,
    surface_packages: &[],
    base_layers: &[],
    grammar_union: &[SpecProvider::Core(Family::Tcl)],
    version_ceiling: None,
    signature_base: None,
    runtime_base: None,
    leading_zero_is_octal: Ternary::Inert,
    expr_grammar_base: None,
    grammar: GRAMMAR_TCL9X,
    operators_as_commands: true,
    tcloo: true,
    has_fixed_ensembles: false,
    vm_runtime_version: TclVersion::V9_0,
    libraries: &[],
    help_terms: &[],
};

/// Set-only `tk` ingress: modern Tcl behaviour, plus Tk in the point.
/// This is deliberately not part of [`DialectProfile::all`] or
/// [`DialectProfile::find`].
static TK_PROFILE: DialectProfile = DialectProfile {
    name: "tk",
    aliases: &[],
    display_name: "Tk",
    short_name: "Tk",
    editor_language_id: None,
    filenames: &[],
    file_extensions: &[],
    vendor_surface: None,
    surface_packages: &["Tk"],
    base_layers: &[],
    grammar_union: &[SpecProvider::Core(Family::Tcl), SpecProvider::Package("Tk")],
    version_ceiling: None,
    signature_base: None,
    runtime_base: None,
    leading_zero_is_octal: Ternary::Inert,
    expr_grammar_base: None,
    grammar: GRAMMAR_TCL9X,
    operators_as_commands: true,
    tcloo: true,
    has_fixed_ensembles: false,
    vm_runtime_version: TclVersion::V9_0,
    libraries: LIBS_TCL86_PLUS,
    help_terms: &["tk"],
};

impl DialectProfile {
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

    /// Whether this dialect keeps the TIP 278 namespace-scope global
    /// variable fallback (Tcl 8.x yes, 9.0+ no).
    ///
    /// Follows the *runtime* base release, so a vendor shell inherits its
    /// embedded core's behaviour — an iRules script runs on a real 8.4. A
    /// profile with no documented base gets `false`: without evidence of an
    /// 8.x core, the stricter 9.0 reading avoids inventing cross-scope
    /// references.
    #[must_use]
    pub fn namespace_var_global_fallback(&self) -> bool {
        self.runtime_base
            .is_some_and(|base| base < crate::version::TclVersion::V9_0)
    }

    /// The point this profile asks surface questions at — which core
    /// family and release it is, and which vendor package it carries.
    ///
    /// The two halves stay apart — which language, and which packages —
    /// because they answer different questions. A profile whose vendor
    /// surface *is* a core family — iRules — asks as that family and
    /// carries no Tcl release: a spec available across the Tcl ladder is
    /// not thereby an iRules spec.
    ///
    /// A family whose ladder is not keyed by version (the iRules `tmos`
    /// line) asks about its whole ladder: there is no release to name.
    #[must_use]
    pub fn surface_query(&self) -> SurfaceQuery<'static> {
        if let Some(SpecProvider::Core(family)) = self.vendor_surface {
            return SurfaceQuery::any_release(family);
        }
        SurfaceQuery {
            core: match self.signature_base {
                Some(version) => Some((Family::Tcl, Some(version.version_string()))),
                // No pinned release, but still a Tcl surface: the permissive
                // `tcl` sink and the `tk` ingress profile ask about the whole
                // ladder. A profile whose grammar names no core family
                // (`f5-bigip`) has no Tcl surface to ask about.
                None => self
                    .grammar_union
                    .contains(&SpecProvider::Core(Family::Tcl))
                    .then_some((Family::Tcl, None)),
            },
            packages: self.surface_packages,
        }
    }

    /// The release this profile's *runtime* behaviour follows, if it names one.
    ///
    /// A thin name over [`Self::runtime_base`], but the name is the point: it
    /// is the sanctioned way to get from a profile to a [`TclVersion`], so the
    /// step reads the same everywhere and a future rule (a vendor surface
    /// that overrides the base, say) has one place to land.
    #[must_use]
    pub fn runtime_version(&self) -> Option<TclVersion> {
        self.runtime_base
    }

    /// The string/character model of the release this profile runs.
    ///
    /// Collapses a three-step composition — profile → `runtime_base` →
    /// [`TclVersion::string_character_model`] — that had four independent
    /// copies across the compiler, the analyser and the explorer, reached
    /// through three different spellings of "get the profile". Each copy was
    /// free to differ in what it did with a profile that names no release.
    #[must_use]
    pub fn character_model(&self) -> Option<StringCharacterModel> {
        self.runtime_version()
            .map(TclVersion::string_character_model)
    }

    /// The full catalog of canonical dialect profiles, in sorted-name order
    /// (the [`KNOWN_DIALECTS`](crate::KNOWN_DIALECTS) order). Excludes the
    /// [`Self::plain_tcl`] fallback — it is a resolution sink, not a
    /// selectable dialect.
    #[must_use]
    pub fn all() -> &'static [DialectProfile] {
        &CATALOG
    }

    /// Look up a **catalogue** profile by canonical name or registered
    /// alias — `None` for anything else, the `tk` ingress and the lenient
    /// sink included.
    ///
    /// This is the one catalogue lookup left on the profile (P1-G): it is
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
    /// dialect. Consumers that used to special-case an empty dialect
    /// string (diagnostic labels, "no specific dialect" paths) key off
    /// this instead.
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        std::ptr::eq(self, Self::plain_tcl())
    }

    /// Whether this profile IS the iRules profile — the canonical
    /// "is this iRules?" predicate (pointer identity against the interned
    /// catalog entry, so aliases are already folded in by resolution).
    #[must_use]
    pub fn is_irules(&self) -> bool {
        std::ptr::eq(self, Self::irules())
    }

    /// The F5-family core `expr` grammar behind this catalogue profile, or
    /// `None` for a profile whose runtime core is not on the F5 tree.
    ///
    /// The nine word-form `expr` operators (`and`/`or`/`not`/`contains`/
    /// `starts_with`/`ends_with`/`equals`/`matches_glob`/`matches_regex`)
    /// are an **`f5-tcl` trunk fact**, measured byte-identical in tmsh and
    /// iApp contexts too, not iRules-only
    /// (`docs/design/bigip-irule-parser-measurements.md` §4a) — so every
    /// F5Tcl-cored catalogue profile answers with the family's own
    /// [`ExprGrammar`](crate::model::expr_grammar::ExprGrammar) here and
    /// consumers read the word-operator surface off that table instead of
    /// duplicating rows (ledger C12/B6). The old-catalogue `f5-tmsh` /
    /// `f5-iapps` grammar rows themselves are deliberately retained
    /// (P1-G): only the expr word-operator acceptance follows the family
    /// fact.
    ///
    /// `f5-bigip` is excluded by design: it is a config-schema identity
    /// with no Tcl runtime or expr grammar of its own (its embedded iRules
    /// route through `f5-irules`).
    #[must_use]
    pub fn f5_core_expr_grammar(&self) -> Option<&'static crate::model::ExprGrammar> {
        use crate::model::family::Release;
        match self.vendor_surface {
            // The iRules offshoot overrides no expr axis — it answers with
            // the trunk grammar along the fork edge (measurements §4a).
            Some(SpecProvider::Core(Family::F5Irules)) => {
                Some(crate::model::expr(Family::F5Irules, Release::F5_IRULES_TMM))
            }
            Some(SpecProvider::Package("tmsh" | "iapps")) => {
                Some(crate::model::expr(Family::F5Tcl, Release::F5_TCL_TMOS))
            }
            _ => None,
        }
    }

    /// The version-aware *compile-time fold* projection — deliberately
    /// exact: `Some` only for the plain versioned-Tcl
    /// profiles, `None` for every vendor dialect (including iRules, whose
    /// [`Self::runtime_base`] is a real `V8_4`) so versioned const-folds
    /// keep returning the dialect-invariant subset there until the
    /// optimiser/SCCP output is verified against real 8.4/8.5/8.6
    /// interpreters. The modelled runtime is `runtime_base`; this accessor
    /// is the *fold* policy.
    #[must_use]
    pub fn const_fold_version(&self) -> Option<TclVersion> {
        TclVersion::from_profile(self)
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
    /// shipped version, `Keyed` → the D5 oldest-supported default), for
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

/// Canonical dialect profile names, in sorted order.
///
/// Kept pre-sorted so [`available_dialects`] returns them in sorted
/// order. This
/// is the single source of truth for the explorer's dialect dropdown and
/// the CLI's `--dialect` choices. Every name here resolves to its own
/// [`DialectProfile::find`] entry (`f5-tmsh` / `f5-bigip` are first-class
/// profiles, D8; `tk` is a library pin, not a profile — §7.2).
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
    "microchip-libero-eda-tcl",
    "spectcl",
    "sslictcl",
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

#[cfg(test)]
mod tests {
    use super::DialectProfile;
    use super::KNOWN_DIALECTS;
    use crate::grammar::{BracedVarStyle, EscapeSyntax, ExprCommentStyle, NumberSyntax};
    use crate::library::{LibraryVersion, LibraryVersionOverrides, VersionKey};
    use crate::model::{Family, SpecProvider, SpecSurface, SurfaceQuery, surface_admits};
    use crate::version::{TclVersion, Ternary};

    #[test]
    fn library_pins_follow_the_dialect_shape() {
        // §7 "Libraries" column invariants, derived not enumerated:
        for p in all_with_fallback() {
            // Every F5 profile keys its own surface on the BIG-IP release.
            if p.name.starts_with("f5-") {
                assert!(
                    p.libraries.iter().any(|pin| pin.ambient
                        && pin.version == LibraryVersion::Keyed(VersionKey::BigipVersion)),
                    "{}: F5 surfaces are keyed on BigipVersion",
                    p.name
                );
            }
            // Every EDA shell keys sdc + upf + its tool; all ambient.
            if p.name.ends_with("-eda-tcl") {
                assert!(
                    p.libraries.iter().any(|pin| pin.package == "sdc"
                        && pin.version == LibraryVersion::Keyed(VersionKey::SdcVersion)),
                    "{}: sdc keyed on SdcVersion",
                    p.name
                );
                assert!(
                    p.libraries.iter().any(|pin| pin.package == "upf"
                        && pin.version == LibraryVersion::Keyed(VersionKey::UpfVersion)),
                    "{}: upf keyed on UpfVersion",
                    p.name
                );
                assert!(
                    p.libraries
                        .iter()
                        .any(|pin| pin.version == LibraryVersion::Keyed(VersionKey::ToolVersion)),
                    "{}: tool keyed on ToolVersion",
                    p.name
                );
                assert!(p.libraries.iter().all(|pin| pin.ambient), "{}", p.name);
            }
            // Plain versioned Tcl hosts Tk (tracking base) + Itcl (pinned),
            // neither ambient — `package require` still applies.
            if p.name.starts_with("tcl") && p.name != "tcl" {
                let tk = p.library_pin("Tk").expect("plain Tcl hosts Tk");
                assert_eq!(tk.version, LibraryVersion::TracksBase, "{}", p.name);
                assert!(!tk.ambient, "{}: Tk still needs its require", p.name);
                assert!(
                    p.library_pin("Itcl").is_some_and(
                        |i| !i.ambient && matches!(i.version, LibraryVersion::Pinned(_))
                    ),
                    "{}: Itcl is a pinned hosted library",
                    p.name
                );
            }
            // Closed vendor worlds never host Tk (§2.2).
            if p.vendor_surface.is_some() {
                assert!(
                    p.library_pin("Tk").is_none(),
                    "{}: vendor shells never host Tk",
                    p.name
                );
            }
            // The permissive fallback pins nothing.
            if p.is_fallback() {
                assert!(p.libraries.is_empty());
            }
        }
    }

    #[test]
    fn library_floor_resolution_covers_all_pin_kinds() {
        let none = LibraryVersionOverrides::default();

        // TracksBase → the runtime base as a package version.
        let tcl86 = DialectProfile::find("tcl8.6").expect("catalogue profile");
        assert_eq!(tcl86.library_floor("Tk", &none), Some("8.6"));
        let tcl90 = DialectProfile::find("tcl9.0").expect("catalogue profile");
        assert_eq!(tcl90.library_floor("Tk", &none), Some("9.0"));

        // Pinned → the pinned string.
        assert_eq!(tcl86.library_floor("Itcl", &none), Some("4.2"));
        assert_eq!(
            DialectProfile::find("tcl8.4")
                .expect("catalogue profile")
                .library_floor("Itcl", &none),
            Some("3.4")
        );
        assert_eq!(
            DialectProfile::find("expect")
                .expect("catalogue profile")
                .library_floor("Expect", &none),
            Some("5.45.4")
        );

        // Keyed → override, else the D5 oldest-supported default.
        let irules = DialectProfile::irules();
        assert_eq!(
            irules.library_floor("f5-irules-cmds", &none),
            Some("16.1.0"),
            "BigipVersion defaults to the oldest supported TMOS"
        );
        let pinned = LibraryVersionOverrides {
            bigip_version: Some("17.1.0".to_owned()),
            ..LibraryVersionOverrides::default()
        };
        assert_eq!(
            irules.library_floor("f5-irules-cmds", &pinned),
            Some("17.1.0"),
            "an explicit pin overrides the default"
        );
        // Keyed with no default and no override → permissive.
        let synopsys = DialectProfile::find("synopsys-eda-tcl").expect("catalogue profile");
        assert_eq!(synopsys.library_floor("synopsys-dc", &none), None);

        // Unpinned package → no profile floor.
        assert_eq!(tcl86.library_floor("no-such-lib", &none), None);
        // Ambience predicate: F5 surface ambient, Tk hosted.
        assert!(irules.is_ambient_package("f5-irules-cmds"));
        assert!(!tcl86.is_ambient_package("Tk"));
        assert!(!tcl86.is_ambient_package("no-such-lib"));
    }

    #[test]
    fn help_terms_cover_every_real_dialect() {
        // §5.4: every catalog profile carries help-filter terms; only the
        // permissive fallback filters nothing. The versioned-Tcl profiles
        // (tcl9.1 included — the old string table missed it) share the
        // tcl/tk terms; every vendor profile's terms include a
        // vendor-identifying string.
        for p in DialectProfile::all() {
            assert!(
                !p.help_terms.is_empty(),
                "{}: catalog profiles carry help terms",
                p.name
            );
            if p.name.starts_with("tcl") {
                assert_eq!(p.help_terms, &["tcl", "tk"], "{}", p.name);
            }
        }
        assert!(
            DialectProfile::plain_tcl().help_terms.is_empty(),
            "the fallback applies no filter"
        );
        // Alias canonicalisation (§2.4): the legacy `irules` spelling
        // resolves to the same terms as the canonical profile — the old
        // string-keyed table silently applied no filter to it.
        assert_eq!(
            DialectProfile::find("irules")
                .expect("catalogue profile")
                .help_terms,
            DialectProfile::find("f5-irules")
                .expect("catalogue profile")
                .help_terms
        );
        assert!(
            DialectProfile::find("f5-tmsh")
                .expect("catalogue profile")
                .help_terms
                .contains(&"tmsh")
        );
        assert!(
            DialectProfile::find("bpf")
                .expect("catalogue profile")
                .help_terms
                .contains(&"bpf")
        );
    }

    #[test]
    fn catalog_matches_known_dialects_exactly() {
        // One profile per canonical name, in the same (sorted) order —
        // KNOWN_DIALECTS and the catalog can never drift apart.
        let names: Vec<&str> = DialectProfile::all().iter().map(|p| p.name).collect();
        assert_eq!(names.as_slice(), KNOWN_DIALECTS);
    }

    #[test]
    fn find_resolves_canonical_names_to_themselves() {
        for &name in KNOWN_DIALECTS {
            assert_eq!(
                DialectProfile::find(name).expect("catalogue profile").name,
                name
            );
        }
    }

    #[test]
    fn unknown_and_ingress_only_names_are_not_catalogue_entries() {
        // The catalogue lookup answers `None` for anything that is not a
        // canonical name or alias — the lenient-sink behaviour every
        // user-written string used to get from `by_name` now lives in the
        // environment seam (`tcl_registry::model::ingress`), where its
        // tests pin it.
        for unknown in ["", "nonsense", "tcl8.7", "TCL8.6", "tk", "tcl"] {
            assert!(DialectProfile::find(unknown).is_none(), "{unknown:?}");
        }
    }

    #[test]
    fn tk_ingress_profile_keeps_the_typed_library_bit() {
        // Tk deliberately remains a library pin rather than an
        // editor-visible catalog profile, but an explicit `tk` / wish
        // document must retain the typed command-surface fact — carried by
        // the dedicated ingress profile the environment seam promotes.
        assert_eq!(DialectProfile::tk().name, "tk");
        assert_eq!(
            DialectProfile::tk().surface_query(),
            SurfaceQuery {
                core: DialectProfile::plain_tcl().surface_query().core,
                packages: &["Tk"],
            }
        );
        // A canonical profile and a legacy alias keep their profile-owned
        // security scope: the iRules sandbox point is the same under either
        // spelling.
        assert_eq!(
            DialectProfile::find("f5-irules")
                .expect("catalogue profile")
                .surface_query(),
            SurfaceQuery::any_release(Family::F5Irules)
        );
        assert_eq!(
            DialectProfile::find("irules")
                .expect("registered alias")
                .surface_query(),
            SurfaceQuery::any_release(Family::F5Irules)
        );
    }

    #[test]
    fn irules_handle_is_the_catalog_entry() {
        let via_handle = DialectProfile::irules();
        let via_name = DialectProfile::find("f5-irules").expect("catalogue profile");
        assert!(std::ptr::eq(via_handle, via_name));
        assert_eq!(via_handle.name, "f5-irules");
        assert!(via_handle.is_irules());
        assert!(
            !DialectProfile::find("tcl8.4")
                .expect("catalogue profile")
                .is_irules()
        );
    }

    #[test]
    fn profiles_are_interned_pointer_identities() {
        assert!(std::ptr::eq(
            DialectProfile::find("tcl8.6").expect("catalogue profile"),
            DialectProfile::find("tcl8.6").expect("catalogue profile")
        ));
        assert!(!std::ptr::eq(
            DialectProfile::find("tcl8.6").expect("catalogue profile"),
            DialectProfile::find("tcl9.0").expect("catalogue profile")
        ));
    }

    #[test]
    fn irules_aliases_canonicalise_to_the_same_profile() {
        // §2.4: `irules` / `tcl-irule` must resolve to the f5-irules
        // profile so profile predicates can never disagree with the
        // canonical spelling.
        for alias in ["irules", "tcl-irule"] {
            let p = DialectProfile::find(alias).expect("catalogue profile");
            assert!(p.is_irules(), "{alias:?} must canonicalise to f5-irules");
        }
    }

    #[test]
    fn the_irules_point_names_the_family_and_no_release() {
        // A command is available under iRules iff its own surface names the
        // family. Every core command states a surface, so there is no
        // catch-all and no subtractive ban list — a sandbox-banned command
        // such as `exec` simply does not name iRules and falls out. The
        // command-level checks (`exec`/`file`/`socket` unavailable,
        // `pool`/`set` available) live in tcl-registry's
        // `dialect_profile.rs` suite, which has the registry; here we pin
        // only the point shape the scheme rests on.
        let p = DialectProfile::irules();
        assert_eq!(
            p.surface_query(),
            SurfaceQuery::any_release(Family::F5Irules)
        );
        assert_eq!(p.vendor_surface, Some(SpecProvider::Core(Family::F5Irules)));
    }

    #[test]
    fn an_additive_vendor_point_carries_its_base_release_and_its_package() {
        // §7: an *additive* vendor profile is a base release plus its own
        // package. The EDA shells moved to the packaged model (a plain base
        // release plus a `required_package` gate — see
        // eda-library-packages.md), so only the F5 iApps host shell and
        // Expect remain additive here.
        // f5-iapps composes the **8.4** line: it rides the `f5-tcl` trunk
        // (fork of Tcl at 8.4.6) — measured, bigip-irule-parser-measurements.md
        // §4a; the 8.5 hypothesis is falsified.
        let cases: &[(&str, &str, &[&str])] = &[
            ("f5-iapps", "8.4", &["iapps"]),
            ("expect", "8.6", &["expect"]),
        ];
        for &(name, base, packages) in cases {
            let p = DialectProfile::find(name).expect("catalogue profile");
            assert_eq!(
                p.surface_query(),
                SurfaceQuery::core(Family::Tcl, base).with_packages(packages),
                "{name}"
            );
        }
    }

    #[test]
    fn eda_shells_are_packaged_vendors_with_a_plain_release_point() {
        // The EDA-as-packages migration: each EDA shell's point is its plain
        // base Tcl release, with no vendor surface — the vendor command
        // surface is gated by `required_package` (ambient in the profile).
        // Base versions follow the tools' embedded cores (owner decisions):
        // Cadence 8.4-safe, Xilinx/Quartus 8.5, Synopsys + modern Questa 8.6.
        for (name, base) in [
            ("xilinx-eda-tcl", "8.5"),
            ("synopsys-eda-tcl", "8.6"),
            ("cadence-eda-tcl", "8.4"),
            ("intel-quartus-eda-tcl", "8.5"),
            ("mentor-eda-tcl", "8.6"),
        ] {
            let p = DialectProfile::find(name).expect("catalogue profile");
            assert_eq!(
                p.surface_query(),
                SurfaceQuery::core(Family::Tcl, base),
                "{name}: a pure base-release point with no vendor package"
            );
        }
    }

    #[test]
    fn plain_tcl_versions_keep_their_exact_release() {
        for (name, release) in [
            ("tcl8.4", "8.4"),
            ("tcl8.5", "8.5"),
            ("tcl8.6", "8.6"),
            ("tcl9.0", "9.0"),
            ("tcl9.1", "9.1"),
        ] {
            assert_eq!(
                DialectProfile::find(name)
                    .expect("catalogue profile")
                    .surface_query(),
                SurfaceQuery::core(Family::Tcl, release)
            );
        }
    }

    #[test]
    fn first_class_vendor_masks_are_precise() {
        // The first-class vendor masks are precise (D7/D8). f5-tmsh: the
        // tmsh shell hosts the `f5-tcl` trunk (fork of Tcl at 8.4.6 —
        // measured, bigip-irule-parser-measurements.md §4a) plus its own
        // surface.
        assert_eq!(
            DialectProfile::find("f5-tmsh")
                .expect("catalogue profile")
                .surface_query(),
            SurfaceQuery::core(Family::Tcl, "8.4").with_packages(&["tmsh"])
        );
        // f5-bigip: identity only — a config parser with no Tcl surface;
        // BIG-IP documents route to the tcl-bigip validator, never the Tcl
        // analyser.
        assert_eq!(
            DialectProfile::find("f5-bigip")
                .expect("catalogue profile")
                .surface_query(),
            SurfaceQuery {
                core: None,
                packages: &["bigip"],
            }
        );
        // bpf embeds a genuine Tcl 9.0 (D7).
        assert_eq!(
            DialectProfile::find("bpf")
                .expect("catalogue profile")
                .surface_query(),
            SurfaceQuery::core(Family::Tcl, "9.0").with_packages(&["bpf"])
        );
    }

    #[test]
    fn grammar_union_covers_every_provider_the_point_names() {
        // §10: the static-grammar union over-approximates (or equals, for
        // the iRules profile) the precise point — never under-approximates.
        for p in DialectProfile::all() {
            let query = p.surface_query();
            if let Some((family, _)) = query.core {
                assert!(
                    p.grammar_union.contains(&SpecProvider::Core(family)),
                    "{}: grammar_union must cover the point's core family",
                    p.name
                );
            }
            for package in query.packages {
                assert!(
                    p.grammar_union.contains(&SpecProvider::Package(package)),
                    "{}: grammar_union must cover the point's `{package}` package",
                    p.name
                );
            }
        }
    }

    #[test]
    fn irules_grammar_union_is_the_shipped_bare_bit_fix() {
        // The shipped iRules highlight fix is literally "the grammar union
        // is the iRules surface and nothing else" (§9.1).
        let p = DialectProfile::irules();
        assert_eq!(p.grammar_union, &[SpecProvider::Core(Family::F5Irules)]);
    }

    // Behaviour axis — the §7.1 derivation rules as
    // invariants, so the hand-laid table can never drift from the model.

    fn all_with_fallback() -> impl Iterator<Item = &'static DialectProfile> {
        DialectProfile::all()
            .iter()
            .chain(std::iter::once(DialectProfile::plain_tcl()))
    }

    #[test]
    fn surface_packages_carry_the_vendor_surface() {
        // The point's package half is the vendor package wherever there is
        // one; `tk` is the single documented addition (a library, not a
        // closed-world vendor surface).
        for p in all_with_fallback() {
            match p.vendor_surface {
                Some(SpecProvider::Package(package)) => assert_eq!(
                    p.surface_packages,
                    [package],
                    "{}: the point carries exactly its vendor package",
                    p.name
                ),
                _ => assert!(
                    p.surface_packages.is_empty() || p.name == "tk",
                    "{}: only `tk` adds a package without a vendor surface",
                    p.name
                ),
            }
        }
    }

    #[test]
    fn the_vendor_surface_composes_the_point() {
        // §2.2 + the EDA-as-packages migration: a profile's point is one of
        // - a bare core family (iRules, §9);
        // - a Tcl release plus the vendor package (iApps, Expect, tmsh,
        //   bpf) or the bigip identity; or
        // - a bare Tcl release for a *packaged* vendor (the EDA shells),
        //   whose `vendor_surface` is a loading marker only — the vendor
        //   command surface is gated by `required_package`, not the point.
        for p in all_with_fallback() {
            let query = p.surface_query();
            match p.vendor_surface {
                Some(SpecProvider::Core(family)) => {
                    assert_eq!(
                        query,
                        SurfaceQuery::any_release(family),
                        "{}: a core vendor surface asks as that family alone (§9)",
                        p.name
                    );
                }
                Some(SpecProvider::Package(package)) => {
                    assert!(
                        query.packages.contains(&package),
                        "{}: the point must carry the vendor package",
                        p.name
                    );
                }
                None => {
                    assert!(
                        query.packages.is_empty(),
                        "{}: no vendor surface — the point carries no package",
                        p.name
                    );
                }
            }
            assert!(
                query
                    .core
                    .is_none_or(|(family, _)| family == Family::Tcl || p.is_irules()),
                "{}: a non-iRules point asks on the Tcl ladder",
                p.name
            );
        }
    }

    #[test]
    fn version_ceiling_tracks_the_signature_base() {
        // §7: the option-gating ceiling is the signature base everywhere —
        // including the first-class vendor profiles (f5-tmsh V8_5;
        // f5-bigip has neither, being a non-Tcl surface). Only the
        // permissive fallback stays unceilinged by design.
        for p in all_with_fallback() {
            assert_eq!(
                p.version_ceiling, p.signature_base,
                "{}: ceiling == signature base",
                p.name
            );
        }
    }

    #[test]
    fn octal_policy_is_derived_from_the_runtime_base() {
        // §7.1: octal = runtime_base < V9_0; Inert exactly when there is
        // no Tcl runtime (never a silent default — §11.1).
        for p in all_with_fallback() {
            let expected = match p.runtime_base {
                None => Ternary::Inert,
                Some(v) if v < TclVersion::V9_0 => Ternary::Yes,
                Some(_) => Ternary::No,
            };
            assert_eq!(p.leading_zero_is_octal, expected, "{}", p.name);
        }
    }

    #[test]
    fn expr_grammar_base_equals_runtime_base() {
        // §7.1: the expr grammar (TIP 201/461, mathfunc tiers) follows the
        // runtime, not the signature surface.
        for p in all_with_fallback() {
            assert_eq!(p.expr_grammar_base, p.runtime_base, "{}", p.name);
        }
    }

    #[test]
    fn vm_runtime_version_tracks_the_profile_runtime_base() {
        // The VM must not silently execute a vendor or versioned profile with
        // Tcl 9 semantics.  A non-Tcl / permissive profile has no runtime base,
        // so its deliberately documented fallback stays the C Tcl 9.0 oracle.
        for p in all_with_fallback() {
            assert_eq!(
                p.vm_runtime_version,
                p.runtime_base.unwrap_or(TclVersion::V9_0),
                "{}",
                p.name
            );
        }
    }

    #[test]
    fn signature_and_runtime_base_agree_everywhere_today() {
        // Every profile in the current catalog has signature == runtime
        // (iRules has both at V8_4 — D3). The fields stay structurally
        // separate because future dialects may split them; this test
        // documents that no current entry does.
        for p in all_with_fallback() {
            assert_eq!(p.signature_base, p.runtime_base, "{}", p.name);
        }
    }

    #[test]
    fn tcloo_agrees_with_what_the_point_resolves() {
        // §11.2: the hand-filled tcloo bool must agree with what the point
        // resolves for `oo::*` (8.6 and later) — otherwise hover and the oo
        // handler would contradict each other. Documented exception:
        // f5-bigip has NO Tcl surface at all (tcloo false is the model
        // truth, §7), so it is asserted directly.
        for p in all_with_fallback() {
            if p.name == "f5-bigip" {
                assert!(!p.tcloo, "f5-bigip has no Tcl surface at all");
                continue;
            }
            assert_eq!(
                p.tcloo,
                surface_admits(SpecSurface::TCL86_PLUS, Some(&p.surface_query())),
                "{}: tcloo must match what the point resolves for oo::*",
                p.name
            );
        }
    }

    #[test]
    fn fixed_ensembles_cover_exactly_the_f5_family() {
        // §7.1: {f5-irules, f5-iapps, f5-bigip} — NOT f5-tmsh (a wrong
        // `true` mis-minifies tmsh scripts).
        for p in all_with_fallback() {
            let expected = matches!(p.name, "f5-irules" | "f5-iapps" | "f5-bigip");
            assert_eq!(p.has_fixed_ensembles, expected, "{}", p.name);
        }
    }

    #[test]
    fn operators_are_commands_everywhere_but_irules_bigip_tcl84_and_cadence() {
        // §9: the math-operator heads (`::tcl::mathop`, TIP 174) exist in every
        // command dialect on a Tcl 8.5+ core. The pre-8.5 cores have none — the
        // `::tcl::` namespace itself is 8.5+: the F5 trunk profiles (all three
        // ride the fork of Tcl at 8.4.6, and `::tcl::mathop` is measured
        // absent in every BIG-IP execution context —
        // bigip-irule-parser-measurements.md §4a), plain tcl8.4, and Cadence
        // Innovus/Genus (8.4-safe, owner decision). f5-bigip has no command
        // surface at all; `tk` is a library pin, not a profile.
        for p in all_with_fallback() {
            let expected = !(p.is_irules()
                || p.name == "f5-bigip"
                || p.name == "f5-iapps"
                || p.name == "f5-tmsh"
                || p.name == "tcl8.4"
                || p.name == "cadence-eda-tcl");
            assert_eq!(p.operators_as_commands, expected, "{}", p.name);
        }
    }

    #[test]
    fn lexer_grammar_follows_the_runtime_base() {
        // 8.x runtimes lex `${…}` to the FIRST close brace; 9.x (and the
        // no-runtime profiles) use the modern nesting rule. `{*}` expansion
        // is 8.5+; the `}{` ghost separator is iRules-only.
        for p in all_with_fallback() {
            let expected_braced = match p.runtime_base {
                Some(v) if v < TclVersion::V9_0 => BracedVarStyle::FirstClose,
                _ => BracedVarStyle::Tcl9Nesting,
            };
            assert_eq!(p.grammar.braced_var, expected_braced, "{}", p.name);
            let expected_expand = p.runtime_base.is_none_or(|v| v >= TclVersion::V8_5);
            assert_eq!(p.grammar.expand_syntax, expected_expand, "{}", p.name);
            // The implicit word break (R-rules) is an `f5-tcl` **trunk**
            // fact, measured byte-identical in all three BIG-IP execution
            // contexts (bigip-irule-parser-measurements.md §1, §4a) — so
            // every trunk-riding profile carries it, not just iRules.
            let expected_separator = matches!(p.name, "f5-irules" | "f5-iapps" | "f5-tmsh");
            assert_eq!(
                p.grammar.irules_brace_separator, expected_separator,
                "{}",
                p.name
            );
            // TIP 582 `expr` comments: the `COMMENT` lexeme and
            // `ParseLexeme`'s `case '#':` appear in `tclCompExpr.c` from the
            // 8.7/9.0 cycle and are absent at core-8-4-20 / core-8-5-19 /
            // core-8-6-16, so `>= V9_0` is the exact gate for the versions
            // modelled here.
            let expected_comments = if p.runtime_base.is_none_or(|v| v >= TclVersion::V9_0) {
                ExprCommentStyle::Hash
            } else {
                ExprCommentStyle::None
            };
            assert_eq!(p.grammar.expr_comments, expected_comments, "{}", p.name);
            // Numeric-literal grammar: `tclStrToD.c` does not exist at
            // core-8-4-20 (8.4 scans integers strtoul-style, knowing only `0x`
            // and leading-zero octal); core-8-5-19 / core-8-6-16 add the
            // `ZERO_B`/`ZERO_O` states but no `ZERO_D` and keep
            // `#undef KILL_OCTAL`; 9.0.4 adds `ZERO_D` plus `_` separators and
            // drops leading-zero octal (`changes.md`). So the gates are exactly
            // `>= V8_5` for `0b`/`0o` and `>= V9_0` for `0d` / `_` / decimal
            // leading zero.
            let expected_numbers = match p.runtime_base {
                Some(TclVersion::V8_4) => NumberSyntax::Tcl84,
                Some(TclVersion::V8_5 | TclVersion::V8_6) => NumberSyntax::Tcl85,
                _ => NumberSyntax::Tcl90,
            };
            assert_eq!(p.grammar.numbers, expected_numbers, "{}", p.name);
            // Backslash-escape grammar: TIP 388 (8.6) capped `\x` at two hex
            // digits, added `\U`, and guarded the octal third digit, so 8.4 and
            // 8.5 share one rule and 8.6 starts another. 9.0 keeps 8.6's
            // widths and raises `TCL_UTF_MAX` to 4, so a decoded scalar past
            // U+FFFF stops degrading to U+FFFD.
            let expected_escapes = match p.runtime_base {
                Some(TclVersion::V8_4 | TclVersion::V8_5) => EscapeSyntax::Tcl84,
                Some(TclVersion::V8_6) => EscapeSyntax::Tcl86,
                _ => EscapeSyntax::Tcl90,
            };
            assert_eq!(p.grammar.escapes, expected_escapes, "{}", p.name);
        }
    }

    #[test]
    fn escape_grammar_splits_85_from_86() {
        // The one axis on which 8.5 and 8.6 differ — they share a numeral
        // grammar, a `${…}` rule, and an `expr` grammar, so a single shared
        // 8.x `LexerGrammar` constant would silently give 8.5 TIP 388's rules.
        assert_eq!(
            DialectProfile::find("tcl8.5")
                .expect("catalogue profile")
                .grammar
                .escapes,
            EscapeSyntax::Tcl84
        );
        assert_eq!(
            DialectProfile::find("tcl8.6")
                .expect("catalogue profile")
                .grammar
                .escapes,
            EscapeSyntax::Tcl86
        );
        assert_eq!(
            DialectProfile::find("tcl8.5")
                .expect("catalogue profile")
                .grammar
                .numbers,
            DialectProfile::find("tcl8.6")
                .expect("catalogue profile")
                .grammar
                .numbers
        );
        // iRules is a genuine embedded 8.4.6, so it takes the 8.4 escapes.
        assert_eq!(
            DialectProfile::find("f5-irules")
                .expect("catalogue profile")
                .grammar
                .escapes,
            EscapeSyntax::Tcl84
        );
    }

    #[test]
    fn expect_and_tmsh_lex_braced_vars_with_the_8x_rule() {
        // The 8.x rule the old string-keyed lexer table missed:
        // expect (8.6) and f5-tmsh (8.5) are 8.x runtimes, so `${a{b}c}`
        // names `a{b` — not the Tcl 9 nesting read.
        for name in ["expect", "f5-tmsh"] {
            assert_eq!(
                DialectProfile::find(name)
                    .expect("catalogue profile")
                    .grammar
                    .braced_var,
                BracedVarStyle::FirstClose,
                "{name}"
            );
        }
        // bpf embeds Tcl 9.0: nesting, unchanged.
        assert_eq!(
            DialectProfile::find("bpf")
                .expect("catalogue profile")
                .grammar
                .braced_var,
            BracedVarStyle::Tcl9Nesting
        );
    }

    #[test]
    fn const_fold_version_stays_bit_identical_to_from_dialect() {
        // The const-fold guardrail: versioned const-folds keep the
        // exact `TclVersion::from_dialect` behaviour — plain versioned Tcl
        // resolves, every vendor dialect (iRules included, despite its
        // modelled V8_4 runtime) stays None until tclsh-verified.
        for p in all_with_fallback() {
            assert_eq!(
                p.const_fold_version(),
                TclVersion::from_dialect(Some(p.name)),
                "{}",
                p.name
            );
        }
        assert_eq!(
            DialectProfile::irules().const_fold_version(),
            None,
            "iRules const-folds stay dialect-invariant this milestone"
        );
        assert_eq!(
            DialectProfile::find("tcl8.4")
                .expect("catalogue profile")
                .const_fold_version(),
            Some(TclVersion::V8_4)
        );
    }

    #[test]
    fn presentation_fields_follow_the_catalog_shape() {
        let mut seen_ext: Vec<&str> = Vec::new();
        let mut seen_lang: Vec<&str> = Vec::new();
        for p in all_with_fallback() {
            assert!(!p.display_name.is_empty(), "{}: display_name", p.name);
            assert!(!p.short_name.is_empty(), "{}: short_name", p.name);
            if let Some(lang) = p.editor_language_id {
                // Undotted by contract (issue #1122), and unique: two
                // dialects can't claim the same editor language.
                assert!(
                    !lang.contains('.'),
                    "{}: language id {lang:?} has a dot",
                    p.name
                );
                assert!(
                    !seen_lang.contains(&lang),
                    "{}: language id {lang:?} reused",
                    p.name
                );
                seen_lang.push(lang);
            }
            for row in p.file_extensions {
                assert!(
                    !row.extension.is_empty()
                        && !row.extension.starts_with('.')
                        && row.extension.chars().all(|c| c.is_ascii_lowercase()),
                    "{}: extension {:?} must be lower-case with no dot",
                    p.name,
                    row.extension
                );
                assert!(
                    !row.display_name.is_empty(),
                    "{}: {} name",
                    p.name,
                    row.extension
                );
                // Extension routing is a function: one owner per extension
                // across the whole catalog.
                assert!(
                    !seen_ext.contains(&row.extension),
                    "{}: extension {:?} owned twice",
                    p.name,
                    row.extension
                );
                seen_ext.push(row.extension);
                // A dialect that owns file extensions must give the editors
                // somewhere to register them.
                assert!(
                    p.editor_language_id.is_some() || p.name == "tcl",
                    "{}: owns extensions but has no editor language",
                    p.name
                );
            }
        }
    }

    /// The `filenames` axis obeys the same shape rules as `file_extensions`:
    /// lower-case whole basenames, one owner apiece, and an editor language
    /// to register them under (issue #1625).
    #[test]
    fn owned_filenames_follow_the_catalog_shape() {
        let mut seen: Vec<&str> = Vec::new();
        for p in all_with_fallback() {
            for name in p.filenames {
                assert!(
                    !name.is_empty() && *name == name.to_ascii_lowercase(),
                    "{}: filename {name:?} must be lower-case and non-empty",
                    p.name
                );
                assert!(
                    !name.contains('/') && !name.contains('\\'),
                    "{}: filename {name:?} must be a bare basename",
                    p.name
                );
                assert!(
                    !seen.contains(name),
                    "{}: filename {name:?} owned twice",
                    p.name
                );
                seen.push(name);
                assert!(
                    p.editor_language_id.is_some(),
                    "{}: owns filenames but has no editor language",
                    p.name
                );
            }
        }
        // The axis is not vacuous — BIG-IP's config basenames are its whole
        // reason to exist, and an empty catalog would pass every rule above.
        assert!(
            seen.contains(&"bigip.conf"),
            "the BIG-IP config basenames must be catalogued"
        );
    }
}
