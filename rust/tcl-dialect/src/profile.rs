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
//! This is the compositional model of `docs/design/dialect-profile-model.md`
//! landing milestone by milestone: identity (Milestone 1), the availability
//! axis — masks, disable list, load layers, grammar unions (Milestone 2) —
//! and the behaviour/runtime axis — base versions, octal policy, expr
//! grammar, lexer grammar, the per-dialect predicates (Milestone 3). The
//! versioned-library axis follows.

use crate::dialect_set::DialectSet;
use crate::grammar::{BracedVarStyle, LexerGrammar};
use crate::version::{TclVersion, Ternary};

/// The commands F5's TMM interpreter removes from iRules: the K36322151
/// bans (file system, process, channel-config, event-loop, and
/// introspection commands the data-plane sandbox strips) plus the
/// project-modelled iRules-excluded internals (`tcl::build-info`,
/// `tcl_findLibrary`, the regex-quote helpers).
///
/// This is the **subtractive** half of the iRules profile: the
/// [`DialectProfile::availability_mask`] is the *bare* `IRULES` bit and every
/// availability consumer applies this disable list **after** the mask query
/// (`is_available` = mask ∧ ¬disabled). Never fold these into the mask as a
/// `TCL84|IRULES` union — the union re-admits exactly these commands (see
/// §9 of the design doc, the subtractive-iRules trap).
///
/// Membership is data-bound, not hand-curated: until the Milestone 5 data
/// retag the per-spec `NON_IRULES_OPERATORS` tags encode the same exclusion,
/// and `tcl-registry`'s `irules_disabled_commands_match_the_spec_data`
/// contract test keeps this list and the spec data in lock-step from both
/// sides. (8.5+/8.6 core such as `dict`/`lmap` is deliberately NOT here —
/// that is version-gating on the pinned 8.4 base, a different axis.)
const IRULES_DISABLED_COMMANDS: &[&str] = &[
    "::tcl::build-info",
    "::tcl::unsupported::corotype",
    "auto_execok",
    "auto_import",
    "auto_load",
    "auto_mkindex",
    "auto_mkindex_old",
    "auto_qualify",
    "auto_reset",
    "bgerror",
    "cd",
    "eof",
    "exec",
    "exit",
    "fblocked",
    "fconfigure",
    "fcopy",
    "file",
    "fileevent",
    "filename",
    "flush",
    "gets",
    "glob",
    "http",
    "interp",
    "load",
    "memory",
    "namespace",
    "open",
    "package",
    "pid",
    "pkg_mkindex",
    "pwd",
    "re_quote",
    "regex::quote",
    "regex_quote",
    "regexp::quote",
    "rename",
    "seek",
    "socket",
    "source",
    "tcl::build-info",
    "tcl_findLibrary",
    "tell",
    "time",
    "timerate",
    "unknown",
    "unload",
    "update",
    "vwait",
];

/// The Tcl 8.4 lexing grammar: no `{*}` expansion (TIP 157 is 8.5), the
/// 8.x first-close `${…}` rule.
const GRAMMAR_TCL84: LexerGrammar = LexerGrammar {
    expand_syntax: false,
    irules_brace_separator: false,
    braced_var: BracedVarStyle::FirstClose,
};

/// The 8.5/8.6-family lexing grammar (plain 8.5/8.6, iApps, tmsh, Expect,
/// the EDA shells): `{*}` expansion, the 8.x first-close `${…}` rule.
const GRAMMAR_TCL8X: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    irules_brace_separator: false,
    braced_var: BracedVarStyle::FirstClose,
};

/// The modern 9.x grammar (also the permissive default): `{*}` expansion,
/// Tcl 9's nesting `${…}` rule.
const GRAMMAR_TCL9X: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    irules_brace_separator: false,
    braced_var: BracedVarStyle::Tcl9Nesting,
};

/// The iRules lexing grammar: a Tcl 8.4 base (no `{*}`) plus the
/// iRules-only `}{` ghost word separator.
const GRAMMAR_IRULES: LexerGrammar = LexerGrammar {
    expand_syntax: false,
    irules_brace_separator: true,
    braced_var: BracedVarStyle::FirstClose,
};

