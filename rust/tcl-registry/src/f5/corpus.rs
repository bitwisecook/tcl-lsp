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

//! The **F5 conformance corpus** (ruling R11,
//! `docs/design/dialect-and-package-registry-centralisation.md` §4; review
//! finding F8).
//!
//! Hermetic vectors derived from the checked-in appliance transcripts in
//! `scripts/dev/bigip-probes/`. Every row cites the measurements section
//! that reports it, and every row is asserted against the shipping model,
//! so a model change that contradicts measured behaviour fails a test here
//! instead of shipping.
//!
//! ## The two-sided expectation
//!
//! Rows do not merely record what the appliance did — they record how the
//! model is expected to relate to it:
//!
//! - [`ModelExpectation::Agrees`] — the model's answer equals the measured
//!   cell, and must keep doing so.
//! - [`ModelExpectation::Diverges`] — the model's answer differs today,
//!   with a recorded reason. The test asserts the divergence is *exactly
//!   where it is recorded*, so closing one of these gaps fails the row and
//!   forces it to be re-classified deliberately. The six that remain are
//!   all the *correct* kind — a `RULE_INIT` compile acceptance that the
//!   measurements themselves say must not be read as "valid to use". The
//!   sixteen open gaps this corpus made visible on its first run (fifteen
//!   event cells and the missing `matches` operator) were closed in P4 by
//!   moving the model to the measurement, never by weakening a row.
//! - [`ModelExpectation::NotComparable`] — the measured cell has no model
//!   counterpart at all (`static::` is a variable namespace, not a
//!   command).
//!
//! Nothing here runs against an appliance. The re-runnable probe corpus is
//! the owner-run appliance tier
//! (`dialect-and-package-registry-centralisation.md` §7.6); this module is
//! the hermetic CI tier that consumes its transcripts.

use crate::f5::evidence::{DiscriminatorBehaviour, ProbeSetId};
use crate::irules_policy::IrulesDisabledClass;

/// How the shipping model is expected to relate to one measured cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelExpectation {
    /// The model agrees with the appliance, and must keep agreeing.
    Agrees,
    /// The model disagrees, deliberately or as a recorded open gap.
    Diverges(DivergenceReason),
    /// There is no model answer to compare with.
    NotComparable(&'static str),
}

/// Why a model answer differs from the measured cell.
///
/// The first variant is a *correct* difference, and since P4 closed the
/// registry-data gaps it is the only one any row carries. The other three
/// stay as vocabulary rather than being deleted: a corpus that can only
/// say "diverges" cannot say **which way** a future regression hurts, and
/// the direction is the whole point — an over-permissive cell costs the
/// user a missed load-time error, an over-strict one a false positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceReason {
    /// The appliance accepts the command in `RULE_INIT` at **compile
    /// time** and the model rejects it — deliberately. §8 is explicit:
    /// *"`RULE_INIT` permits the `HTTP::*` commands at load but they are
    /// meaningless there … Compile acceptance in `RULE_INIT` should not be
    /// read as 'valid to use'"*, and the same run logged a runtime error
    /// from a probe that had compiled cleanly.
    RuleInitCompileAcceptance,
    /// The model rejects a call the appliance accepts (outside
    /// `RULE_INIT`) — an over-strict row that costs users a false
    /// positive.
    ModelNarrowerThanMeasured,
    /// The model accepts a call the appliance rejects at rule load — an
    /// over-permissive row that costs users a missed error.
    ModelBroaderThanMeasured,
    /// The model does not carry a lexeme, command, or option the
    /// appliance demonstrably has.
    ModelMissingMeasuredSurface,
}

/// One cell of the event-context matrix as the rule compiler answered it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCell {
    /// The rule loaded.
    Accepted,
    /// The rule was rejected with `command is not valid in current event
    /// context (EVENT)`.
    Rejected,
}

impl EventCell {
    /// Whether the appliance accepted the pair.
    #[must_use]
    pub const fn accepted(self) -> bool {
        matches!(self, Self::Accepted)
    }
}

/// The measured result of one probe case in one context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseOutcome {
    /// The case evaluated to this value.
    Value(&'static str),
    /// The case failed with an error of this class.
    Error(&'static str),
}

impl CaseOutcome {
    /// Whether the case evaluated without error.
    #[must_use]
    pub const fn is_value(self) -> bool {
        matches!(self, Self::Value(_))
    }
}

/// The grammar axis that explains one parity row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarAxis {
    /// The implicit word break (R-rules, §1/§3).
    ImplicitWordBreak,
    /// The **gating** of the implicit word break (R5): the rule fires
    /// only when the word *started* with `{` or `"`, so a bare word glued
    /// to a brace stays one word exactly as in stock Tcl. These rows are
    /// identical in every context, and that identity is the evidence —
    /// it is what stops the rule overclaiming `${name}`, `$v{b}` and
    /// `[cmd]{b}`.
    WordBreakGating,
    /// The brace-line continuation (N-rules, §2).
    BraceLineContinuation,
    /// The **bound** of the brace-line continuation (N4): a blank,
    /// whitespace-only, or comment line terminates the command normally,
    /// so these rows fail identically in every context. That identity is
    /// the evidence — it is what keeps N1 from swallowing arbitrary
    /// following lines.
    ContinuationBound,
    /// `{*}` is inert: the separator wins and expansion does not exist.
    InertExpansion,
    /// The 8.4 numeral grammar.
    NumeralGrammar,
    /// The word-form `expr` operators — a trunk fact, not iRules-only.
    ExprWordOperators,
    /// No axis: the row is a control that behaves identically everywhere.
    Unmodified,
}

/// One row of the §4a four-context parity matrix.
///
/// The three F5 contexts were byte-identical on every one of these, which
/// is why a single `f5` column is honest here: the transcript
/// (`results/10-context-parity.txt`) carries all three and the
/// [`PARSER_PARITY_VECTORS`] test asserts the axis that produced them is
/// shared by all three core profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserParityVector {
    /// The probe case id from `suites/10-context-parity.cases`.
    pub case: &'static str,
    /// The Tcl the case ran.
    pub source: &'static str,
    /// The result in all three F5 contexts.
    pub f5: CaseOutcome,
    /// The result under the appliance's `tclsh8.4`.
    pub host84: CaseOutcome,
    /// The result under the appliance's `tclsh8.5`.
    pub host85: CaseOutcome,
    /// The axis that explains the row.
    pub axis: GrammarAxis,
    /// The measurements section reporting it.
    pub section: &'static str,
    /// How the model relates to the row.
    pub expectation: ModelExpectation,
}

