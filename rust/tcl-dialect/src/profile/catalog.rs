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

//! The catalogue data: the library pins and lexing grammars the profiles share, one profile per canonical dialect, the plain-Tcl fallback and the Tk ingress.

use super::{DialectFileExtension, DialectProfile, EvaluationEvidence};
use crate::grammar::{
    BraceBackslashNewline, BraceLineContinuation, BracedVarStyle, EscapeSyntax, ExprCommentStyle,
    LexerGrammar, ListParse, NumberSyntax, QuoteTermination, SourceLineEndings, VarSyntax,
    WordSeparators,
};
use crate::library::{LibraryPin, LibraryVersion, VersionKey};
use crate::model::{Family, PackageFloor, SpecProvider, SurfaceLayer};
use crate::version::{TclVersion, Ternary};

/// Library pins for the 8.4/8.5-era plain Tcl profiles: Tk tracks the
/// embedded base (`wish` 8.5 ships Tk 8.5), Itcl ships the 3.x line.
///
/// tcllib is deliberately NOT pinned here (despite the §7 table's loose
/// "tcllib `TracksBase`"): tcllib packages version on their own axes
/// (`struct 1.4`, not the Tcl release), so a Tcl-base floor would compare
/// apples to oranges — hostability for tcllib is already permissive, and
/// its version floors keep coming from explicit `package require` alone.
pub(super) const LIBS_TCL84_85: &[LibraryPin] = &[
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
pub(super) const LIBS_TCL86_PLUS: &[LibraryPin] = &[
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

/// Library pins for the `tk` environment: `wish` has loaded the Tk that
/// ships with its 8.6 core before the script runs, so Tk is ambient there
/// and Itcl stays hosted, as the environment's placements state.
pub(super) const LIBS_TK: &[LibraryPin] = &[
    LibraryPin {
        package: "Tk",
        version: LibraryVersion::TracksBase,
        ambient: true,
    },
    LibraryPin {
        package: "Itcl",
        version: LibraryVersion::Pinned("4.2"),
        ambient: false,
    },
];

/// The Tcl 8.4 lexing grammar: no `{*}` expansion (TIP 157 is 8.5), the
/// 8.x first-close `${…}` rule, no `expr` comments (TIP 582 is 9.0).
pub(super) const GRAMMAR_TCL84: LexerGrammar = LexerGrammar {
    expand_syntax: false,
    irules_brace_separator: false,
    brace_line_continuation: BraceLineContinuation::Terminates,
    braced_var: BracedVarStyle::FirstClose,
    array_index: crate::ArrayIndexSyntax::Tcl8,
    script_skips_leading_bom: false,
    expr_comments: TclVersion::V8_4.release_policy().expr_comments,
    numbers: TclVersion::V8_4.number_syntax(),
    escapes: TclVersion::V8_4.release_policy().escapes,
    word_separators: WordSeparators::Tcl,
    brace_backslash_newline: BraceBackslashNewline::Folds,
    quote_termination: QuoteTermination::Strict,
    var_syntax: VarSyntax::Tcl,
    list_parse: ListParse::Strict,
    source_line_endings: SourceLineEndings::TranslateAuto,
};

/// The Tcl 8.5 lexing grammar (plain 8.5, iApps, tmsh, the 8.5-based EDA
/// shells): `{*}` expansion, the 8.x first-close `${…}` rule, no `expr`
/// comments (TIP 582 is 9.0), and 8.4's pre-TIP-388 escape grammar.
pub(super) const GRAMMAR_TCL85: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    irules_brace_separator: false,
    brace_line_continuation: BraceLineContinuation::Terminates,
    braced_var: BracedVarStyle::FirstClose,
    array_index: crate::ArrayIndexSyntax::Tcl8,
    script_skips_leading_bom: false,
    expr_comments: TclVersion::V8_5.release_policy().expr_comments,
    numbers: TclVersion::V8_5.number_syntax(),
    escapes: TclVersion::V8_5.release_policy().escapes,
    word_separators: WordSeparators::Tcl,
    brace_backslash_newline: BraceBackslashNewline::Folds,
    quote_termination: QuoteTermination::Strict,
    var_syntax: VarSyntax::Tcl,
    list_parse: ListParse::Strict,
    source_line_endings: SourceLineEndings::TranslateAuto,
};

/// The Tcl 8.6 lexing grammar (plain 8.6, Expect, the 8.6-based EDA shells):
/// 8.5's, except that TIP 388 caps `\x` at two digits and adds `\U` — the one
/// axis on which 8.5 and 8.6 differ.
pub(super) const GRAMMAR_TCL86: LexerGrammar = LexerGrammar {
    expr_comments: TclVersion::V8_6.release_policy().expr_comments,
    numbers: TclVersion::V8_6.number_syntax(),
    escapes: TclVersion::V8_6.release_policy().escapes,
    ..GRAMMAR_TCL85
};

/// The modern 9.x grammar (also the permissive default): `{*}` expansion,
/// Tcl 9's nesting `${…}` rule, TIP 582 `expr` comments.
pub(super) const GRAMMAR_TCL9X: LexerGrammar = LexerGrammar {
    expand_syntax: true,
    irules_brace_separator: false,
    brace_line_continuation: BraceLineContinuation::Terminates,
    braced_var: BracedVarStyle::Tcl9Nesting,
    array_index: crate::ArrayIndexSyntax::Tcl9,
    script_skips_leading_bom: true,
    expr_comments: TclVersion::V9_0.release_policy().expr_comments,
    numbers: TclVersion::V9_0.number_syntax(),
    escapes: TclVersion::V9_0.release_policy().escapes,
    word_separators: WordSeparators::Tcl,
    brace_backslash_newline: BraceBackslashNewline::Folds,
    quote_termination: QuoteTermination::Strict,
    var_syntax: VarSyntax::Tcl,
    list_parse: ListParse::Strict,
    source_line_endings: SourceLineEndings::TranslateAuto,
};

/// The `f5-tcl` **trunk** lexing grammar: a Tcl 8.4 base (no `{*}`, no
/// `expr` comments) plus the two measured fork axes — the implicit word
/// break (R-rules) and the brace-line continuation (N-rules), both
/// live-measured on TMM 21.1.0.1 with same-host stock controls
/// (`docs/design/f5/bigip-irule-parser-measurements.md` §1-§3). Measured
/// **byte-identical in all three BIG-IP execution contexts** (§4a):
/// TMM iRules, `IAppImplementation`, and tmsh `cli script` all reproduce
/// the R-rules, the N-rules, and the inert `{*}`, so this one grammar
/// serves `f5-irules`, `f5-iapps`, and `f5-tmsh` alike — the iRules
/// offshoot overrides no lexical axis. `source_line_endings` keeps the 8.4
/// base's `-translation auto` reading; no oracle decides it for BIG-IP.
pub(super) const GRAMMAR_F5_TCL: LexerGrammar = LexerGrammar {
    expand_syntax: false,
    irules_brace_separator: true,
    brace_line_continuation: BraceLineContinuation::Continues,
    braced_var: BracedVarStyle::FirstClose,
    array_index: crate::ArrayIndexSyntax::Tcl8,
    script_skips_leading_bom: false,
    expr_comments: ExprCommentStyle::None,
    numbers: NumberSyntax::Tcl84,
    escapes: EscapeSyntax::Tcl84,
    word_separators: WordSeparators::Tcl,
    brace_backslash_newline: BraceBackslashNewline::Folds,
    quote_termination: QuoteTermination::Strict,
    var_syntax: VarSyntax::Tcl,
    list_parse: ListParse::Strict,
    source_line_endings: SourceLineEndings::TranslateAuto,
};

/// Where the F5 fork's release was measured: every context reports patchlevel
/// 8.4.6 and behaves as 8.4 on each 8.4 / 8.5 discriminator probed.
pub(super) const F5_FORK_MEASURED: EvaluationEvidence = EvaluationEvidence::MeasuredFork {
    note: "docs/design/f5/bigip-irule-parser-measurements.md §4 and §4a",
};

/// The catalogue: one profile per canonical dialect, in sorted-name order.
///
/// Surface and behaviour values follow the per-dialect table in
/// `docs/design/registry/dialect-profile-model.md` §7.
pub(super) static CATALOG: [DialectProfile; 19] = [
    // bpf embeds a genuine Tcl 9.0 (design doc D7): 9.0 runtime semantics —
    // decimal leading zeros, 9.0 expr grammar, the nesting `${…}` rule —
    // and a precise point: 9.0 core plus the bpf surface resolve, while
    // 8.x-only relics (removed at the 9.0 boundary) are correctly unknown.
    DialectProfile {
        core_point: None,
        name: "bpf",
        aliases: &[],
        display_name: "BPF",
        short_name: "BPF",
        editor_language_id: None,
        filenames: &[],
        file_extensions: &[],
        vendor_surface: Some(SpecProvider::Package("bpf")),
        surface_packages: &[PackageFloor::named("bpf")],
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
        libraries: &[],
        help_terms: &["bpf", "ebpf"],
    },
    DialectProfile {
        core_point: None,
        name: "cadence-eda-tcl",
        aliases: &["genus", "innovus"],
        display_name: "Cadence Genus / Innovus / Xcelium",
        short_name: "Cadence",
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
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
    // which a plain string-keyed lexer table would miss (falling through
    // to the modern-9.x default).
    DialectProfile {
        core_point: None,
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
        surface_packages: &[PackageFloor::named("expect")],
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
        libraries: &[LibraryPin {
            package: "Expect",
            version: LibraryVersion::Pinned("5.45.4"),
            ambient: true,
        }],
        help_terms: &["expect", "spawn", "interact"],
    },
    // f5-bigip is a config parser, not a Tcl surface; it has no command
    // pack, no Tcl runtime (behaviour axis inert — §11.1), and no expr
    // grammar. It is first-class as *identity only*: the bare
    // the `bigip` surface keys the profile and its versioned schema
    // library — BIG-IP config documents route to the tcl-bigip validator,
    // never the Tcl analyser, so this is not a Tcl-availability surface.
    DialectProfile {
        core_point: None,
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
        surface_packages: &[PackageFloor::named("bigip")],
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
        libraries: &[LibraryPin {
            package: "f5-bigip-schema",
            version: LibraryVersion::Keyed(VersionKey::BigipVersion),
            ambient: true,
        }],
        help_terms: &["bigip", "big-ip", "bigip.conf", "f5", "ltm", "gtm"],
    },
    // iApps ride the `f5-tcl` trunk (fork of Tcl at 8.4.6), NOT a real
    // 8.5 host: the 8.5 hypothesis is measured and falsified
    // (`docs/design/f5/bigip-irule-parser-measurements.md` §4a) —
    // `IAppImplementation` reports patchlevel 8.4.6, fails every 8.5
    // discriminator (`dict`, `lassign`, `apply`, `0b101`), and carries
    // the full trunk grammar (R-rules, N-rules, inert `{*}`, expr word
    // operators) byte-identical to TMM. `::tcl::mathop` is measured
    // absent, so operator heads are not commands. Environment deltas
    // (working `exec`, large `package names`, 32-bit `tcl_platform`) are
    // non-grammatical and live on the environment, not here.
    DialectProfile {
        core_point: None,
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
        surface_packages: &[PackageFloor::named("iapps")],
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
        evaluation_evidence: F5_FORK_MEASURED,
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
    // runtime base are both 8.4 and math operators are not command
    // heads.
    DialectProfile {
        core_point: None,
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
        evaluation_evidence: F5_FORK_MEASURED,
        libraries: &[LibraryPin {
            package: "f5-irules-cmds",
            version: LibraryVersion::Keyed(VersionKey::BigipVersion),
            ambient: true,
        }],
        help_terms: &["irules", "irule", "f5", "big-ip", "tmm", "event"],
    },
    // f5-tmsh rides the `f5-tcl` trunk (fork of Tcl at 8.4.6): the
    // previous 8.5/8.5.13 claims are measured and falsified
    // (`docs/design/f5/bigip-irule-parser-measurements.md` §4a) — a
    // `TmshCliScript` reports patchlevel 8.4.6 and reproduces the entire
    // trunk grammar (R-rules, N-rules, inert `{*}`, expr word operators)
    // identically to TMM, and `::tcl::mathop` is measured absent. It is
    // first-class: the tmsh shell hosts the trunk interpreter plus
    // the `tmsh::` surface (shared spec data with iApps, tagged
    // `IAPPS|TMSH`). Environment deltas (working `exec`, empty
    // `tcl_platform`, no `tcl_patchLevel`, `info vartype`) live on the
    // environment, not here.
    DialectProfile {
        core_point: None,
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
        surface_packages: &[PackageFloor::named("tmsh")],
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
        evaluation_evidence: F5_FORK_MEASURED,
        libraries: &[LibraryPin {
            package: "f5-tmsh-cmds",
            version: LibraryVersion::Keyed(VersionKey::BigipVersion),
            ambient: true,
        }],
        help_terms: &["tmsh", "f5", "big-ip", "bigip"],
    },
    DialectProfile {
        core_point: None,
        name: "intel-quartus-eda-tcl",
        aliases: &["quartus"],
        display_name: "Intel Quartus Prime",
        short_name: "Quartus",
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
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
        core_point: None,
        name: "mentor-eda-tcl",
        aliases: &["questa", "modelsim"],
        display_name: "Siemens Questa / ModelSim",
        short_name: "Questa",
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
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
        core_point: None,
        name: "microchip-libero-eda-tcl",
        aliases: &["libero"],
        display_name: "Microchip Libero SoC",
        short_name: "Libero",
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
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
        core_point: None,
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
        surface_packages: &[PackageFloor::named("spectcl")],
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
        libraries: &[],
        help_terms: &["spectcl", "speclib", "tclspec"],
    },
    // SslicTcl — the `.sslictcl` TLS-assurance declaration DSL.
    // A document is an ordinary Tcl script read from the CST and never
    // executed: the loader evaluates nothing, not even a `predicate` body,
    // which it retains verbatim. Like SpecTcl this is an *environment* over
    // Tcl 9.0 rather than a grammar axis — what makes it a dialect is the
    // availability half, the declaration vocabulary (`certificate` /
    // `endpoint` / `policy` / …) that must exist inside a `.sslictcl`
    // document and nowhere else.
    DialectProfile {
        core_point: None,
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
        surface_packages: &[PackageFloor::named("sslictcl")],
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
        libraries: &[],
        help_terms: &["sslictcl", "tls", "certificate", "endpoint"],
    },
    DialectProfile {
        core_point: None,
        name: "synopsys-eda-tcl",
        aliases: &["dc_shell", "primetime"],
        display_name: "Synopsys DC / PrimeTime / ICC2 / Formality",
        short_name: "Synopsys",
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
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
        core_point: None,
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
        evaluation_evidence: EvaluationEvidence::ReferenceToolchain,
        libraries: LIBS_TCL84_85,
        help_terms: &["tcl", "tk"],
    },
    DialectProfile {
        core_point: None,
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
        evaluation_evidence: EvaluationEvidence::ReferenceToolchain,
        libraries: LIBS_TCL84_85,
        help_terms: &["tcl", "tk"],
    },
    DialectProfile {
        core_point: None,
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
        evaluation_evidence: EvaluationEvidence::ReferenceToolchain,
        libraries: LIBS_TCL86_PLUS,
        help_terms: &["tcl", "tk"],
    },
    DialectProfile {
        core_point: None,
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
        evaluation_evidence: EvaluationEvidence::ReferenceToolchain,
        libraries: LIBS_TCL86_PLUS,
        help_terms: &["tcl", "tk"],
    },
    // The 9.0-and-later windows already give 9.1 its 9.0 inheritance, so
    // naming the exact release here keeps per-version gating precise.
    DialectProfile {
        core_point: None,
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
        evaluation_evidence: EvaluationEvidence::ReferenceToolchain,
        libraries: LIBS_TCL86_PLUS,
        help_terms: &["tcl", "tk"],
    },
    DialectProfile {
        core_point: None,
        name: "xilinx-eda-tcl",
        aliases: &["vivado"],
        display_name: "Xilinx Vivado",
        short_name: "Vivado",
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
        evaluation_evidence: EvaluationEvidence::Unmeasured,
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
pub(super) static PLAIN_TCL: DialectProfile = DialectProfile {
    core_point: None,
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
    evaluation_evidence: EvaluationEvidence::Unmeasured,
    libraries: &[],
    help_terms: &[],
};

/// The `TclVersion` a Tcl-family release is, and `None` for every other
/// family — Jim and the F5 forks have no `TclVersion`, exactly as the
/// permissive fallback has none.
pub(super) fn tcl_version_of(release: crate::model::Release) -> Option<TclVersion> {
    crate::model::DialectPoint::tcl_version_of_release(release)
}

/// The one-provider grammar union of a family's own core.
pub(super) const fn core_provider(family: Family) -> &'static [SpecProvider] {
    match family {
        Family::Tcl => &[SpecProvider::Core(Family::Tcl)],
        Family::F5Tcl => &[SpecProvider::Core(Family::F5Tcl)],
        Family::F5Irules => &[SpecProvider::Core(Family::F5Irules)],
        Family::Jim => &[SpecProvider::Core(Family::Jim)],
    }
}

/// Set-only `tk` ingress: Tcl **8.6** behaviour, plus Tk in the point.
///
/// The grammar is the `tk` environment's core (`Release::TCL_8_6`), the
/// same release its library pins (`LIBS_TK`) and its version gating
/// already answer for. A grammar naming a different release than the rest
/// of the row would expose the lexer and codegen disagreeing about one
/// document once resolution is centralised; the agreement test in
/// `grammar.rs` includes this row so the two cannot drift.
/// This is deliberately not part of [`DialectProfile::all`] or
/// [`DialectProfile::find`].
pub(super) static TK_PROFILE: DialectProfile = DialectProfile {
    core_point: None,
    name: "tk",
    aliases: &[],
    display_name: "Tk",
    short_name: "Tk",
    editor_language_id: None,
    filenames: &[],
    file_extensions: &[],
    vendor_surface: None,
    surface_packages: &[PackageFloor::named("Tk")],
    base_layers: &[],
    grammar_union: &[SpecProvider::Core(Family::Tcl), SpecProvider::Package("Tk")],
    version_ceiling: None,
    signature_base: None,
    // The runtime is its 8.6 core: compiled code takes that core's native
    // recipes and release semantics.
    runtime_base: Some(TclVersion::V8_6),
    leading_zero_is_octal: Ternary::Yes,
    expr_grammar_base: Some(TclVersion::V8_6),
    grammar: GRAMMAR_TCL86,
    operators_as_commands: true,
    tcloo: true,
    has_fixed_ensembles: false,
    vm_runtime_version: TclVersion::V8_6,
    evaluation_evidence: EvaluationEvidence::Unmeasured,
    libraries: LIBS_TK,
    help_terms: &["tk"],
};