/// One resolved dialect. `'static`, interned in [`DialectProfile::all`],
/// keyed by canonical name.
///
/// Equality of profiles is pointer identity — there is exactly one profile
/// per canonical dialect, plus the [`DialectProfile::plain_tcl`] fallback
/// every unknown name resolves to.
///
/// The behaviour fields are **derived data fixed at catalog-construction
/// time** from the §7 table of the design doc (`signature_base` /
/// `runtime_base` / the vendor bit drive the rest); the derivation rules
/// (§7.1) are enforced by this module's invariant tests so the hand-laid
/// values can never drift from the model.
#[derive(Debug)]
pub struct DialectProfile {
    /// The canonical dialect name (`"tcl8.6"`, `"f5-irules"`, …). Stable:
    /// this is the string that round-trips through configuration
    /// (`tclLsp.selectDialect`, `folderDialects`), the registry-dump JSON
    /// schema, and `DialectSet::canonical_name`.
    pub name: &'static str,
    /// Legacy / editor spellings that resolve to this profile
    /// (`"irules"` → `f5-irules`). Resolution through [`Self::by_name`]
    /// canonicalises them, so profile predicates can never disagree with
    /// the canonical spelling the way the string-keyed tables used to
    /// (design doc §2.4).
    pub aliases: &'static [&'static str],
    /// Native tag of this dialect's own command surface, if any (`IRULES`,
    /// `IAPPS`, `EXPECT`, an EDA vendor, `BPF`). `None` for the plain
    /// Tcl-version profiles, the config-only dialects, and the permissive
    /// fallback. A vendor shell is a *closed world*: desktop libraries
    /// (Tk) can never be `package require`d into it, which consumers gate
    /// on via this field until the versioned-library axis models library
    /// hosting per profile (§7.2).
    pub vendor_bit: Option<DialectSet>,

    // AXIS A: availability.
    /// The **precise** availability mask commands / subcommands / options /
    /// special variables are membership-tested against
    /// (`spec.supports_dialect(intersects)`). Composed as
    /// `(signature-base Tcl version bits) | (vendor bit)` for the additive
    /// vendor dialects (`TCL85|IAPPS`, `TCL86|EXPECT`, …).
    ///
    /// For **subtractive** profiles (iRules) this is the *bare* vendor bit
    /// and [`Self::disabled_commands`] carries the exclusions — never a
    /// version|vendor union (§9 of the design doc).
    pub availability_mask: DialectSet,
    /// Subtractive disable list, applied AFTER the mask query by every
    /// availability consumer. Non-empty only for `f5-irules` (the 42
    /// K36322151 commands plus modelled internals). Empty for additive
    /// dialects.
    pub disabled_commands: &'static [&'static str],
    /// The registry command packs `load_dialect` applies for this profile,
    /// in order. The plain Tcl versions carry their own version bit (a
    /// spec-less "pack" that records the version on the registry's
    /// `loaded_dialects`); empty only for the config-only dialects that
    /// have no pack yet (`f5-tmsh`, `f5-bigip` — first-class in
    /// Milestone 6).
    pub base_layers: &'static [DialectSet],
    /// Coarse over-approximating union for **static** grammars only
    /// (tree-sitter / tmLanguage first-paint highlighting). Deliberately
    /// wider than [`Self::availability_mask`] — precise per-version
    /// correctness is the LSP semantic-token layer's job (§10). iRules is
    /// the exception: its static grammar is scoped to the bare `IRULES`
    /// bit (the shipped highlight fix this model preserves).
    pub grammar_union: DialectSet,
    /// UPPER-BOUND version guard for option gating (design doc §5.2): the
    /// highest Tcl version whose options may appear under this profile. A
    /// version-gated option resolves only when its gate's
    /// [`DialectSet::min_version`] is at or below this ceiling, so a
    /// tcl9.0-only option can never leak into an 8.5-superset profile whose
    /// mask happens to intersect its gate. `None` = no ceiling (the
    /// permissive fallback and the interim config-only dialects).
    pub version_ceiling: Option<TclVersion>,

    // AXIS B: behaviour / runtime.
    /// The Tcl version whose command/subcommand/option *signatures* this
    /// dialect exposes. Feeds the availability mask's version half. `None`
    /// for a non-Tcl surface (`f5-bigip`) and the permissive fallback.
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
    /// `LexerConfig::for_dialect` reads (Milestone 3) — derived from
    /// `runtime_base` (8.x → first-close, 9.x → nesting) plus the
    /// per-dialect quirks.
    pub grammar: LexerGrammar,
    /// Whether the math-operator command heads (`+`, `eq`,
    /// `tcl::mathop::*`) exist as callable commands. False for iRules —
    /// operators there live only inside `expr` (§9's second conflated
    /// fact; `tk` is modelled as a library, not a profile).
    pub operators_as_commands: bool,
    /// Whether `TclOO` (`oo::*`) is part of this dialect's surface. Explicit
    /// per profile and invariant-tested against the availability mask
    /// (§11.2) so it can never drift from the mask-resolved `oo::*`
    /// availability.
    pub tcloo: bool,
    /// Whether the dialect's ensemble commands are *fixed* — a closed
    /// subcommand set with no user-extensible ensembles — so the minifier
    /// may shorten subcommands to unambiguous prefixes. Exactly the F5
    /// family `{f5-irules, f5-iapps, f5-bigip}` — NOT f5-tmsh (§7.1).
    pub has_fixed_ensembles: bool,
    /// The Tcl version whose number/expr grammar the bytecode VM emulates
    /// when executing this dialect. Defaults to `V9_0` everywhere until
    /// the VM-parity milestone threads it (M8 / VM parity M16).
    pub vm_runtime_version: TclVersion,
}