/// One row of §4a's "where the contexts genuinely differ" table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextEnvironmentVector {
    /// The property probed.
    pub property: &'static str,
    /// Its value in `TmmIRule`.
    pub tmm: &'static str,
    /// Its value in `TmshCliScript`.
    pub tmsh: &'static str,
    /// Its value in `IAppImplementation`.
    pub iapp: &'static str,
    /// Its value under the host `tclsh8.4` — provenance only.
    pub host84: &'static str,
    /// The measurements section reporting it.
    pub section: &'static str,
}

/// What the model can be asked about one 8.4/8.5 discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelProbe {
    /// The feature is a command: the registry must not offer it under any
    /// F5 environment.
    Command(&'static str),
    /// The feature is a subcommand, option, operator, or namespace — not a
    /// command-level fact, so the command registry cannot answer it. The
    /// prose says where the fact belongs instead.
    NotCommandLevel(&'static str),
}

/// One of the sixteen features that cleanly separate stock 8.4 from stock
/// 8.5 (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseDiscriminatorVector {
    /// The probe case id from `suites/09-tcl85-features.tcl`.
    pub probe_case: &'static str,
    /// Human-readable feature name as §4's table spells it.
    pub feature: &'static str,
    /// Present in stock 8.4?
    pub tcl84: bool,
    /// Present in stock 8.5?
    pub tcl85: bool,
    /// Measured behaviour in `TmshCliScript`.
    pub tmsh: DiscriminatorBehaviour,
    /// Measured behaviour in `IAppImplementation`.
    pub iapp: DiscriminatorBehaviour,
    /// What the model can be asked.
    pub probe: ModelProbe,
    /// The measurements section reporting it.
    pub section: &'static str,
}

/// One of the 31 commands the iRules rule compiler refuses, with the
/// mechanism §4b's runtime re-probe attributed it to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandClassVector {
    /// The command name.
    pub command: &'static str,
    /// Which mechanism refuses it.
    pub class: IrulesDisabledClass,
    /// Whether `eval`-ing it at runtime reaches a real command.
    pub runtime_reachable: bool,
    /// The measurements section reporting it.
    pub section: &'static str,
}

/// One cell of the 15-command × 8-event validity matrix (§8).
///
/// The citation is a shared associated constant rather than a per-row
/// field: the whole matrix is one probe sweep
/// (`results/07-event-context.tsv`), so 120 copies of the same literal
/// would be noise rather than provenance. [`EventContextVector::section`]
/// answers it per row all the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventContextVector {
    /// The command probed.
    pub command: &'static str,
    /// The event it was probed in.
    pub event: &'static str,
    /// What the rule compiler did.
    pub measured: EventCell,
    /// How the model relates to the cell.
    pub expectation: ModelExpectation,
}

impl EventContextVector {
    /// The measurements section reporting the matrix.
    pub const SECTION: &'static str = "§8";
    /// The probe set every row came from.
    pub const PROBE_SET: ProbeSetId = ProbeSetId::EVENT_CONTEXT;

    /// This row's measurements citation.
    #[must_use]
    pub const fn section(self) -> &'static str {
        Self::SECTION
    }
}

/// One measured fact about `when` handler priority (§6/§8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityCase {
    /// A priority value the rule compiler accepted or rejected. Rejection
    /// is reported misleadingly as `unexpected extra argument "1001"`
    /// rather than as a range error, which is itself worth pinning.
    Bound {
        /// The value written in the `priority` clause.
        value: i64,
        /// Whether the rule loaded.
        accepted: bool,
    },
    /// One rule in the traffic lab's ordering experiment: three rules
    /// attached as `{ lab_p_low lab_p_default lab_p_high }` executed in
    /// priority order, proving lower-runs-first and the 500 default.
    Ordering {
        /// The lab rule's name.
        rule: &'static str,
        /// The value its `priority` clause declared, if any.
        declared: Option<u16>,
        /// The priority it effectively ran at.
        effective: u16,
        /// Its position in the observed execution order, 0 first.
        rank: u8,
    },
}

/// One priority vector with its citation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriorityVector {
    /// The measured case.
    pub case: PriorityCase,
    /// The measurements section reporting it.
    pub section: &'static str,
}

use CaseOutcome::{Error, Value};
use DiscriminatorBehaviour::{BehavesAs84, FalsePass};
use GrammarAxis::{
    BraceLineContinuation, ContinuationBound, ExprWordOperators, ImplicitWordBreak, InertExpansion,
    NumeralGrammar, Unmodified, WordBreakGating,
};
use IrulesDisabledClass::{CompilerRefused, InterpreterAbsent};
use ModelExpectation::Agrees;