/// The catalog: one profile per canonical dialect, in
/// [`KNOWN_DIALECTS`](crate::KNOWN_DIALECTS) (sorted) order.
///
/// Mask and behaviour values follow the per-dialect table in
/// `docs/design/dialect-profile-model.md` §7. Interim values that the
/// milestone plan tightens later are commented at the entry.
static CATALOG: [DialectProfile; 16] = [
    // bpf embeds a genuine Tcl 9.0 (design doc D7): 9.0 runtime semantics —
    // decimal leading zeros, 9.0 expr grammar, the nesting `${…}` rule.
    // Until Milestone 6 gives it the precise `TCL90|BPF` availability mask
    // (with its reverse-regression golden budget), the mask over-
    // approximates with every Tcl version so no core command is falsely
    // unknown — strictly fewer false positives than the pre-profile
    // bare-`BPF` view, never more.
    DialectProfile {
        name: "bpf",
        aliases: &[],
        vendor_bit: Some(DialectSet::BPF),
        availability_mask: DialectSet::ALL_TCL.union(DialectSet::BPF),
        disabled_commands: &[],
        base_layers: &[DialectSet::BPF],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::BPF),
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
    },
    DialectProfile {
        name: "cadence-eda-tcl",
        aliases: &[],
        vendor_bit: Some(DialectSet::CADENCE),
        availability_mask: DialectSet::TCL86.union(DialectSet::CADENCE),
        disabled_commands: &[],
        base_layers: &[DialectSet::CADENCE],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::CADENCE),
        version_ceiling: Some(TclVersion::V8_6),
        signature_base: Some(TclVersion::V8_6),
        runtime_base: Some(TclVersion::V8_6),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_6),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    // Expect embeds Tcl 8.6 — including the 8.x first-close `${…}` rule,
    // which the old string-keyed lexer table missed (it fell through to
    // the modern-9.x default; Milestone 3 flips it).
    DialectProfile {
        name: "expect",
        aliases: &[],
        vendor_bit: Some(DialectSet::EXPECT),
        availability_mask: DialectSet::TCL86.union(DialectSet::EXPECT),
        disabled_commands: &[],
        base_layers: &[DialectSet::EXPECT],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::EXPECT),
        version_ceiling: Some(TclVersion::V8_6),
        signature_base: Some(TclVersion::V8_6),
        runtime_base: Some(TclVersion::V8_6),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_6),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    // f5-bigip is a config parser, not a Tcl surface; it has no command
    // pack, no Tcl runtime (behaviour axis inert — §11.1), and no expr
    // grammar. Until Milestone 6 models it first-class, availability stays
    // as permissive as the pre-profile fallback so nothing regresses.
    DialectProfile {
        name: "f5-bigip",
        aliases: &[],
        vendor_bit: None,
        availability_mask: DialectSet::ALL_TCL,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
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
    },
    // iApps run a real Tcl 8.5.13 *host* interpreter (not the TMM sandbox):
    // full 8.5 core (dict, lassign, apply) plus the iApp surface; nothing
    // disabled. `TCL85|IAPPS` is the W123/W002 fix Milestone 2 landed.
    DialectProfile {
        name: "f5-iapps",
        aliases: &[],
        vendor_bit: Some(DialectSet::IAPPS),
        availability_mask: DialectSet::TCL85.union(DialectSet::IAPPS),
        disabled_commands: &[],
        base_layers: &[DialectSet::IAPPS],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::IAPPS),
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: true,
        vm_runtime_version: TclVersion::V9_0,
    },
    // iRules is SUBTRACTIVE (§9): a genuine embedded Tcl 8.4.6 whose F5
    // command surface carries the IRULES tag, minus the K36322151
    // disables. The mask stays the BARE bit — `TCL84|IRULES` would
    // re-admit exactly the disabled commands through their (until
    // Milestone 5) `NON_IRULES_OPERATORS` tags. Signature and runtime
    // base are both 8.4 (D3) and math operators are not command heads.
    DialectProfile {
        name: "f5-irules",
        aliases: &["irules", "tcl-irule"],
        vendor_bit: Some(DialectSet::IRULES),
        availability_mask: DialectSet::IRULES,
        disabled_commands: IRULES_DISABLED_COMMANDS,
        base_layers: &[DialectSet::IRULES],
        grammar_union: DialectSet::IRULES,
        version_ceiling: Some(TclVersion::V8_4),
        signature_base: Some(TclVersion::V8_4),
        runtime_base: Some(TclVersion::V8_4),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_4),
        grammar: GRAMMAR_IRULES,
        operators_as_commands: false,
        tcloo: false,
        has_fixed_ensembles: true,
        vm_runtime_version: TclVersion::V9_0,
    },
    // f5-tmsh runs on a Tcl 8.5 base (behaviour axis: octal, 8.5 expr
    // grammar, the 8.x first-close `${…}` rule — a Milestone 3 flip). It
    // has no DialectSet bit or command pack yet (first-class availability
    // in Milestone 6, D8, with the reverse-regression budget); until then
    // it resolves as permissively as the pre-profile fallback did, and
    // `tcloo` tracks that interim mask (Milestone 6 flips it to the 8.5
    // truth alongside the mask).
    DialectProfile {
        name: "f5-tmsh",
        aliases: &[],
        vendor_bit: None,
        availability_mask: DialectSet::ALL_TCL,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
        version_ceiling: None,
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    DialectProfile {
        name: "intel-quartus-eda-tcl",
        aliases: &[],
        vendor_bit: Some(DialectSet::QUARTUS),
        availability_mask: DialectSet::TCL85.union(DialectSet::QUARTUS),
        disabled_commands: &[],
        base_layers: &[DialectSet::QUARTUS],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::QUARTUS),
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    DialectProfile {
        name: "mentor-eda-tcl",
        aliases: &[],
        vendor_bit: Some(DialectSet::MENTOR),
        availability_mask: DialectSet::TCL85.union(DialectSet::MENTOR),
        disabled_commands: &[],
        base_layers: &[DialectSet::MENTOR],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::MENTOR),
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    DialectProfile {
        name: "synopsys-eda-tcl",
        aliases: &[],
        vendor_bit: Some(DialectSet::SYNOPSYS),
        availability_mask: DialectSet::TCL86.union(DialectSet::SYNOPSYS),
        disabled_commands: &[],
        base_layers: &[DialectSet::SYNOPSYS],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::SYNOPSYS),
        version_ceiling: Some(TclVersion::V8_6),
        signature_base: Some(TclVersion::V8_6),
        runtime_base: Some(TclVersion::V8_6),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_6),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    DialectProfile {
        name: "tcl8.4",
        aliases: &[],
        vendor_bit: None,
        availability_mask: DialectSet::TCL84,
        disabled_commands: &[],
        // The version "pack" loads no specs, but registering the version bit
        // on the registry preserves its `loaded_dialects` introspection
        // (e.g. `CommandRegistry::leading_zero_is_octal`) exactly as
        // `DialectSet::parse` + `load_dialect` always did.
        base_layers: &[DialectSet::TCL84],
        grammar_union: DialectSet::ALL_TCL,
        version_ceiling: Some(TclVersion::V8_4),
        signature_base: Some(TclVersion::V8_4),
        runtime_base: Some(TclVersion::V8_4),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_4),
        grammar: GRAMMAR_TCL84,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    DialectProfile {
        name: "tcl8.5",
        aliases: &[],
        vendor_bit: None,
        availability_mask: DialectSet::TCL85,
        disabled_commands: &[],
        // The version "pack" loads no specs, but registering the version bit
        // on the registry preserves its `loaded_dialects` introspection
        // (e.g. `CommandRegistry::leading_zero_is_octal`) exactly as
        // `DialectSet::parse` + `load_dialect` always did.
        base_layers: &[DialectSet::TCL85],
        grammar_union: DialectSet::ALL_TCL,
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    DialectProfile {
        name: "tcl8.6",
        aliases: &[],
        vendor_bit: None,
        availability_mask: DialectSet::TCL86,
        disabled_commands: &[],
        // The version "pack" loads no specs, but registering the version bit
        // on the registry preserves its `loaded_dialects` introspection
        // (e.g. `CommandRegistry::leading_zero_is_octal`) exactly as
        // `DialectSet::parse` + `load_dialect` always did.
        base_layers: &[DialectSet::TCL86],
        grammar_union: DialectSet::ALL_TCL,
        version_ceiling: Some(TclVersion::V8_6),
        signature_base: Some(TclVersion::V8_6),
        runtime_base: Some(TclVersion::V8_6),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_6),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    DialectProfile {
        name: "tcl9.0",
        aliases: &[],
        vendor_bit: None,
        availability_mask: DialectSet::TCL90,
        disabled_commands: &[],
        // The version "pack" loads no specs, but registering the version bit
        // on the registry preserves its `loaded_dialects` introspection
        // (e.g. `CommandRegistry::leading_zero_is_octal`) exactly as
        // `DialectSet::parse` + `load_dialect` always did.
        base_layers: &[DialectSet::TCL90],
        grammar_union: DialectSet::ALL_TCL,
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
    },
    // Tag-level `TCL90_PLUS` unions already give 9.1 its 9.0 inheritance,
    // so the exact bit keeps per-version gating precise.
    DialectProfile {
        name: "tcl9.1",
        aliases: &[],
        vendor_bit: None,
        availability_mask: DialectSet::TCL91,
        disabled_commands: &[],
        // The version "pack" loads no specs, but registering the version bit
        // on the registry preserves its `loaded_dialects` introspection
        // (e.g. `CommandRegistry::leading_zero_is_octal`) exactly as
        // `DialectSet::parse` + `load_dialect` always did.
        base_layers: &[DialectSet::TCL91],
        grammar_union: DialectSet::ALL_TCL,
        version_ceiling: Some(TclVersion::V9_1),
        signature_base: Some(TclVersion::V9_1),
        runtime_base: Some(TclVersion::V9_1),
        leading_zero_is_octal: Ternary::No,
        expr_grammar_base: Some(TclVersion::V9_1),
        grammar: GRAMMAR_TCL9X,
        operators_as_commands: true,
        tcloo: true,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
    DialectProfile {
        name: "xilinx-eda-tcl",
        aliases: &[],
        vendor_bit: Some(DialectSet::XILINX),
        availability_mask: DialectSet::TCL85.union(DialectSet::XILINX),
        disabled_commands: &[],
        base_layers: &[DialectSet::XILINX],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::XILINX),
        version_ceiling: Some(TclVersion::V8_5),
        signature_base: Some(TclVersion::V8_5),
        runtime_base: Some(TclVersion::V8_5),
        leading_zero_is_octal: Ternary::Yes,
        expr_grammar_base: Some(TclVersion::V8_5),
        grammar: GRAMMAR_TCL8X,
        operators_as_commands: true,
        tcloo: false,
        has_fixed_ensembles: false,
        vm_runtime_version: TclVersion::V9_0,
    },
];