/// The §4a parity matrix: one case list, four wrappers, so every
/// difference is a real context difference.
///
/// All 21 rows were identical in `TmmIRule`, `TmshCliScript` and
/// `IAppImplementation` — that identity is the evidence that the fork
/// grammar belongs to the `f5-tcl` **trunk** rather than to iRules.
pub const PARSER_PARITY_VECTORS: &[ParserParityVector] = &[
    ParserParityVector {
        case: "g_if_control",
        source: "if {1} {expr {6*7}}",
        f5: Value("42"),
        host84: Value("42"),
        host85: Value("42"),
        axis: Unmodified,
        section: "§4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "g_if_chain",
        source: "if {1}{expr {6*7}}",
        f5: Value("43"),
        host84: Error("extra characters after close-brace"),
        host85: Error("extra characters after close-brace"),
        axis: ImplicitWordBreak,
        section: "§3, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "g_list_chain",
        source: "list {a}{b}",
        f5: Value("a b"),
        host84: Error("extra characters after close-brace"),
        host85: Error("extra characters after close-brace"),
        axis: ImplicitWordBreak,
        section: "§3, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "g_set_chain",
        source: "set zq {a}{b}",
        f5: Error("wrong # args"),
        host84: Error("extra characters after close-brace"),
        host85: Error("extra characters after close-brace"),
        axis: ImplicitWordBreak,
        section: "§3, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "g_cmd_glued",
        source: "if{1}{expr {6*7}}",
        f5: Error("invalid command name"),
        host84: Error("invalid command name"),
        host85: Error("invalid command name"),
        axis: WordBreakGating,
        section: "§3 row 5, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "g_expansion",
        source: "list {*}{a b}",
        f5: Value("* {a b}"),
        host84: Error("extra characters after close-brace"),
        host85: Value("a b"),
        axis: InertExpansion,
        section: "§1, §3 row 6, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "g_quote_chain",
        source: "list \"a\"b",
        f5: Value("a b"),
        host84: Error("extra characters after close-quote"),
        host85: Error("extra characters after close-quote"),
        axis: ImplicitWordBreak,
        section: "§1, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "g_dollarbrace",
        source: "list ${zz}b",
        f5: Value("XXb"),
        host84: Value("XXb"),
        host85: Value("XXb"),
        axis: WordBreakGating,
        section: "§1 R5, §4a, §7",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "n_if_nextline",
        source: "if {1}\\n{…}",
        f5: Value("45"),
        host84: Error("wrong # args"),
        host85: Error("wrong # args"),
        axis: BraceLineContinuation,
        section: "§2, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "n_while_nextline",
        source: "while {$n<30}\\n{…}",
        f5: Value("31"),
        host84: Error("wrong # args"),
        host85: Error("wrong # args"),
        axis: BraceLineContinuation,
        section: "§2, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "n_else_nextline",
        source: "if {0} {…}\\nelse {…}",
        f5: Value("else"),
        host84: Error("invalid command name \"else\""),
        host85: Error("invalid command name \"else\""),
        axis: BraceLineContinuation,
        section: "§2 N5, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "n_list_absorb",
        source: "list a b\\n{c}",
        f5: Value("a b c"),
        host84: Error("invalid command name \"c\""),
        host85: Error("invalid command name \"c\""),
        axis: BraceLineContinuation,
        section: "§2 N2, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "n_blank_breaks",
        source: "if {1}\\n\\n{…}",
        f5: Error("wrong # args"),
        host84: Error("wrong # args"),
        host85: Error("wrong # args"),
        axis: ContinuationBound,
        section: "§2 N4, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "n_comment_breaks",
        source: "if {1}\\n# c\\n{…}",
        f5: Error("wrong # args"),
        host84: Error("wrong # args"),
        host85: Error("wrong # args"),
        axis: ContinuationBound,
        section: "§2 N4, §4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "e_starts_with",
        source: "expr {\"abc\" starts_with \"a\"}",
        f5: Value("1"),
        host84: Error("syntax error in expression"),
        host85: Error("invalid bareword"),
        axis: ExprWordOperators,
        section: "§4a, §6",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "e_contains",
        source: "expr {\"abc\" contains \"b\"}",
        f5: Value("1"),
        host84: Error("syntax error in expression"),
        host85: Error("invalid bareword"),
        axis: ExprWordOperators,
        section: "§4a, §6",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "e_matches",
        source: "expr {\"abc\" matches \"abc\"}",
        f5: Value("1"),
        host84: Error("syntax error in expression"),
        host85: Error("invalid bareword"),
        axis: ExprWordOperators,
        section: "§4a, §4b, §6",
        // CLOSED in P4. The trunk's word-operator table carried nine
        // operators and the bare `matches` was not one of them, though
        // the appliance answered `1` for it in all three F5 contexts and
        // both host builds rejected it. It is the tenth operator now, at
        // the equality level beside `matches_glob` — a *precedence* the
        // transcripts do not pin (a single-operator expression exercises
        // no binding power), so it takes its siblings' class and §12
        // keeps the discriminating re-probe open.
        expectation: Agrees,
    },
    ParserParityVector {
        case: "e_and_word",
        source: "expr {1 and 1}",
        f5: Value("1"),
        host84: Error("syntax error in expression"),
        host85: Error("invalid bareword"),
        axis: ExprWordOperators,
        section: "§4a, §6",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "e_adjacent_eq",
        source: "expr {[string length \"xy\"]eq\"2\"}",
        f5: Value("1"),
        host84: Value("1"),
        host85: Value("1"),
        axis: Unmodified,
        section: "§7",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "num_octal",
        source: "expr {010}",
        f5: Value("8"),
        host84: Value("8"),
        host85: Value("8"),
        axis: NumeralGrammar,
        section: "§4a",
        expectation: Agrees,
    },
    ParserParityVector {
        case: "num_binary",
        source: "expr {0b101}",
        f5: Error("syntax error in expression"),
        host84: Error("syntax error in expression"),
        host85: Value("5"),
        axis: NumeralGrammar,
        section: "§4a",
        expectation: Agrees,
    },
];

/// §4a's "where the contexts genuinely differ" table — the environment
/// deltas that make [`BigIpExecutionContext`] a real key.
pub const CONTEXT_ENVIRONMENT_VECTORS: &[ContextEnvironmentVector] = &[
    ContextEnvironmentVector {
        property: "info patchlevel",
        tmm: "8.4.6",
        tmsh: "8.4.6",
        iapp: "8.4.6",
        host84: "8.4.13",
        section: "§4a",
    },
    ContextEnvironmentVector {
        property: "tcl_patchLevel",
        tmm: "8.4.6",
        tmsh: "UNSET",
        iapp: "8.4.6",
        host84: "8.4.13",
        section: "§4a",
    },
    ContextEnvironmentVector {
        property: "tmsh::version",
        tmm: "n/a",
        tmsh: "21.1.0.1",
        iapp: "21.1.0.1",
        host84: "n/a",
        section: "§4a",
    },
    ContextEnvironmentVector {
        property: "llength [info commands]",
        tmm: "152",
        tmsh: "95",
        iapp: "95",
        host84: "85",
        section: "§4a",
    },
    ContextEnvironmentVector {
        property: "tcl_platform keys",
        tmm: "7",
        tmsh: "0",
        iapp: "7",
        host84: "8",
        section: "§4, §4a",
    },
    ContextEnvironmentVector {
        property: "tcl_platform(wordSize)",
        tmm: "8",
        tmsh: "",
        iapp: "4",
        host84: "8",
        section: "§4, §4a",
    },
    ContextEnvironmentVector {
        property: "tcl_platform(machine)",
        tmm: "<hostname>",
        tmsh: "",
        iapp: "x86_64",
        host84: "x86_64",
        section: "§4, §4a",
    },
    ContextEnvironmentVector {
        property: "exec",
        tmm: "absent",
        tmsh: "works",
        iapp: "works",
        host84: "works",
        section: "§4a",
    },
    ContextEnvironmentVector {
        property: "package names",
        tmm: "Tcl",
        tmsh: "Tcl",
        iapp: "tclparser xml::tcl http uri uuencode xslt::libxslt sha256 …",
        host84: "Tcl",
        section: "§4a",
    },
];