/// The single sink for every unparseable / typo / unset dialect string.
/// Deliberately permissive so an unknown dialect never flags valid code
/// (design doc §8): full `ALL_TCL` availability, nothing disabled, no pack,
/// inert octal policy, no expr-grammar opinion, modern lexing grammar.
static PLAIN_TCL: DialectProfile = DialectProfile {
    name: "tcl",
    aliases: &[],
    vendor_bit: None,
    availability_mask: DialectSet::ALL_TCL,
    disabled_commands: &[],
    base_layers: &[],
    grammar_union: DialectSet::ALL_TCL,
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
};

impl DialectProfile {
    /// The full catalog of canonical dialect profiles, in sorted-name order
    /// (the [`KNOWN_DIALECTS`](crate::KNOWN_DIALECTS) order). Excludes the
    /// [`Self::plain_tcl`] fallback — it is a resolution sink, not a
    /// selectable dialect.
    #[must_use]
    pub fn all() -> &'static [DialectProfile] {
        &CATALOG
    }

    /// Resolve a dialect-name string to its interned profile,
    /// alias-normalised; every unknown name resolves to the permissive
    /// [`Self::plain_tcl`] fallback (never fails).
    #[must_use]
    pub fn by_name(name: &str) -> &'static DialectProfile {
        Self::find(name).unwrap_or(&PLAIN_TCL)
    }

    /// Resolve an *optional* dialect-name string: `None` and unknown names
    /// both land on [`Self::plain_tcl`]. The ingest-boundary form of
    /// [`Self::by_name`] for callers holding `Option<&str>`.
    #[must_use]
    pub fn by_opt_name(name: Option<&str>) -> &'static DialectProfile {
        name.map_or(&PLAIN_TCL, Self::by_name)
    }

    /// Like [`Self::by_name`] but distinguishing "unknown" from a real
    /// profile: returns `None` for a name that is neither a canonical
    /// dialect name nor a registered alias.
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

    /// Whether this profile's disable list bans `name` (the subtractive
    /// filter every availability consumer applies after its mask query —
    /// §9 of the design doc). Compares the *registry spec name*; callers
    /// normalise any `::` qualification before asking.
    #[must_use]
    pub fn is_command_disabled(&self, name: &str) -> bool {
        // Binary search is possible (the list is sorted) but a 49-entry
        // linear scan of `&str`s is already cheaper than the hash lookups
        // around it, and only iRules has a non-empty list at all.
        self.disabled_commands.contains(&name)
    }

    /// The version-aware *compile-time fold* projection — the drop-in
    /// replacement for `TclVersion::from_dialect` and deliberately
    /// **bit-identical** to it: `Some` only for the plain versioned-Tcl
    /// profiles, `None` for every vendor dialect (including iRules, whose
    /// [`Self::runtime_base`] is a real `V8_4`) so versioned const-folds
    /// keep returning the dialect-invariant subset there until the
    /// optimiser/SCCP output is verified against real 8.4/8.5/8.6
    /// interpreters (design doc §12 Milestone 3 guardrail). The modelled
    /// runtime is `runtime_base`; this accessor is the *fold* policy.
    #[must_use]
    pub fn const_fold_version(&self) -> Option<TclVersion> {
        TclVersion::from_dialect(Some(self.name))
    }
}

#[cfg(test)]
mod tests {
    use super::DialectProfile;
    use crate::dialect_set::{DialectSet, KNOWN_DIALECTS};
    use crate::grammar::BracedVarStyle;
    use crate::version::{TclVersion, Ternary};

    #[test]
    fn catalog_matches_known_dialects_exactly() {
        // One profile per canonical name, in the same (sorted) order —
        // KNOWN_DIALECTS and the catalog can never drift apart.
        let names: Vec<&str> = DialectProfile::all().iter().map(|p| p.name).collect();
        assert_eq!(names.as_slice(), KNOWN_DIALECTS);
    }

    #[test]
    fn by_name_resolves_canonical_names_to_themselves() {
        for &name in KNOWN_DIALECTS {
            assert_eq!(DialectProfile::by_name(name).name, name);
        }
    }

    #[test]
    fn unknown_names_sink_to_plain_tcl() {
        for unknown in ["", "nonsense", "tcl8.7", "TCL8.6"] {
            let p = DialectProfile::by_name(unknown);
            assert!(
                std::ptr::eq(p, DialectProfile::plain_tcl()),
                "{unknown:?} must resolve to the PLAIN_TCL sink"
            );
        }
        assert!(DialectProfile::find("nonsense").is_none());
    }

    #[test]
    fn by_opt_name_treats_none_as_plain_tcl() {
        assert!(std::ptr::eq(
            DialectProfile::by_opt_name(None),
            DialectProfile::plain_tcl()
        ));
        assert_eq!(DialectProfile::by_opt_name(Some("expect")).name, "expect");
    }