/// The sixteen 8.4-vs-8.5 discriminators of §4. All sixteen behaved as 8.4
/// in both contexts they were probed in; `{*}` is the lone apparent pass
/// and it is the implicit-word-break artefact, not expansion.
pub const RELEASE_DISCRIMINATOR_VECTORS: &[ReleaseDiscriminatorVector] = &[
    ReleaseDiscriminatorVector {
        probe_case: "expand_op",
        feature: "{*} expansion",
        tcl84: false,
        tcl85: true,
        tmsh: FalsePass,
        iapp: FalsePass,
        probe: ModelProbe::NotCommandLevel("the `expand_syntax` lexer axis"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "dict",
        feature: "dict",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::Command("dict"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "lassign",
        feature: "lassign",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::Command("lassign"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "apply",
        feature: "apply",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::Command("apply"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "lreverse",
        feature: "lreverse",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::Command("lreverse"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "lrepeat",
        feature: "lrepeat",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::Command("lrepeat"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "string_reverse",
        feature: "string reverse",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("a `string` subcommand, gated by the 8.4 ceiling"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "pow_operator",
        feature: "** operator",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("an expr operator in the precedence table"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "in_operator",
        feature: "in operator",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("an expr word operator"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "ni_operator",
        feature: "ni operator",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("an expr word operator"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "mathop_ns",
        feature: "::tcl::mathop",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("an ensemble namespace, not a bare command name"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "chan_cmd",
        feature: "chan",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::Command("chan"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "switch_matchvar",
        feature: "switch -matchvar",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("a `switch` option, gated by the 8.4 ceiling"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "string_is_wide",
        feature: "string is wideinteger",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("a `string is` class, gated by the 8.4 ceiling"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "info_frame",
        feature: "info frame",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("an `info` subcommand, gated by the 8.4 ceiling"),
        section: "§4",
    },
    ReleaseDiscriminatorVector {
        probe_case: "namespace_ens",
        feature: "namespace ensemble",
        tcl84: false,
        tcl85: true,
        tmsh: BehavesAs84,
        iapp: BehavesAs84,
        probe: ModelProbe::NotCommandLevel("a `namespace` subcommand, gated by the 8.4 ceiling"),
        section: "§4",
    },
];

/// §4b's two mechanisms, one row per refused command: 16 absent from
/// TMM's interpreter, 15 present and refused only by the rule compiler.
pub const COMMAND_CLASS_VECTORS: &[CommandClassVector] = &[
    class("auto_execok", InterpreterAbsent),
    class("auto_import", InterpreterAbsent),
    class("auto_load", InterpreterAbsent),
    class("auto_qualify", InterpreterAbsent),
    class("cd", InterpreterAbsent),
    class("eof", CompilerRefused),
    class("exec", InterpreterAbsent),
    class("exit", InterpreterAbsent),
    class("fblocked", CompilerRefused),
    class("fconfigure", InterpreterAbsent),
    class("fcopy", CompilerRefused),
    class("file", InterpreterAbsent),
    class("flush", CompilerRefused),
    class("gets", CompilerRefused),
    class("glob", InterpreterAbsent),
    class("interp", CompilerRefused),
    class("load", InterpreterAbsent),
    class("namespace", CompilerRefused),
    class("open", InterpreterAbsent),
    class("package", CompilerRefused),
    class("pid", CompilerRefused),
    class("pwd", InterpreterAbsent),
    class("rename", CompilerRefused),
    class("seek", CompilerRefused),
    class("socket", InterpreterAbsent),
    class("source", InterpreterAbsent),
    class("tell", CompilerRefused),
    class("time", CompilerRefused),
    class("unknown", InterpreterAbsent),
    class("update", CompilerRefused),
    class("vwait", CompilerRefused),
];

const fn class(command: &'static str, class: IrulesDisabledClass) -> CommandClassVector {
    CommandClassVector {
        command,
        class,
        runtime_reachable: matches!(class, CompilerRefused),
        section: "§4b",
    }
}

/// The traffic lab's priority findings (§6, §8): the accepted range, the
/// default, and the observed execution order.
pub const PRIORITY_VECTORS: &[PriorityVector] = &[
    PriorityVector {
        case: PriorityCase::Bound {
            value: 0,
            accepted: true,
        },
        section: "§6",
    },
    PriorityVector {
        case: PriorityCase::Bound {
            value: 500,
            accepted: true,
        },
        section: "§6",
    },
    PriorityVector {
        case: PriorityCase::Bound {
            value: 1000,
            accepted: true,
        },
        section: "§6",
    },
    PriorityVector {
        case: PriorityCase::Bound {
            value: 1001,
            accepted: false,
        },
        section: "§6",
    },
    PriorityVector {
        case: PriorityCase::Bound {
            value: -1,
            accepted: false,
        },
        section: "§6",
    },
    PriorityVector {
        case: PriorityCase::Ordering {
            rule: "lab_p_high",
            declared: Some(100),
            effective: 100,
            rank: 0,
        },
        section: "§8",
    },
    PriorityVector {
        case: PriorityCase::Ordering {
            rule: "lab_p_default",
            declared: None,
            effective: 500,
            rank: 1,
        },
        section: "§8",
    },
    PriorityVector {
        case: PriorityCase::Ordering {
            rule: "lab_p_low",
            declared: Some(900),
            effective: 900,
            rank: 2,
        },
        section: "§8",
    },
];

/// The 15-command × 8-event validity matrix of §8, exactly as
/// `results/07-event-context.tsv` recorded it.
///
/// Two caveats travel with this table and are encoded rather than
/// narrated. It is **compile-time only** — `RULE_INIT` accepts `HTTP::*`
/// at load though they are meaningless there — and `static::` is a
/// variable namespace rather than a command, so no command oracle answers
/// it.
pub const EVENT_CONTEXT_VECTORS: &[EventContextVector] = &[
    EventContextVector {
        command: "HTTP::uri",
        event: "RULE_INIT",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Diverges(DivergenceReason::RuleInitCompileAcceptance),
    },
    EventContextVector {
        command: "HTTP::uri",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::uri",
        event: "CLIENT_DATA",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::uri",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::uri",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::uri",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::uri",
        event: "HTTP_RESPONSE",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::uri",
        event: "CLIENT_CLOSED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::status",
        event: "RULE_INIT",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Diverges(DivergenceReason::RuleInitCompileAcceptance),
    },
    EventContextVector {
        command: "HTTP::status",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::status",
        event: "CLIENT_DATA",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::status",
        event: "HTTP_REQUEST",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::status",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::status",
        event: "SERVER_CONNECTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::status",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::status",
        event: "CLIENT_CLOSED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::respond",
        event: "RULE_INIT",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Diverges(DivergenceReason::RuleInitCompileAcceptance),
    },
    EventContextVector {
        command: "HTTP::respond",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::respond",
        event: "CLIENT_DATA",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::respond",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::respond",
        event: "LB_SELECTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::respond",
        event: "SERVER_CONNECTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::respond",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::respond",
        event: "CLIENT_CLOSED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::collect",
        event: "RULE_INIT",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Diverges(DivergenceReason::RuleInitCompileAcceptance),
    },
    EventContextVector {
        command: "HTTP::collect",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::collect",
        event: "CLIENT_DATA",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::collect",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::collect",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::collect",
        event: "SERVER_CONNECTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::collect",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "HTTP::collect",
        event: "CLIENT_CLOSED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::client_addr",
        event: "RULE_INIT",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::client_addr",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::client_addr",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::client_addr",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::client_addr",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::client_addr",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::client_addr",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::client_addr",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::server_addr",
        event: "RULE_INIT",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::server_addr",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::server_addr",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::server_addr",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::server_addr",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::server_addr",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::server_addr",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "IP::server_addr",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::client_port",
        event: "RULE_INIT",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::client_port",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::client_port",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::client_port",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::client_port",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::client_port",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::client_port",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::client_port",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::collect",
        event: "RULE_INIT",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Diverges(DivergenceReason::RuleInitCompileAcceptance),
    },
    EventContextVector {
        command: "TCP::collect",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::collect",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::collect",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::collect",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::collect",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::collect",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::collect",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::payload",
        event: "RULE_INIT",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Diverges(DivergenceReason::RuleInitCompileAcceptance),
    },
    EventContextVector {
        command: "TCP::payload",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::payload",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::payload",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::payload",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::payload",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::payload",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "TCP::payload",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "LB::server",
        event: "RULE_INIT",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "LB::server",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "LB::server",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "LB::server",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "LB::server",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "LB::server",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "LB::server",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "LB::server",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "SSL::cipher",
        event: "RULE_INIT",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "SSL::cipher",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "SSL::cipher",
        event: "CLIENT_DATA",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "SSL::cipher",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "SSL::cipher",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "SSL::cipher",
        event: "SERVER_CONNECTED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "SSL::cipher",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "SSL::cipher",
        event: "CLIENT_CLOSED",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "pool",
        event: "RULE_INIT",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "pool",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "pool",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "pool",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "pool",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "pool",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "pool",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "pool",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "node",
        event: "RULE_INIT",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "node",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "node",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "node",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "node",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "node",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "node",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "node",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "table",
        event: "RULE_INIT",
        measured: EventCell::Rejected,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "table",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "table",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "table",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "table",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "table",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "table",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "table",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::Agrees,
    },
    EventContextVector {
        command: "static::",
        event: "RULE_INIT",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::NotComparable(
            "a variable namespace, not a command: no command oracle answers it",
        ),
    },
    EventContextVector {
        command: "static::",
        event: "CLIENT_ACCEPTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::NotComparable(
            "a variable namespace, not a command: no command oracle answers it",
        ),
    },
    EventContextVector {
        command: "static::",
        event: "CLIENT_DATA",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::NotComparable(
            "a variable namespace, not a command: no command oracle answers it",
        ),
    },
    EventContextVector {
        command: "static::",
        event: "HTTP_REQUEST",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::NotComparable(
            "a variable namespace, not a command: no command oracle answers it",
        ),
    },
    EventContextVector {
        command: "static::",
        event: "LB_SELECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::NotComparable(
            "a variable namespace, not a command: no command oracle answers it",
        ),
    },
    EventContextVector {
        command: "static::",
        event: "SERVER_CONNECTED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::NotComparable(
            "a variable namespace, not a command: no command oracle answers it",
        ),
    },
    EventContextVector {
        command: "static::",
        event: "HTTP_RESPONSE",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::NotComparable(
            "a variable namespace, not a command: no command oracle answers it",
        ),
    },
    EventContextVector {
        command: "static::",
        event: "CLIENT_CLOSED",
        measured: EventCell::Accepted,
        expectation: ModelExpectation::NotComparable(
            "a variable namespace, not a command: no command oracle answers it",
        ),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventRegistry;
    use crate::f5::evidence::{BigIpBuild, RuntimeFact, RuntimeFactKind, measured_fact};
    use crate::f5::execution_context::BigIpExecutionContext;
    use crate::model::ingress::{resolve_environment, static_context_for};
    use crate::profiles::ProfileRegistry;
    use tcl_dialect::model::family::{Family, Release, grammar};
    use tcl_dialect::model::{VersionAxisId, VersionSet};

    const F5_ENVIRONMENTS: [&str; 3] = ["f5-irules", "f5-tmsh", "f5-iapps"];

    /// The corpus is a fixed, citeable size. A row appearing or vanishing
    /// without a deliberate edit here is drift.
    #[test]
    fn the_corpus_has_its_measured_row_counts() {
        assert_eq!(PARSER_PARITY_VECTORS.len(), 21, "§4a parity cases");
        assert_eq!(CONTEXT_ENVIRONMENT_VECTORS.len(), 9, "§4a environment rows");
        assert_eq!(
            RELEASE_DISCRIMINATOR_VECTORS.len(),
            16,
            "§4's 8.4-vs-8.5 discriminators"
        );
        assert_eq!(COMMAND_CLASS_VECTORS.len(), 31, "§4b's 16 + 15 split");
        assert_eq!(
            EVENT_CONTEXT_VECTORS.len(),
            120,
            "§8's 15 commands × 8 events"
        );
        assert_eq!(PRIORITY_VECTORS.len(), 8, "§6/§8 priority facts");

        for row in PARSER_PARITY_VECTORS {
            assert!(row.section.contains('§'), "{}: no citation", row.case);
        }
        for row in CONTEXT_ENVIRONMENT_VECTORS {
            assert!(row.section.contains('§'), "{}: no citation", row.property);
        }
        for row in RELEASE_DISCRIMINATOR_VECTORS {
            assert!(row.section.contains('§'), "{}: no citation", row.feature);
        }
        for row in COMMAND_CLASS_VECTORS {
            assert!(row.section.contains('§'), "{}: no citation", row.command);
        }
        for row in PRIORITY_VECTORS {
            assert!(row.section.contains('§'));
        }
        for row in EVENT_CONTEXT_VECTORS {
            assert!(row.section().contains('§'), "{}", row.command);
        }
        assert_eq!(
            EventContextVector::PROBE_SET,
            ProbeSetId::EVENT_CONTEXT,
            "the matrix's provenance is the 120-cell sweep"
        );
    }

    /// The expr half of the parity check, extracted so the axis walk stays
    /// readable: the word operator the row exercises must resolve
    /// identically on the trunk and the offshoot, must be absent from
    /// stock 8.4, and must match the row's recorded expectation.
    fn assert_expr_word_operator_row(row: &ParserParityVector) {
        let f5_expr = tcl_dialect::model::expr_grammar::expr(Family::F5Tcl, Release::F5_TCL_TMOS);
        let irules_expr =
            tcl_dialect::model::expr_grammar::expr(Family::F5Irules, Release::F5_IRULES_TMM);
        let host84_expr = tcl_dialect::model::expr_grammar::expr(Family::Tcl, Release::TCL_8_4);
        let operator = match row.case {
            "e_starts_with" => "starts_with",
            "e_contains" => "contains",
            "e_matches" => "matches",
            "e_and_word" => "and",
            other => panic!("{other}: unmapped expr row"),
        };
        let present = f5_expr.has_word_operator(operator);
        assert_eq!(
            present,
            irules_expr.has_word_operator(operator),
            "{}: the offshoot answers from the trunk",
            row.case
        );
        assert!(
            !host84_expr.has_word_operator(operator),
            "{operator} is not stock Tcl"
        );
        match row.expectation {
            ModelExpectation::Agrees => assert!(
                present,
                "{}: measured {:?} on the appliance",
                row.case, row.f5
            ),
            ModelExpectation::Diverges(DivergenceReason::ModelMissingMeasuredSurface) => assert!(
                !present,
                "{}: the recorded gap is closed — reclassify the row as Agrees",
                row.case
            ),
            other => panic!("{}: unexpected expectation {other:?}", row.case),
        }
    }

    /// Each parity row names the grammar axis that produced it, and that
    /// axis must hold in **all three** F5 core profiles (the evidence that
    /// the fork grammar is a trunk fact) while differing at the host build
    /// wherever the row differs there.
    #[test]
    fn parser_parity_rows_are_explained_by_a_shared_trunk_axis() {
        let f5_grammars = [
            grammar(Family::F5Irules, Release::F5_IRULES_TMM),
            grammar(Family::F5Tcl, Release::F5_TCL_TMOS),
        ];
        let host84 = grammar(Family::Tcl, Release::TCL_8_4);
        let host85 = grammar(Family::Tcl, Release::TCL_8_5);

        for row in PARSER_PARITY_VECTORS {
            match row.axis {
                GrammarAxis::ImplicitWordBreak | GrammarAxis::WordBreakGating => {
                    for g in f5_grammars {
                        assert!(g.irules_brace_separator, "{}", row.case);
                    }
                    assert!(!host84.irules_brace_separator);
                    assert!(!host85.irules_brace_separator);
                }
                GrammarAxis::BraceLineContinuation | GrammarAxis::ContinuationBound => {
                    for g in f5_grammars {
                        assert!(g.brace_line_continuation.continues(), "{}", row.case);
                    }
                    assert!(!host84.brace_line_continuation.continues());
                    assert!(!host85.brace_line_continuation.continues());
                }
                GrammarAxis::InertExpansion => {
                    for g in f5_grammars {
                        assert!(!g.expand_syntax, "{}: the separator wins", row.case);
                    }
                    assert!(!host84.expand_syntax);
                    assert!(
                        host85.expand_syntax,
                        "the 8.5 control really does expand — which is why the \
                         F5 value is a false pass, not a feature"
                    );
                }
                GrammarAxis::NumeralGrammar => {
                    for g in f5_grammars {
                        assert_eq!(g.numbers, host84.numbers, "{}: 8.4 numerals", row.case);
                    }
                    assert_ne!(host85.numbers, host84.numbers);
                }
                GrammarAxis::ExprWordOperators => assert_expr_word_operator_row(row),
                GrammarAxis::Unmodified => {
                    assert_eq!(row.f5, row.host84, "{}: control row", row.case);
                }
            }
            match row.axis {
                // A fork axis exists because the F5 answer differs from
                // the stock answer; a row that does not differ has no
                // business citing one.
                GrammarAxis::ImplicitWordBreak
                | GrammarAxis::BraceLineContinuation
                | GrammarAxis::InertExpansion
                | GrammarAxis::ExprWordOperators => assert!(
                    row.f5 != row.host84 || row.f5 != row.host85,
                    "{}: no divergence to record",
                    row.case
                ),
                // The numeral rows are the opposite claim: the F5 tree
                // behaves as its 8.4 fork point throughout.
                GrammarAxis::NumeralGrammar => assert_eq!(
                    row.f5, row.host84,
                    "{}: 8.4 numerals throughout (§4a)",
                    row.case
                ),
                // The gating rows say the opposite of a fork row: every
                // context agrees, which is what keeps R5 from
                // overclaiming.
                GrammarAxis::WordBreakGating | GrammarAxis::ContinuationBound => {
                    assert_eq!(row.f5, row.host84, "{}: boundary row", row.case);
                    assert_eq!(row.f5, row.host85, "{}: boundary row", row.case);
                }
                GrammarAxis::Unmodified => {}
            }
        }
    }

    /// None of the sixteen discriminators may be offered under any F5
    /// environment, and the ones that are commands are checked through the
    /// registry rather than asserted in prose.
    #[test]
    fn release_discriminators_are_absent_from_every_f5_environment() {
        let control = static_context_for("tcl8.5").commands();
        let control_mask = resolve_environment("tcl8.5")
            .analyser_profile()
            .surface_query();
        for row in RELEASE_DISCRIMINATOR_VECTORS {
            assert!(
                !row.tcl84 && row.tcl85,
                "{}: not a discriminator",
                row.feature
            );
            for behaviour in [row.tmsh, row.iapp] {
                assert_ne!(
                    behaviour,
                    DiscriminatorBehaviour::BehavesAs85,
                    "{}: measured as 8.4 in both contexts",
                    row.feature
                );
            }
            let ModelProbe::Command(command) = row.probe else {
                continue;
            };
            assert!(
                control.get_for_surface(command, control_mask).is_some(),
                "{command}: the 8.5 control must have it"
            );
            for environment in F5_ENVIRONMENTS {
                let registry = static_context_for(environment).commands();
                let mask = resolve_environment(environment)
                    .analyser_profile()
                    .surface_query();
                assert!(
                    registry.get_for_surface(command, mask).is_none(),
                    "{environment}: {command} is measured absent (§4)"
                );
            }
        }
    }

    /// The §4b split, from the transcript side: the corpus, the policy
    /// module, and the evidence table must all say the same thing.
    #[test]
    fn command_classes_match_the_measured_split() {
        let build = BigIpBuild::MEASURED_21_1_0_1;
        let mut absent = 0;
        let mut refused = 0;
        for row in COMMAND_CLASS_VECTORS {
            assert_eq!(
                crate::irules_policy::irules_disabled_class(row.command),
                Some(row.class),
                "{}",
                row.command
            );
            assert_eq!(
                row.runtime_reachable,
                row.class == IrulesDisabledClass::CompilerRefused,
                "{}: reachability follows the mechanism",
                row.command
            );
            assert_eq!(
                row.class.is_language_fact(),
                !row.runtime_reachable,
                "{}: severity follows the mechanism",
                row.command
            );
            let evidence = measured_fact(
                BigIpExecutionContext::TmmIRule,
                build,
                RuntimeFactKind::CommandSurface(row.command),
            )
            .unwrap_or_else(|| panic!("{}: no evidence row", row.command));
            assert_eq!(
                evidence.provenance.probe_set,
                ProbeSetId::PROC_SEMANTICS,
                "{}",
                row.command
            );
            match row.class {
                IrulesDisabledClass::InterpreterAbsent => absent += 1,
                IrulesDisabledClass::CompilerRefused => refused += 1,
            }
        }
        assert_eq!((absent, refused), (16, 15));
    }

    /// The 120-cell matrix, asserted against the model's own
    /// event-validity oracle.
    ///
    /// Rows marked [`ModelExpectation::Agrees`] must keep agreeing; rows
    /// marked [`ModelExpectation::Diverges`] must keep diverging *exactly
    /// where they are recorded*, so that closing a gap is a deliberate
    /// edit of this table and never a silent change of behaviour.
    #[test]
    fn event_context_vectors_pin_the_model() {
        let registry = static_context_for("f5-irules").commands();
        let events = EventRegistry::build();
        let profiles = ProfileRegistry::build();
        let mut agreeing = 0;
        let mut diverging = 0;
        for row in EVENT_CONTEXT_VECTORS {
            if let ModelExpectation::NotComparable(reason) = row.expectation {
                assert_eq!(
                    row.command, "static::",
                    "only the variable-namespace row is incomparable"
                );
                assert!(!reason.is_empty(), "an incomparable row must say why");
                continue;
            }
            let model = registry.is_irules_call_legal_in_event(
                row.command,
                &[],
                row.event,
                &events,
                &profiles,
            );
            match row.expectation {
                ModelExpectation::Agrees => {
                    assert_eq!(
                        model,
                        row.measured.accepted(),
                        "{} in {}: the model no longer matches the appliance (§8)",
                        row.command,
                        row.event
                    );
                    agreeing += 1;
                }
                ModelExpectation::Diverges(_) => {
                    assert_ne!(
                        model,
                        row.measured.accepted(),
                        "{} in {}: the recorded divergence is closed — \
                         reclassify the row as Agrees",
                        row.command,
                        row.event
                    );
                    diverging += 1;
                }
                ModelExpectation::NotComparable(_) => unreachable!("handled above"),
            }
        }
        assert_eq!(
            (agreeing, diverging),
            (106, 6),
            "the divergence budget is fixed: change it deliberately"
        );

        // Every remaining divergence is the deliberate `RULE_INIT`
        // compile-acceptance one.
        let deliberate = EVENT_CONTEXT_VECTORS
            .iter()
            .filter(|row| {
                matches!(
                    row.expectation,
                    ModelExpectation::Diverges(DivergenceReason::RuleInitCompileAcceptance)
                )
            })
            .count();
        assert_eq!(deliberate, 6, "§8's compile-acceptance caveat");
        assert_eq!(deliberate, diverging, "no open event-context gap is left");

        // …and the two open-gap classes are empty. They are kept as
        // vocabulary rather than deleted: the corpus has to be able to
        // say *which way* a future regression hurts — an over-permissive
        // cell costs the user a missed load-time error, an over-strict
        // one a false positive.
        let count = |reason: DivergenceReason| {
            EVENT_CONTEXT_VECTORS
                .iter()
                .filter(|row| row.expectation == ModelExpectation::Diverges(reason))
                .count()
        };
        assert_eq!(
            (
                count(DivergenceReason::ModelBroaderThanMeasured),
                count(DivergenceReason::ModelNarrowerThanMeasured),
            ),
            (0, 0),
            "open event-context gaps: over-permissive vs over-strict"
        );
    }

    /// The priority policy the traffic lab measured is the policy the
    /// registry ships.
    #[test]
    fn priority_vectors_match_the_shipping_policy() {
        let policy = crate::events::BIGIP_EVENT_HANDLER_PRIORITY;
        let mut ordering: Vec<(u8, u16)> = Vec::new();
        for row in PRIORITY_VECTORS {
            match row.case {
                PriorityCase::Bound { value, accepted } => {
                    let modelled = u16::try_from(value).is_ok_and(|v| policy.accepts(v));
                    assert_eq!(modelled, accepted, "priority {value}");
                }
                PriorityCase::Ordering {
                    declared,
                    effective,
                    rank,
                    ..
                } => {
                    match declared {
                        Some(value) => assert_eq!(value, effective),
                        None => assert_eq!(
                            effective, policy.default_priority,
                            "an omitted clause takes the default"
                        ),
                    }
                    ordering.push((rank, effective));
                }
            }
        }
        ordering.sort_unstable();
        assert!(
            ordering.windows(2).all(|pair| pair[0].1 < pair[1].1),
            "lower numbers ran first: {ordering:?}"
        );
        assert!(policy.lower_runs_first);
    }

    /// The §4a environment table and the evidence records are two views of
    /// one transcript; they cannot be allowed to drift apart.
    #[test]
    fn context_environment_vectors_agree_with_the_evidence_table() {
        let build = BigIpBuild::MEASURED_21_1_0_1;
        let value_of =
            |row: &ContextEnvironmentVector, context: BigIpExecutionContext| match context {
                BigIpExecutionContext::TmmIRule => row.tmm,
                BigIpExecutionContext::TmshCliScript => row.tmsh,
                BigIpExecutionContext::IAppImplementation => row.iapp,
                BigIpExecutionContext::HostShellTcl => row.host84,
                other => panic!("{other} has no column"),
            };
        let contexts = [
            BigIpExecutionContext::TmmIRule,
            BigIpExecutionContext::TmshCliScript,
            BigIpExecutionContext::IAppImplementation,
            BigIpExecutionContext::HostShellTcl,
        ];

        for row in CONTEXT_ENVIRONMENT_VECTORS {
            for context in contexts {
                let cell = value_of(row, context);
                match row.property {
                    "info patchlevel" => {
                        let Some(RuntimeFact::ReportedPatchlevel {
                            info_patchlevel, ..
                        }) = measured_fact(context, build, RuntimeFactKind::ReportedPatchlevel)
                            .map(|e| e.fact)
                        else {
                            panic!("{context}: no patchlevel row");
                        };
                        assert_eq!(info_patchlevel, cell, "{context}");
                    }
                    "llength [info commands]" => {
                        let Some(RuntimeFact::CommandCount(count)) =
                            measured_fact(context, build, RuntimeFactKind::CommandCount)
                                .map(|e| e.fact)
                        else {
                            panic!("{context}: no command-count row");
                        };
                        assert_eq!(count.to_string(), cell, "{context}");
                    }
                    "tcl_platform keys" => {
                        let Some(RuntimeFact::TclPlatform { keys, .. }) =
                            measured_fact(context, build, RuntimeFactKind::TclPlatform)
                                .map(|e| e.fact)
                        else {
                            panic!("{context}: no platform row");
                        };
                        assert_eq!(keys.to_string(), cell, "{context}");
                    }
                    "tcl_platform(wordSize)" => {
                        let Some(RuntimeFact::TclPlatform { word_size, .. }) =
                            measured_fact(context, build, RuntimeFactKind::TclPlatform)
                                .map(|e| e.fact)
                        else {
                            panic!("{context}: no platform row");
                        };
                        assert_eq!(
                            word_size.map(|w| w.to_string()).unwrap_or_default(),
                            cell,
                            "{context}"
                        );
                    }
                    _ => {}
                }
            }
        }

        // TMM's seven fabricated `tcl_platform` keys are exactly the seven
        // the shipping variable model claims for iRules.
        let irules_keys = crate::special_vars::special_var("tcl_platform")
            .expect("tcl_platform is modelled")
            .keys
            .iter()
            .filter(|key| surface_admits(SpecSurface::IRULES, Some(&key.surface)))
            .count();
        assert_eq!(irules_keys, 7, "§4: TMM reports 7 fabricated keys");
    }

    /// The two `Unknown` contexts have no corpus coverage at all, and the
    /// acceptance matrix's remaining columns are still open — the corpus
    /// says so rather than implying completeness.
    #[test]
    fn the_corpus_covers_one_build_and_four_contexts() {
        assert_eq!(crate::f5::evidence::MEASURED_BUILDS.len(), 1);
        for context in [
            BigIpExecutionContext::IAppPresentationApl,
            BigIpExecutionContext::IAppPresentationTclCallback,
        ] {
            assert!(!context.measurement().is_measured(), "{context}");
        }
        // The BIG-IP axis the acceptance matrix's other two columns will
        // land on is typed and ready; nothing on it is claimed yet.
        let measured =
            VersionSet::from_requirements(VersionAxisId::big_ip(), &["21.1.0.1-21.1.0.1"])
                .expect("measured build");
        let seventeen = VersionSet::from_requirements(VersionAxisId::big_ip(), &["17.1.0-18.0"])
            .expect("17.x column");
        assert!(
            measured
                .intersect(&seventeen)
                .expect("same axis")
                .is_empty(),
            "the 17.x column is not covered by this build"
        );
    }
}