    #[test]
    fn irules_handle_is_the_catalog_entry() {
        let via_handle = DialectProfile::irules();
        let via_name = DialectProfile::by_name("f5-irules");
        assert!(std::ptr::eq(via_handle, via_name));
        assert_eq!(via_handle.name, "f5-irules");
        assert!(via_handle.is_irules());
        assert!(!DialectProfile::by_name("tcl8.4").is_irules());
    }

    #[test]
    fn profiles_are_interned_pointer_identities() {
        assert!(std::ptr::eq(
            DialectProfile::by_name("tcl8.6"),
            DialectProfile::by_name("tcl8.6")
        ));
        assert!(!std::ptr::eq(
            DialectProfile::by_name("tcl8.6"),
            DialectProfile::by_name("tcl9.0")
        ));
    }

    #[test]
    fn irules_aliases_canonicalise_to_the_same_profile() {
        // §2.4: `irules` / `tcl-irule` must resolve to the f5-irules
        // profile so profile predicates can never disagree with the
        // canonical spelling.
        for alias in ["irules", "tcl-irule"] {
            let p = DialectProfile::by_name(alias);
            assert!(p.is_irules(), "{alias:?} must canonicalise to f5-irules");
        }
    }

    #[test]
    fn irules_mask_is_the_bare_vendor_bit_with_disables() {
        // The subtractive-iRules trap (§9): the mask must stay the bare
        // IRULES bit — a TCL84|IRULES union would re-admit the disabled
        // commands — and the disable list must be present and sorted.
        let p = DialectProfile::irules();
        assert_eq!(p.availability_mask, DialectSet::IRULES);
        assert_eq!(p.disabled_commands.len(), 50);
        let mut sorted = p.disabled_commands.to_vec();
        sorted.sort_unstable();
        assert_eq!(p.disabled_commands, sorted.as_slice(), "list stays sorted");
        assert!(p.is_command_disabled("exec"));
        assert!(p.is_command_disabled("file"));
        assert!(p.is_command_disabled("socket"));
        assert!(!p.is_command_disabled("pool"));
        assert!(!p.is_command_disabled("set"));
    }

    #[test]
    fn only_irules_is_subtractive() {
        for p in DialectProfile::all() {
            if p.is_irules() {
                continue;
            }
            assert!(
                p.disabled_commands.is_empty(),
                "{}: only iRules carries a disable list",
                p.name
            );
        }
        assert!(
            DialectProfile::plain_tcl().disabled_commands.is_empty(),
            "the fallback sink disables nothing"
        );
    }

    #[test]
    fn additive_vendor_masks_compose_base_version_and_vendor_bit() {
        // §7: the composed (version|vendor) masks the W123/W002 fix rests on.
        let cases: &[(&str, DialectSet, DialectSet)] = &[
            ("f5-iapps", DialectSet::TCL85, DialectSet::IAPPS),
            ("expect", DialectSet::TCL86, DialectSet::EXPECT),
            ("synopsys-eda-tcl", DialectSet::TCL86, DialectSet::SYNOPSYS),
            ("cadence-eda-tcl", DialectSet::TCL86, DialectSet::CADENCE),
            ("xilinx-eda-tcl", DialectSet::TCL85, DialectSet::XILINX),
            (
                "intel-quartus-eda-tcl",
                DialectSet::TCL85,
                DialectSet::QUARTUS,
            ),
            ("mentor-eda-tcl", DialectSet::TCL85, DialectSet::MENTOR),
        ];
        for &(name, base, vendor) in cases {
            let p = DialectProfile::by_name(name);
            assert_eq!(p.availability_mask, base | vendor, "{name}");
        }
    }

    #[test]
    fn plain_tcl_versions_keep_their_exact_bit() {
        for (name, bit) in [
            ("tcl8.4", DialectSet::TCL84),
            ("tcl8.5", DialectSet::TCL85),
            ("tcl8.6", DialectSet::TCL86),
            ("tcl9.0", DialectSet::TCL90),
            ("tcl9.1", DialectSet::TCL91),
        ] {
            assert_eq!(DialectProfile::by_name(name).availability_mask, bit);
        }
    }

    #[test]
    fn config_only_dialects_stay_permissive_until_first_class() {
        // f5-tmsh / f5-bigip (Milestone 6, D8) must not regress before their
        // reverse-regression golden budget lands.
        for name in ["f5-tmsh", "f5-bigip"] {
            assert_eq!(
                DialectProfile::by_name(name).availability_mask,
                DialectSet::ALL_TCL,
                "{name}"
            );
        }
        // bpf keeps a strictly-widening interim mask until D7 lands in M6.
        assert_eq!(
            DialectProfile::by_name("bpf").availability_mask,
            DialectSet::ALL_TCL | DialectSet::BPF
        );
    }

    #[test]
    fn grammar_union_is_never_narrower_than_the_mask() {
        // §10: the static-grammar union over-approximates (or equals, for
        // subtractive iRules) the precise mask — never under-approximates.
        for p in DialectProfile::all() {
            assert!(
                p.grammar_union.contains(p.availability_mask),
                "{}: grammar_union must cover availability_mask",
                p.name
            );
        }
    }

    #[test]
    fn irules_grammar_union_is_the_shipped_bare_bit_fix() {
        // The shipped iRules highlight fix is literally
        // `grammar_union == availability_mask == IRULES` (§9.1).
        let p = DialectProfile::irules();
        assert_eq!(p.grammar_union, DialectSet::IRULES);
    }

    // Behaviour axis (Milestone 3) — the §7.1 derivation rules as
    // invariants, so the hand-laid table can never drift from the model.

    fn all_with_fallback() -> impl Iterator<Item = &'static DialectProfile> {
        DialectProfile::all()
            .iter()
            .chain(std::iter::once(DialectProfile::plain_tcl()))
    }

    #[test]
    fn vendor_bit_composes_the_availability_mask() {
        // §2.2: for the additive vendor profiles the mask is exactly
        // (version bits | vendor_bit); for the subtractive iRules profile
        // the mask IS the bare vendor bit; profiles without a vendor bit
        // carry pure version masks.
        for p in all_with_fallback() {
            match p.vendor_bit {
                Some(bit) => {
                    assert!(
                        p.availability_mask.contains(bit),
                        "{}: mask must contain the vendor bit",
                        p.name
                    );
                    let version_half = p.availability_mask.difference(bit);
                    assert!(
                        DialectSet::ALL_TCL.contains(version_half),
                        "{}: the non-vendor half of the mask is version bits",
                        p.name
                    );
                    if p.is_irules() {
                        assert_eq!(p.availability_mask, bit, "iRules mask is BARE (§9)");
                    }
                }
                None => {
                    assert!(
                        DialectSet::ALL_TCL.contains(p.availability_mask),
                        "{}: no vendor bit — the mask is pure Tcl versions",
                        p.name
                    );
                }
            }
        }
    }

    #[test]
    fn version_ceiling_tracks_the_signature_base() {
        // §7: the option-gating ceiling is the signature base everywhere a
        // profile has one — except the interim-permissive config-only
        // dialects (no ceiling until Milestone 6 makes them first-class)
        // and the permissive fallback.
        for p in all_with_fallback() {
            match p.name {
                "f5-tmsh" | "f5-bigip" | "tcl" => {
                    assert_eq!(p.version_ceiling, None, "{}: interim-permissive", p.name);
                }
                _ => {
                    assert_eq!(
                        p.version_ceiling, p.signature_base,
                        "{}: ceiling == signature base",
                        p.name
                    );
                }
            }
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
    fn expr_grammar_matches_the_retired_string_table() {
        // The profile's expr_grammar_base must agree with the (still
        // shipping) `DialectSet::expr_grammar_base_version` string table
        // for every canonical name it has an arm for — one source of
        // truth, two projections, zero drift.
        for p in DialectProfile::all() {
            let legacy = DialectSet::expr_grammar_base_version(p.name);
            let via_profile = p.expr_grammar_base.map(|v| match v {
                TclVersion::V8_4 => DialectSet::TCL84,
                TclVersion::V8_5 => DialectSet::TCL85,
                TclVersion::V8_6 => DialectSet::TCL86,
                TclVersion::V9_0 => DialectSet::TCL90,
                TclVersion::V9_1 => DialectSet::TCL91,
            });
            // bpf is the one deliberate divergence: the legacy table had no
            // arm (None), the profile models its Tcl 9.0 core (D7).
            if p.name == "bpf" {
                assert_eq!(legacy, None);
                assert_eq!(via_profile, Some(DialectSet::TCL90));
                continue;
            }
            assert_eq!(via_profile, legacy, "{}", p.name);
        }
    }

    #[test]
    fn tcloo_is_invariant_with_the_availability_mask() {
        // §11.2: the hand-filled tcloo bool must agree with what the mask
        // resolves for `oo::*` (gated ~TCL86_PLUS) — otherwise hover and
        // the oo handler would contradict each other. Documented exception:
        // f5-bigip has NO Tcl surface (tcloo false is the model truth,
        // §7) while its availability mask stays interim-permissive until
        // Milestone 6 makes it first-class — the mask side of the
        // invariant tightens there.
        for p in all_with_fallback() {
            if p.name == "f5-bigip" {
                assert!(!p.tcloo, "f5-bigip has no Tcl surface at all");
                continue;
            }
            assert_eq!(
                p.tcloo,
                p.availability_mask.intersects(DialectSet::TCL86_PLUS),
                "{}: tcloo must match the mask-resolved oo::* availability",
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
    fn operators_are_commands_everywhere_but_irules() {
        // §9: the math-operator heads exist in every command dialect except
        // iRules (operators live only inside expr there). `tk` is a
        // library pin, not a profile, so it does not appear here. f5-bigip
        // has no command surface at all.
        for p in all_with_fallback() {
            let expected = !(p.is_irules() || p.name == "f5-bigip");
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
            assert_eq!(
                p.grammar.irules_brace_separator,
                p.is_irules(),
                "{}",
                p.name
            );
        }
    }

    #[test]
    fn expect_and_tmsh_lex_braced_vars_with_the_8x_rule() {
        // The Milestone 3 flip the old string-keyed lexer table missed:
        // expect (8.6) and f5-tmsh (8.5) are 8.x runtimes, so `${a{b}c}`
        // names `a{b` — not the Tcl 9 nesting read.
        for name in ["expect", "f5-tmsh"] {
            assert_eq!(
                DialectProfile::by_name(name).grammar.braced_var,
                BracedVarStyle::FirstClose,
                "{name}"
            );
        }
        // bpf embeds Tcl 9.0: nesting, unchanged.
        assert_eq!(
            DialectProfile::by_name("bpf").grammar.braced_var,
            BracedVarStyle::Tcl9Nesting
        );
    }

    #[test]
    fn const_fold_version_stays_bit_identical_to_from_dialect() {
        // Milestone 3 guardrail (§12): versioned const-folds keep the
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
            DialectProfile::by_name("tcl8.4").const_fold_version(),
            Some(TclVersion::V8_4)
        );
    }
}
