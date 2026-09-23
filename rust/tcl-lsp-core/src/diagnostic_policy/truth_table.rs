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

//! The diagnostic-policy truth table: one table from `(program, policy)` to
//! the report, beside [`super::apply`], that every adapter runs
//! (`docs/design/compiler/diagnostic-policy.md` § The truth table).
//!
//! Each [`Row`] is a small program, the configuration by slot, and what the
//! editor's report holds for the codes the row names — shown, shown at a
//! severity, suppressed for a reason, or absent with the reason the report
//! gives for the gap. The expectations are written once, for the editor
//! ([`Surface::Core`]); [`Row::expected`] derives every other surface's by
//! the rules the page states, so one set of hand-checked expectations gates
//! them all (`docs/design/lanes/diagnostic-policy.md` § Decisions taken,
//! D30). [`Row::runs_on`] says which surfaces can realise a row at all, and
//! [`check`] compares what a surface rendered with what it should have.
//!
//! A row's program is a means: when a producer stops emitting a subject code
//! where a row says, the program or the line changes, never the wanted
//! reason — a reason that does not hold is a defect in the policy step or in
//! an adapter. A row whose wanted outcome does not hold today says so with a
//! [`Defect`]: [`check`] holds the surfaces it names to what they render
//! today, and fails the day the wanted outcome holds, so the fix removes the
//! marker.
//!
//! Compiled for this crate's tests and, through the `truth-table` feature,
//! for the adapter crates' tests, which enable the feature from their
//! `[dev-dependencies]`; no shipped build carries it.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::{Map, Value};
use tcl_compiler::optimiser::optimise_with_dialect;
use tcl_compiler::optimiser::profiles::OptimisationProfile;
use tcl_core_types::{DiagCode, Severity};

use super::{
    Directives, Finding, OverlapOwner, PolicyBuilder, PolicyLayer, Producer, Reason, Report,
};
use crate::diagnostic_report::{
    DocumentSource, SourcePass, StandaloneDocument, document_report, standalone_findings,
};
use crate::source_decode::{DecodeReport, decode_source};
use crate::source_style::DEFAULT_LINE_LENGTH;

/// What a row expects of one code at one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    /// Shown, at whatever severity the producer chose.
    Shown,
    /// Shown at this severity.
    ShownAt(Severity),
    /// A finding, suppressed for this reason.
    Suppressed(Reason),
    /// No finding at all; the report explains the code with this reason.
    Gap(Reason),
    /// The finding's fix is offered as a code action, or is not. Derived by
    /// [`Row::expected`] for an action surface; a row never writes it.
    Offered(bool),
    /// The finding's rewrite is applied by a rewrite surface, or is not.
    /// Derived by [`Row::expected`]; a row never writes it.
    Applied(bool),
}

impl Want {
    /// Whether the finding shows.
    const fn is_shown(self) -> bool {
        matches!(self, Self::Shown | Self::ShownAt(_))
    }
}

/// One expectation. `line` is 1-based, as a reader counts; a `Gap` has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expect {
    /// The code.
    pub code: DiagCode,
    /// The 1-based line of the finding; `None` for a gap.
    pub line: Option<u32>,
    /// What the report holds for it.
    pub want: Want,
}

/// One `(program, policy)` row.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// A stable `snake_case` name, used in every failure message.
    pub name: &'static str,
    /// The document's dialect.
    pub dialect: &'static str,
    /// The document's text.
    pub program: &'static str,
    /// Bytes decoded in place of `program`, for an abstaining document.
    pub bytes: Option<&'static [u8]>,
    /// The user's global file, as the `tclLsp` content shape in JSON text.
    pub global: Option<&'static str>,
    /// The editor slot: the editor layer on the server, a surface's
    /// invocation layer elsewhere.
    pub slot: Option<&'static str>,
    /// The project file.
    pub project: Option<&'static str>,
    /// The shown set is exactly the `Shown` / `ShownAt` expectations.
    pub exhaustive: bool,
    /// What the editor's report holds for the codes the row names.
    pub expect: &'static [Expect],
    /// A defect the row records: where the wanted outcome does not hold
    /// today.
    pub defect: Option<Defect>,
}

/// A row whose wanted outcome does not hold on some surfaces today — a
/// defect the table records rather than hides. On those surfaces [`check`]
/// holds the rendering to `today`, and fails once the wanted outcome holds,
/// so the change that fixes the defect removes the marker.
#[derive(Debug, Clone, Copy)]
pub struct Defect {
    /// The surfaces the wanted outcome does not hold on.
    pub surfaces: &'static [Surface],
    /// What the editor's report holds today, written as a row's
    /// expectations are and rendered per surface the same way.
    pub today: &'static [Expect],
    /// What is wrong, for the reader and every failure message.
    pub note: &'static str,
}

/// Where a row is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// The report itself ([`core_report`]).
    Core,
    /// The language server's published diagnostics.
    Lsp,
    /// The language server's code actions.
    LspActions,
    /// `tcl-lsp.optimiseDocument`.
    LspRewrite,
    /// `tcl diag --show-suppressed`.
    Cli,
    /// `tcl opt`.
    CliRewrite,
    /// The MCP diagnostics tools' `diagnostics` and `suppressed` arrays.
    Mcp,
    /// The MCP `code_actions` tool.
    McpActions,
    /// The MCP `optimize` tool.
    McpRewrite,
}

/// The codes an action surface judges: the brace refactor (W100), the
/// shimmer `# noqa` action (S100) and the fold quick-fix (O101).
const ACTION_SUBJECTS: &[DiagCode] = &[DiagCode::W100, DiagCode::S100, DiagCode::O101];

/// The code a rewrite surface judges: the constant fold.
const REWRITE_SUBJECT: DiagCode = DiagCode::O101;

/// The title of the brace refactor [`offered`] looks for.
const BRACE_TITLE: &str = "Brace expr for safety and performance";

impl Row {
    /// Whether `surface` can realise this row's configuration and program.
    ///
    /// The core report and the published diagnostics run every row. An
    /// action surface needs a subject it can act on, a rewrite surface the
    /// fold. The batch verbs read no document gate (`features`,
    /// `diagnostics.exclude`), and their flags carry only per-code booleans
    /// — `diagnostics` ones for the diagnostics verbs, `optimiser` ones and a
    /// profile for `tcl opt`. An MCP call carries neither a project file
    /// (its `source` has no path) nor bytes (its tools take decoded text).
    #[must_use]
    pub fn runs_on(&self, surface: Surface) -> bool {
        match surface {
            Surface::Core | Surface::Lsp => true,
            Surface::LspActions => self.names_any(ACTION_SUBJECTS),
            Surface::LspRewrite => self.names_any(&[REWRITE_SUBJECT]),
            Surface::Cli => self.reads_no_document_gate() && self.slot_holds_only("diagnostics"),
            Surface::Mcp => self.runs_on(Surface::Cli) && self.fits_an_mcp_call(),
            Surface::McpActions => self.runs_on(Surface::Mcp) && self.names_any(ACTION_SUBJECTS),
            Surface::CliRewrite => {
                self.names_any(&[REWRITE_SUBJECT])
                    && self.reads_no_document_gate()
                    && self.slot_holds_only("optimiser")
            }
            Surface::McpRewrite => self.runs_on(Surface::CliRewrite) && self.fits_an_mcp_call(),
        }
    }

    /// The wanted expectations as `surface` renders them (a [`Defect`]'s
    /// `today` renders by the same rules).
    ///
    /// - `Lsp`: what shows stays; a suppressed finding or a gap is no
    ///   published diagnostic of the code — the adapter publishes only what
    ///   shows.
    /// - `Cli`, `Mcp`: `Disabled(Editor)` is `Disabled(Invocation)` — the
    ///   flags occupy the editor layer's slot; an O-code that would show, or
    ///   that the profile or an overlap hides, is `OptimiserOff` — the
    ///   diagnostics verbs and tools run with the optimiser off, the first
    ///   family gate, while steps 1 to 4 fire before it and keep their
    ///   reasons; a default-off gap is not rendered (D21). `Cli` alone: an
    ///   abstention suppression is absent, because the CLI does not analyse
    ///   an abstaining document — the integrity pass alone runs.
    /// - `LspActions`, `McpActions`: each actionable subject is
    ///   [`Want::Offered`], true exactly when it shows. Code actions run
    ///   with the optimiser on, so the `Mcp` rule's O-code gate does not
    ///   apply, and its layer rule changes no subject's visibility.
    /// - `LspRewrite`, `CliRewrite`, `McpRewrite`: each fold subject is
    ///   [`Want::Applied`], true exactly when it shows.
    #[must_use]
    pub fn expected(&self, surface: Surface) -> Vec<Expect> {
        self.expect
            .iter()
            .filter_map(|expect| rendered(*expect, surface))
            .collect()
    }

    /// A layer's JSON, parsed; an absent layer is an empty object.
    #[must_use]
    pub fn layer(json: Option<&str>) -> Value {
        json.map_or_else(
            || Value::Object(Map::new()),
            |text| {
                serde_json::from_str(text)
                    .unwrap_or_else(|err| panic!("a truth-table layer is JSON ({err}): {text}"))
            },
        )
    }

    /// A layer written as an INI file — `[diagnostics]` per-code keys and
    /// `exclude`, `[diagnosticSeverity]`, `[optimiser]`, `[shimmer]`,
    /// `[features]` — for the CLI's `config.ini` / `.tcl-lsp.ini`, which
    /// [`crate::config_ini::settings_from_ini`] reads back as the same layer.
    #[must_use]
    pub fn ini(json: Option<&str>) -> String {
        let layer = Self::layer(json);
        let mut out = String::new();
        let Some(sections) = layer.as_object() else {
            return out;
        };
        for (section, entries) in sections {
            let Some(entries) = entries.as_object() else {
                panic!("a truth-table layer's `{section}` is an object: {layer}");
            };
            let _ = writeln!(out, "[{section}]");
            for (key, value) in entries {
                match value {
                    Value::Array(items) => {
                        // One item per continuation line, as `exclude` reads
                        // them: a glob may itself hold a comma.
                        let _ = writeln!(out, "{key} =");
                        for item in items {
                            let _ = writeln!(out, "    {}", scalar(item));
                        }
                    }
                    scalar_value => {
                        let _ = writeln!(out, "{key} = {}", scalar(scalar_value));
                    }
                }
            }
            out.push('\n');
        }
        out
    }

    /// The slot as `--disable` / `--enable` / `--profile` arguments, and as
    /// MCP `disable` / `enable` / `profile` values.
    #[must_use]
    pub fn slot_flags(&self) -> SlotFlags {
        let slot = Self::layer(self.slot);
        let mut flags = SlotFlags::default();
        for section in ["diagnostics", "optimiser"] {
            let Some(entries) = slot.get(section).and_then(Value::as_object) else {
                continue;
            };
            for (key, value) in entries {
                if section == "optimiser" && key == "profile" {
                    flags.profile = value.as_str().map(str::to_owned);
                } else if key.parse::<DiagCode>().is_ok() {
                    match value.as_bool() {
                        Some(false) => flags.disable.push(key.clone()),
                        Some(true) => flags.enable.push(key.clone()),
                        None => {}
                    }
                }
            }
        }
        flags
    }

    /// The text a surface analyses: `program`, or `bytes` decoded by
    /// [`decode_source`], with the decode report.
    #[must_use]
    pub fn text(&self) -> (String, Option<DecodeReport>) {
        match self.bytes {
            Some(bytes) => {
                let (text, report) = decode_source(bytes);
                (text, Some(report))
            }
            None => (self.program.to_owned(), None),
        }
    }

    /// The row's three layers, parsed: global, slot, project.
    fn layers(&self) -> [Value; 3] {
        [
            Self::layer(self.global),
            Self::layer(self.slot),
            Self::layer(self.project),
        ]
    }

    /// Whether an expectation names one of `codes`.
    fn names_any(&self, codes: &[DiagCode]) -> bool {
        self.expect
            .iter()
            .any(|expect| codes.contains(&expect.code))
    }

    /// No layer carries a document gate: `features` or
    /// `diagnostics.exclude`.
    fn reads_no_document_gate(&self) -> bool {
        self.layers()
            .iter()
            .all(|layer| layer.get("features").is_none() && !carries_exclude(layer))
    }

    /// The slot holds only `section`'s keys, each a catalogued code with a
    /// boolean — or, for `optimiser`, the profile too.
    fn slot_holds_only(&self, section: &str) -> bool {
        let slot = Self::layer(self.slot);
        let Some(sections) = slot.as_object() else {
            return false;
        };
        sections.iter().all(|(name, entries)| {
            name == section
                && entries.as_object().is_some_and(|entries| {
                    entries.iter().all(|(key, value)| {
                        (section == "optimiser" && key == "profile" && value.is_string())
                            || (key.parse::<DiagCode>().is_ok() && value.is_boolean())
                    })
                })
        })
    }

    /// No project layer and no bytes: what an MCP call can carry.
    const fn fits_an_mcp_call(&self) -> bool {
        self.project.is_none() && self.bytes.is_none()
    }
}

/// `layer` names files that produce no diagnostics at all.
fn carries_exclude(layer: &Value) -> bool {
    layer
        .get("diagnostics")
        .and_then(|diagnostics| diagnostics.get("exclude"))
        .is_some()
}

/// A JSON scalar as an INI value.
fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// One expectation as `surface` renders it; `None` when the surface renders
/// nothing for it ([`Row::expected`] states the rules).
fn rendered(expect: Expect, surface: Surface) -> Option<Expect> {
    let want = match surface {
        Surface::Core => expect.want,
        Surface::Lsp => {
            if !expect.want.is_shown() {
                return None;
            }
            expect.want
        }
        Surface::Cli | Surface::Mcp => batch_want(expect, surface == Surface::Cli)?,
        Surface::LspActions | Surface::McpActions => {
            if !ACTION_SUBJECTS.contains(&expect.code) {
                return None;
            }
            Want::Offered(expect.want.is_shown())
        }
        Surface::LspRewrite | Surface::CliRewrite | Surface::McpRewrite => {
            if expect.code != REWRITE_SUBJECT {
                return None;
            }
            Want::Applied(expect.want.is_shown())
        }
    };
    Some(Expect { want, ..expect })
}

/// What a diagnostics verb or tool renders for `expect`.
fn batch_want(expect: Expect, cli: bool) -> Option<Want> {
    let invocation = |reason: Reason| match reason {
        Reason::Disabled(PolicyLayer::Editor) => Reason::Disabled(PolicyLayer::Invocation),
        other => other,
    };
    Some(match expect.want {
        Want::Gap(Reason::DefaultOff) => return None,
        Want::Suppressed(Reason::EncodingAbstention) if cli => return None,
        Want::Shown
        | Want::ShownAt(_)
        | Want::Suppressed(Reason::OptimiserProfile { .. } | Reason::Overlap { .. })
            if expect.code.is_optimisation() =>
        {
            Want::Suppressed(Reason::OptimiserOff)
        }
        Want::Suppressed(reason) => Want::Suppressed(invocation(reason)),
        Want::Gap(reason) => Want::Gap(invocation(reason)),
        want => want,
    })
}

/// A row's slot as a surface's own arguments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlotFlags {
    /// `diagnostics.<CODE>: false` and `optimiser.<CODE>: false`.
    pub disable: Vec<String>,
    /// `diagnostics.<CODE>: true` and `optimiser.<CODE>: true`.
    pub enable: Vec<String>,
    /// `optimiser.profile`.
    pub profile: Option<String>,
}

/// One observed rendering of a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observed {
    /// The code.
    pub code: DiagCode,
    /// The 1-based line; `None` for a gap.
    pub line: Option<u32>,
    /// What the surface rendered.
    pub state: ObservedState,
}

/// What a surface rendered for one finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservedState {
    /// Shown, at this severity when the surface renders one.
    Shown(Option<Severity>),
    /// Hidden, with the reason's spelling (`Reason`'s `Display`).
    Suppressed(String),
    /// Whether the finding's fix was offered as an action.
    Offered(bool),
    /// Whether the finding's rewrite was applied.
    Applied(bool),
}

/// One offered code action, in a shape every surface can build.
#[derive(Debug, Clone)]
pub struct ActionView<'a> {
    /// The action's title.
    pub title: &'a str,
    /// The action's LSP kind (`quickfix`, `refactor.rewrite`, …).
    pub kind: &'a str,
    /// Each edit's 1-based start line and new text.
    pub edits: Vec<(u32, &'a str)>,
}

/// Whether `actions` offer the fix of the `code` finding at `line`: for
/// W100 the brace refactor with an edit on the line, for S100 the `# noqa:
/// S100` comment inserted above the line, for O101 a quick-fix whose edit
/// writes the folded command.
#[must_use]
pub fn offered(code: DiagCode, line: u32, actions: &[ActionView<'_>]) -> bool {
    actions.iter().any(|action| match code {
        DiagCode::W100 => {
            action.title == BRACE_TITLE && action.edits.iter().any(|(at, _)| *at == line)
        }
        DiagCode::S100 => action
            .edits
            .iter()
            .any(|(at, text)| *at == line && text.trim() == "# noqa: S100"),
        DiagCode::O101 => {
            action.kind == "quickfix" && action.edits.iter().any(|(_, text)| *text == "set x 3")
        }
        _ => false,
    })
}

/// Compare `observed` with `row.expected(surface)`: for every code the
/// expectations name, the observed entries of that code equal the expected
/// ones as a multiset of `(line, state)` — [`Want::Shown`] matching any
/// severity; codes the row does not name are ignored unless `exhaustive`,
/// which forbids any other shown code. Every failure names the row.
///
/// On a surface a row's [`Defect`] names, the rendering is held to the
/// defect's `today` instead, and a rendering that meets the wanted outcome
/// fails: the defect is fixed, and the marker goes with it.
pub fn check(row: &Row, surface: Surface, observed: &[Observed]) -> Result<(), String> {
    let Some(defect) = row
        .defect
        .filter(|defect| defect.surfaces.contains(&surface))
    else {
        return compare(row, row.expect, surface, observed);
    };
    if compare(row, row.expect, surface, observed).is_ok() {
        return Err(format!(
            "row `{}` on {surface:?} holds as wanted, so the defect it records is fixed — \
             remove the marker: {}",
            row.name, defect.note
        ));
    }
    compare(row, defect.today, surface, observed)
        .map_err(|failure| format!("{failure} (the row records a defect: {})", defect.note))
}

/// [`check`]'s comparison of `observed` with `expect` as `surface` renders
/// it.
fn compare(
    row: &Row,
    expect: &[Expect],
    surface: Surface,
    observed: &[Observed],
) -> Result<(), String> {
    let expected: Vec<Expect> = expect
        .iter()
        .filter_map(|expect| rendered(*expect, surface))
        .collect();
    let named: BTreeSet<DiagCode> = expect.iter().map(|expect| expect.code).collect();
    let mut problems: Vec<String> = Vec::new();
    for code in &named {
        // The specific expectations claim their observations first, so a
        // `Shown` wildcard cannot take the one a `ShownAt` needs.
        let mut wanted: Vec<Expect> = expected
            .iter()
            .filter(|expect| expect.code == *code)
            .copied()
            .collect();
        wanted.sort_by_key(|expect| expect.want == Want::Shown);
        let mut seen: Vec<&Observed> = observed.iter().filter(|o| o.code == *code).collect();
        let mut missing: Vec<Expect> = Vec::new();
        for expect in wanted {
            match seen.iter().position(|o| satisfies(expect, o)) {
                Some(found) => {
                    seen.remove(found);
                }
                None => missing.push(expect),
            }
        }
        if !missing.is_empty() || !seen.is_empty() {
            problems.push(format!(
                "{code}: expected {missing:?} not observed; observed {seen:?} not expected"
            ));
        }
    }
    if row.exhaustive {
        let extra: Vec<&Observed> = observed
            .iter()
            .filter(|o| !named.contains(&o.code) && matches!(o.state, ObservedState::Shown(_)))
            .collect();
        if !extra.is_empty() {
            problems.push(format!("the row is exhaustive, but {extra:?} also shows"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "row `{}` on {surface:?}: {}",
            row.name,
            problems.join("; ")
        ))
    }
}

/// Whether `observed` is what `expect` wants.
fn satisfies(expect: Expect, observed: &Observed) -> bool {
    if expect.line != observed.line {
        return false;
    }
    match (expect.want, &observed.state) {
        (Want::Shown, ObservedState::Shown(_)) => true,
        (Want::ShownAt(want), ObservedState::Shown(Some(got))) => want == *got,
        (Want::Suppressed(reason) | Want::Gap(reason), ObservedState::Suppressed(got)) => {
            reason.to_string() == *got
        }
        (Want::Offered(want), ObservedState::Offered(got))
        | (Want::Applied(want), ObservedState::Applied(got)) => want == *got,
        _ => false,
    }
}

/// The report the editor's policy makes of `row`, and the text it is over.
///
/// The server's producer set, run the way a surface without the database
/// runs it: the analyser under the policy's production skip, the O111 hints
/// and the compiler checks ([`standalone_findings`]), then the optimiser's
/// rewrites; [`document_report`] adds the style pass, the integrity pass and
/// the `SslicTcl` loader, under the three layers (the slot as the editor
/// layer), the decode report, the dialect's overlaps and the analysis's
/// directives; the analyser's skip is declared. `diagnostics.exclude` in any
/// layer marks the file excluded — matching the glob is the server's
/// business, not the table's.
#[must_use]
pub fn core_report(row: &Row) -> (String, Report) {
    let (text, decode) = row.text();
    let analysis_text = tcl_lexer::normalise_lone_cr(&text).into_owned();
    let dialect = crate::profile_for_dialect(row.dialect);
    let registry = tcl_registry::model::ingress::static_context_for(row.dialect).commands();
    let [global, slot, project] = row.layers();
    let builder = PolicyBuilder::new()
        .layer(PolicyLayer::Global, &global)
        .layer(PolicyLayer::Editor, &slot)
        .layer(PolicyLayer::Project, &project)
        .decode(decode.as_ref())
        .dialect(dialect)
        .excluded([&global, &slot, &project].into_iter().any(carries_exclude));
    let skip = builder.clone().build().production_skip();
    let standalone = standalone_findings(
        &StandaloneDocument {
            source: &analysis_text,
            file_path: None,
            dialect,
            registry,
            pack_overlay: 0,
            external_call_sites: None,
        },
        &skip,
    );
    let mut produced = standalone.produced;
    produced.extend(
        optimise_with_dialect(&analysis_text, registry, Some(dialect))
            .into_iter()
            .map(Finding::from),
    );
    let policy = builder
        .directives(Directives::from_analysis(
            &standalone.analysis,
            &analysis_text,
        ))
        .build();
    let doc = DocumentSource {
        text: &text,
        analysis_text: &analysis_text,
        decode: decode.as_ref(),
        dialect,
        pass: SourcePass::Tcl {
            line_length: DEFAULT_LINE_LENGTH,
        },
    };
    let mut report = document_report(&doc, produced, &policy);
    report.declare_analyser_skip(&policy);
    (text, report)
}

/// `set x 1   ` then `puts $y`: trailing whitespace on line 1 (W112) and an
/// unset variable on line 2 (W210).
const TRAILING: &str = "set x 1   \nputs $y\n";
/// An unset variable on line 1 (W210).
const UNSET: &str = "puts $y\n";
/// A loop whose termination cannot be proven (W242, default-off).
const LOOP: &str = "while {$x < 10} {puts hi}\n";
/// A string incremented as an integer (S100 on line 2).
const SHIMMER: &str = "set x hello\nincr x\n";
/// A constant expression the optimiser folds (O101).
const FOLD: &str = "set x [expr {1 + 2}]\n";
/// An unbraced expression on line 2 (W100, and O111 over it).
const UNBRACED: &str = "set a 1\nset b [expr $a + 1]\n";
/// A string compared with `==` on line 2 (W110, and O120 at its span).
const STREQ: &str = "proc p {x} {\n    if {$x == \"foo\"} { return 1 }\n    return 0\n}\n";
/// A variable set and never read on line 2 (W211, tagged unnecessary).
const UNUSED: &str = "proc p {} {\n    set x 1\n    return 0\n}\n";
/// Request data written into a response body on line 2 (IRULE3001).
const TAINT: &str = "set u [HTTP::uri]\nHTTP::respond 200 content $u\n";
/// A declaration the `SslicTcl` loader keeps as an extension on line 2
/// (SSLIC1101), which the analyser reads as an unknown command (W123).
const SSLIC: &str = "sslictcl 1\nunknown-declaration {a b}\n";
/// A UTF-16 byte-order mark ahead of [`TRAILING`]: the document abstains.
const BOM: &[u8] = b"\xFF\xFEset x 1   \nputs $y\n";
/// A UTF-16 byte-order mark, then a `# noqa` on a line of its own over
/// trailing whitespace on line 3.
const BOM_NOQA: &[u8] = b"\xFF\xFE\n# noqa\nset x 1   \n";

/// A `tcl9.0` row with no configuration.
const fn row(name: &'static str, program: &'static str, expect: &'static [Expect]) -> Row {
    Row {
        name,
        dialect: "tcl9.0",
        program,
        bytes: None,
        global: None,
        slot: None,
        project: None,
        exhaustive: false,
        expect,
        defect: None,
    }
}

/// `code` at 1-based `line`.
const fn at(code: DiagCode, line: u32, want: Want) -> Expect {
    Expect {
        code,
        line: Some(line),
        want,
    }
}

/// No finding of `code`, for `reason`.
const fn gap(code: DiagCode, reason: Reason) -> Expect {
    Expect {
        code,
        line: None,
        want: Want::Gap(reason),
    }
}

/// Suppressed by an inline `# noqa`, keyed by the finding's 0-based line.
const fn inline(line: i32) -> Want {
    Want::Suppressed(Reason::InlineDirective { line })
}

/// Suppressed for `reason`.
const fn hidden(reason: Reason) -> Want {
    Want::Suppressed(reason)
}

const DISABLED_EDITOR: Reason = Reason::Disabled(PolicyLayer::Editor);

/// The rows, numbered as `docs/design/lanes/diagnostic-policy.md` § DP9.4
/// numbers them.
pub const ROWS: &[Row] = &[
    // 1–2: the document-wide gates.
    Row {
        global: Some(r#"{"features": {"diagnostics": false}}"#),
        ..row(
            "reporting_off",
            TRAILING,
            &[
                at(DiagCode::W112, 1, hidden(Reason::ReportingOff)),
                at(DiagCode::W210, 2, hidden(Reason::ReportingOff)),
            ],
        )
    },
    Row {
        global: Some(r#"{"diagnostics": {"exclude": ["*.tcl"]}}"#),
        ..row(
            "excluded",
            TRAILING,
            &[
                at(DiagCode::W112, 1, hidden(Reason::Excluded)),
                at(DiagCode::W210, 2, hidden(Reason::Excluded)),
            ],
        )
    },
    // 3–4: an abstaining document.
    Row {
        bytes: Some(BOM),
        exhaustive: true,
        ..row(
            "encoding_abstention",
            TRAILING,
            &[
                at(DiagCode::W109, 1, Want::Shown),
                at(DiagCode::W112, 1, hidden(Reason::EncodingAbstention)),
                at(DiagCode::W210, 2, hidden(Reason::EncodingAbstention)),
            ],
        )
    },
    Row {
        bytes: Some(BOM_NOQA),
        ..row(
            "abstention_beats_a_directive",
            "\n# noqa\nset x 1   \n",
            &[
                at(DiagCode::W109, 1, Want::Shown),
                at(DiagCode::W112, 3, hidden(Reason::EncodingAbstention)),
            ],
        )
    },
    // 5–14: the directives.
    row(
        "inline_noqa_named",
        "# noqa: W210\nputs $y\n",
        &[at(DiagCode::W210, 2, inline(1))],
    ),
    row(
        "inline_noqa_bare",
        "# noqa\nset x 1   \nputs $y\n",
        &[
            at(DiagCode::W112, 2, inline(1)),
            at(DiagCode::W210, 3, Want::Shown),
        ],
    ),
    row(
        "inline_noqa_star",
        "# noqa: *\nputs $y\n",
        &[at(DiagCode::W210, 2, inline(1))],
    ),
    row(
        "file_directive_named",
        "# tcl-lsp: disable=W112\nset x 1   \nputs $y\n",
        &[
            at(DiagCode::W112, 2, hidden(Reason::FileDirective)),
            at(DiagCode::W210, 3, Want::Shown),
        ],
    ),
    row(
        "file_directive_star",
        "# tcl-lsp: disable=*\nset x 1   \nputs $y\n",
        &[
            at(DiagCode::W112, 2, hidden(Reason::FileDirective)),
            at(DiagCode::W210, 3, hidden(Reason::FileDirective)),
        ],
    ),
    row(
        "file_directive_on_an_analyser_code",
        "# tcl-lsp: disable=W210\nputs $y\n",
        &[gap(DiagCode::W210, Reason::FileDirective)],
    ),
    row(
        "inline_noqa_on_a_check",
        "set x hello\n# noqa: S100\nincr x\n",
        &[at(DiagCode::S100, 3, inline(2))],
    ),
    row(
        "an_unrelated_noqa_leaves_a_check",
        "set x hello\n# noqa: W999\nincr x\n",
        &[at(DiagCode::S100, 3, Want::Shown)],
    ),
    row(
        "a_whole_file_code_ignores_an_inline_noqa",
        "# noqa\r\nset x 1\r\n",
        &[at(DiagCode::W118, 1, Want::Shown)],
    ),
    row(
        "a_whole_file_code_honours_the_file_directive",
        "# tcl-lsp: disable=W118\r\nset x 1\r\n",
        &[at(DiagCode::W118, 1, hidden(Reason::FileDirective))],
    ),
    // 15–24: the per-code decision, layer by layer, and the precedence
    // pairs.
    Row {
        global: Some(r#"{"diagnostics": {"W112": false}}"#),
        ..row(
            "disabled_at_global",
            TRAILING,
            &[
                at(
                    DiagCode::W112,
                    1,
                    hidden(Reason::Disabled(PolicyLayer::Global)),
                ),
                at(DiagCode::W210, 2, Want::Shown),
            ],
        )
    },
    Row {
        slot: Some(r#"{"diagnostics": {"W112": false}}"#),
        ..row(
            "disabled_in_the_slot",
            TRAILING,
            &[at(DiagCode::W112, 1, hidden(DISABLED_EDITOR))],
        )
    },
    Row {
        project: Some(r#"{"diagnostics": {"W112": false}}"#),
        ..row(
            "disabled_at_project",
            TRAILING,
            &[at(
                DiagCode::W112,
                1,
                hidden(Reason::Disabled(PolicyLayer::Project)),
            )],
        )
    },
    Row {
        slot: Some(r#"{"diagnostics": {"W210": false}}"#),
        ..row(
            "a_disabled_analyser_code_is_a_gap",
            UNSET,
            &[gap(DiagCode::W210, DISABLED_EDITOR)],
        )
    },
    row(
        "default_off",
        LOOP,
        &[gap(DiagCode::W242, Reason::DefaultOff)],
    ),
    Row {
        global: Some(r#"{"diagnostics": {"W242": true}}"#),
        ..row(
            "default_off_turned_on_at_global",
            LOOP,
            &[at(DiagCode::W242, 1, Want::Shown)],
        )
    },
    Row {
        slot: Some(r#"{"diagnostics": {"W242": true}}"#),
        ..row(
            "default_off_turned_on_in_the_slot",
            LOOP,
            &[at(DiagCode::W242, 1, Want::Shown)],
        )
    },
    Row {
        project: Some(r#"{"diagnostics": {"W242": true}}"#),
        ..row(
            "default_off_turned_on_at_project",
            LOOP,
            &[at(DiagCode::W242, 1, Want::Shown)],
        )
    },
    Row {
        global: Some(r#"{"diagnostics": {"W112": false}}"#),
        project: Some(r#"{"diagnostics": {"W112": true}}"#),
        ..row(
            "a_project_enable_over_a_global_disable",
            TRAILING,
            &[at(DiagCode::W112, 1, Want::Shown)],
        )
    },
    Row {
        project: Some(r#"{"diagnostics": {"W112": true}}"#),
        ..row(
            "an_inline_directive_over_a_project_enable",
            "# noqa: W112\nset x 1   \n",
            &[at(DiagCode::W112, 2, inline(1))],
        )
    },
    // 25–28: a compiler check, severity overrides and tags.
    Row {
        dialect: "f5-irules",
        ..row(
            "irules_taint_flow",
            TAINT,
            &[at(DiagCode::Irule3001, 2, Want::Shown)],
        )
    },
    Row {
        dialect: "f5-irules",
        slot: Some(r#"{"diagnostics": {"IRULE3001": false}}"#),
        ..row(
            "a_check_disabled_in_the_slot",
            TAINT,
            &[at(DiagCode::Irule3001, 2, hidden(DISABLED_EDITOR))],
        )
    },
    Row {
        global: Some(r#"{"diagnosticSeverity": {"W112": "error"}}"#),
        ..row(
            "a_severity_override_relabels_only_its_code",
            TRAILING,
            &[
                at(DiagCode::W112, 1, Want::ShownAt(Severity::Error)),
                at(DiagCode::W210, 2, Want::ShownAt(Severity::Warning)),
            ],
        )
    },
    row(
        "a_tagged_code_carries_its_tag",
        UNUSED,
        &[at(DiagCode::W211, 2, Want::Shown)],
    ),
    // 29–35: the family gates.
    Row {
        global: Some(r#"{"shimmer": {"enabled": false}}"#),
        ..row(
            "shimmer_off",
            SHIMMER,
            &[at(DiagCode::S100, 2, hidden(Reason::ShimmerOff))],
        )
    },
    Row {
        slot: Some(r#"{"optimiser": {"profile": "full"}}"#),
        ..row(
            "a_rewrite_shows_under_its_profile",
            FOLD,
            &[at(DiagCode::O101, 1, Want::Shown)],
        )
    },
    Row {
        slot: Some(r#"{"optimiser": {"profile": "readability"}}"#),
        ..row(
            "a_rewrite_outside_the_profile",
            FOLD,
            &[at(
                DiagCode::O101,
                1,
                hidden(Reason::OptimiserProfile {
                    profile: OptimisationProfile::Readability,
                }),
            )],
        )
    },
    Row {
        global: Some(r#"{"optimiser": {"enabled": false}}"#),
        slot: Some(r#"{"optimiser": {"profile": "full"}}"#),
        ..row(
            "the_optimiser_switch_reaches_a_rewrite",
            FOLD,
            &[at(DiagCode::O101, 1, hidden(Reason::OptimiserOff))],
        )
    },
    Row {
        slot: Some(r#"{"optimiser": {"profile": "full", "O101": false}}"#),
        ..row(
            "a_per_code_toggle_reaches_a_rewrite",
            FOLD,
            &[at(
                DiagCode::O101,
                1,
                hidden(Reason::OptimiserProfile {
                    profile: OptimisationProfile::Full,
                }),
            )],
        )
    },
    Row {
        slot: Some(r#"{"optimiser": {"profile": "readability"}}"#),
        ..row(
            "a_directive_over_the_profile",
            "# noqa: O101\nset x [expr {1 + 2}]\n",
            &[at(DiagCode::O101, 2, inline(1))],
        )
    },
    Row {
        slot: Some(r#"{"optimiser": {"profile": "full"}}"#),
        ..row(
            "a_file_directive_reaches_a_rewrite",
            "# tcl-lsp: disable=*\nset x [expr {1 + 2}]\n",
            &[at(DiagCode::O101, 2, hidden(Reason::FileDirective))],
        )
    },
    // 36–38: the overlap table.
    Row {
        defect: Some(Defect {
            surfaces: &[Surface::Core, Surface::Lsp],
            today: &[
                at(DiagCode::W110, 2, Want::Shown),
                at(DiagCode::O120, 2, Want::Shown),
            ],
            note: "W110 never owns O120: the analyser anchors W110 on the `==` operator \
                   and the optimiser spans O120 over the whole condition, so the \
                   same-span overlap never fires and both show",
        }),
        ..row(
            "a_same_span_overlap",
            STREQ,
            &[
                at(DiagCode::W110, 2, Want::Shown),
                at(
                    DiagCode::O120,
                    2,
                    hidden(Reason::Overlap {
                        owner: OverlapOwner::Code(DiagCode::W110),
                    }),
                ),
            ],
        )
    },
    Row {
        slot: Some(r#"{"diagnostics": {"W110": false}}"#),
        ..row(
            "an_overlap_needs_a_standing_owner",
            STREQ,
            &[
                gap(DiagCode::W110, DISABLED_EDITOR),
                at(DiagCode::O120, 2, Want::Shown),
            ],
        )
    },
    Row {
        dialect: "sslictcl",
        ..row(
            "a_document_overlap_owned_by_a_producer",
            SSLIC,
            &[
                at(DiagCode::Sslic1101, 2, Want::Shown),
                at(
                    DiagCode::W123,
                    2,
                    hidden(Reason::Overlap {
                        owner: OverlapOwner::Producer(Producer::SslicTcl),
                    }),
                ),
            ],
        )
    },
    // 39–41: O111 beside W100.
    Row {
        slot: Some(r#"{"diagnostics": {"W100": false}}"#),
        ..row(
            "o111_survives_a_disabled_w100",
            UNBRACED,
            &[
                at(DiagCode::W100, 2, hidden(DISABLED_EDITOR)),
                at(DiagCode::O111, 2, Want::Shown),
            ],
        )
    },
    row(
        "o111_survives_a_noqa_on_w100",
        "set a 1\n# noqa: W100\nset b [expr $a + 1]\n",
        &[
            at(DiagCode::W100, 3, inline(2)),
            at(DiagCode::O111, 3, Want::Shown),
        ],
    ),
    Row {
        global: Some(r#"{"optimiser": {"enabled": false}}"#),
        ..row(
            "the_optimiser_switch_reaches_o111",
            UNBRACED,
            &[
                at(DiagCode::W100, 2, Want::Shown),
                at(DiagCode::O111, 2, hidden(Reason::OptimiserOff)),
            ],
        )
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_policy::Outcome;
    use tcl_lexer::LineIndex;

    /// Every finding of `report` and every gap, as observations — the lines on
    /// the client's line model over `text`.
    fn observed_in(text: &str, report: &Report) -> Vec<Observed> {
        let line_index = LineIndex::new_lsp(text);
        let mut observed: Vec<Observed> = report
            .iter()
            .map(|(finding, outcome)| Observed {
                code: finding.code,
                line: Some(line_index.line_at(finding.span.start()) + 1),
                state: match outcome {
                    Outcome::Shown { severity, .. } => ObservedState::Shown(Some(*severity)),
                    Outcome::Suppressed(reason) => ObservedState::Suppressed(reason.to_string()),
                },
            })
            .collect();
        observed.extend(report.gaps().map(|(code, reason)| Observed {
            code,
            line: None,
            state: ObservedState::Suppressed(reason.to_string()),
        }));
        observed
    }

    /// The spelling of `reason`'s variant. A new [`Reason`] variant needs an
    /// arm here, an entry in [`REASON_KINDS`] and a row.
    fn kind(reason: Reason) -> &'static str {
        match reason {
            Reason::ReportingOff => "reporting-off",
            Reason::Excluded => "excluded",
            Reason::EncodingAbstention => "encoding-abstention",
            Reason::InlineDirective { .. } => "inline-directive",
            Reason::FileDirective => "file-directive",
            Reason::Disabled(_) => "disabled",
            Reason::DefaultOff => "default-off",
            Reason::OptimiserOff => "optimiser-off",
            Reason::OptimiserProfile { .. } => "optimiser-profile",
            Reason::ShimmerOff => "shimmer-off",
            Reason::Overlap { .. } => "overlap",
        }
    }

    /// Every [`Reason`] variant, as [`kind`] spells it.
    const REASON_KINDS: &[&str] = &[
        "reporting-off",
        "excluded",
        "encoding-abstention",
        "inline-directive",
        "file-directive",
        "disabled",
        "default-off",
        "optimiser-off",
        "optimiser-profile",
        "shimmer-off",
        "overlap",
    ];

    /// The spelling of `layer`. A new [`PolicyLayer`] needs an arm here, an
    /// entry in [`LAYERS`] and a row.
    fn layer_kind(layer: PolicyLayer) -> &'static str {
        match layer {
            PolicyLayer::Global => "global",
            PolicyLayer::Editor => "editor",
            PolicyLayer::Invocation => "invocation",
            PolicyLayer::Project => "project",
        }
    }

    /// Every [`PolicyLayer`], as [`layer_kind`] spells it.
    const LAYERS: &[&str] = &["global", "editor", "invocation", "project"];

    #[test]
    fn every_row_holds_in_the_core_report() {
        let mut failures: Vec<String> = Vec::new();
        for row in ROWS {
            let (text, report) = core_report(row);
            if let Err(failure) = check(row, Surface::Core, &observed_in(&text, &report)) {
                failures.push(failure);
                continue;
            }
            // The core report keeps the typed reason, so the inline
            // directive's line is checked here too; the rendered spelling
            // leaves it to the finding's own row.
            let line_index = LineIndex::new_lsp(&text);
            for expect in holding_on(row, Surface::Core) {
                let Want::Suppressed(want) = expect.want else {
                    continue;
                };
                let typed = report.suppressed().any(|(finding, reason)| {
                    finding.code == expect.code
                        && Some(line_index.line_at(finding.span.start()) + 1) == expect.line
                        && reason == want
                });
                if !typed {
                    failures.push(format!(
                        "row `{}`: no {} at line {:?} suppressed as {want:?}",
                        row.name, expect.code, expect.line
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// The expectations that hold on `surface`: a defect's `today` where it
    /// names the surface, else the row's own.
    fn holding_on(row: &Row, surface: Surface) -> &'static [Expect] {
        match row.defect {
            Some(defect) if defect.surfaces.contains(&surface) => defect.today,
            _ => row.expect,
        }
    }

    #[test]
    fn every_reason_is_covered() {
        let mut reasons: BTreeSet<&str> = BTreeSet::new();
        let mut layers: BTreeSet<&str> = BTreeSet::new();
        let mut note = |want: Want| {
            if let Want::Suppressed(reason) | Want::Gap(reason) = want {
                reasons.insert(kind(reason));
                if let Reason::Disabled(layer) = reason {
                    layers.insert(layer_kind(layer));
                }
            }
        };
        // A reason counts where it holds: a defect's wanted reason does not.
        for row in ROWS {
            for expect in holding_on(row, Surface::Core) {
                note(expect.want);
            }
            if row.runs_on(Surface::Cli) {
                for expect in row.expected(Surface::Cli) {
                    note(expect.want);
                }
            }
        }
        assert_eq!(
            reasons,
            REASON_KINDS.iter().copied().collect::<BTreeSet<_>>(),
            "every reason needs a row"
        );
        assert_eq!(
            layers,
            LAYERS.iter().copied().collect::<BTreeSet<_>>(),
            "every layer needs a row"
        );
    }

    #[test]
    fn row_names_are_unique_and_snake_case() {
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for row in ROWS {
            assert!(names.insert(row.name), "`{}` names two rows", row.name);
            assert!(
                row.name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "`{}` is not snake_case",
                row.name
            );
        }
        assert_eq!(ROWS.len(), 41);
    }

    /// The surfaces each row runs on, as § DP9.4's table lists them.
    #[test]
    fn the_surfaces_each_row_runs_on() {
        use Surface::{
            Cli, CliRewrite, Core, Lsp, LspActions, LspRewrite, Mcp, McpActions, McpRewrite,
        };
        let all = [
            Core, Lsp, LspActions, LspRewrite, Cli, CliRewrite, Mcp, McpActions, McpRewrite,
        ];
        let listed: &[&[Surface]] = &[
            &[Core, Lsp],
            &[Core, Lsp],
            &[Core, Lsp, Cli],
            &[Core, Lsp, Cli],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, LspActions, Cli, Mcp, McpActions],
            &[Core, Lsp, LspActions, Cli, Mcp, McpActions],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli],
            &[Core, Lsp, Cli],
            &[Core, Lsp, Cli],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, LspActions, Cli, Mcp, McpActions],
            &[Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite],
            &[Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite],
            &[Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite],
            &[Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite],
            &[Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite],
            &[Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, Cli, Mcp],
            &[Core, Lsp, LspActions, Cli, Mcp, McpActions],
            &[Core, Lsp, LspActions, Cli, Mcp, McpActions],
            &[Core, Lsp, LspActions, Cli, Mcp, McpActions],
        ];
        assert_eq!(listed.len(), ROWS.len());
        for (row, surfaces) in ROWS.iter().zip(listed) {
            let computed: Vec<Surface> = all
                .iter()
                .copied()
                .filter(|surface| row.runs_on(*surface))
                .collect();
            assert_eq!(&computed, surfaces, "row `{}`", row.name);
        }
    }

    #[test]
    fn the_surface_rules_restate_the_page() {
        let disabled = &ROWS[15];
        assert_eq!(disabled.name, "disabled_in_the_slot");
        assert_eq!(
            disabled.expected(Surface::Cli)[0].want,
            Want::Suppressed(Reason::Disabled(PolicyLayer::Invocation)),
            "the flags occupy the editor layer's slot"
        );
        assert!(disabled.expected(Surface::Lsp).is_empty());

        let overlap = &ROWS[35];
        assert_eq!(overlap.name, "a_same_span_overlap");
        assert_eq!(
            overlap.expected(Surface::Mcp)[1].want,
            Want::Suppressed(Reason::OptimiserOff),
            "the diagnostics tools run with the optimiser off"
        );

        let default_off = &ROWS[18];
        assert_eq!(default_off.name, "default_off");
        assert!(default_off.expected(Surface::Cli).is_empty(), "D21");

        let abstaining = &ROWS[2];
        assert_eq!(
            abstaining
                .expected(Surface::Cli)
                .iter()
                .map(|expect| expect.code)
                .collect::<Vec<_>>(),
            vec![DiagCode::W109],
            "the CLI analyses no abstaining document"
        );

        let fold = &ROWS[29];
        assert_eq!(fold.name, "a_rewrite_shows_under_its_profile");
        assert_eq!(
            fold.expected(Surface::LspActions)[0].want,
            Want::Offered(true)
        );
        assert_eq!(
            fold.expected(Surface::McpRewrite)[0].want,
            Want::Applied(true)
        );
        let outside = &ROWS[30];
        assert_eq!(
            outside.expected(Surface::CliRewrite)[0].want,
            Want::Applied(false)
        );
    }

    #[test]
    fn a_layer_round_trips_through_its_ini_file() {
        use crate::config_ini::{Layer, settings_from_ini};
        for row in ROWS {
            for json in [row.global, row.slot, row.project] {
                let layer = Row::layer(json);
                let back = settings_from_ini(&Row::ini(json), Layer::Global);
                let layer = if layer.as_object().is_some_and(Map::is_empty) {
                    Value::Object(Map::new())
                } else {
                    layer
                };
                assert_eq!(back, layer, "row `{}`: {}", row.name, Row::ini(json));
            }
        }
    }

    #[test]
    fn the_slot_becomes_flags() {
        let per_code = ROWS
            .iter()
            .find(|row| row.name == "a_per_code_toggle_reaches_a_rewrite")
            .expect("the row");
        assert_eq!(
            per_code.slot_flags(),
            SlotFlags {
                disable: vec!["O101".to_owned()],
                enable: Vec::new(),
                profile: Some("full".to_owned()),
            }
        );
        let enabled = ROWS
            .iter()
            .find(|row| row.name == "default_off_turned_on_in_the_slot")
            .expect("the row");
        assert_eq!(enabled.slot_flags().enable, vec!["W242".to_owned()]);
    }

    #[test]
    fn check_names_the_row_and_the_mismatch() {
        let row = &ROWS[4];
        let wrong = [Observed {
            code: DiagCode::W210,
            line: Some(2),
            state: ObservedState::Shown(Some(Severity::Warning)),
        }];
        let failure = check(row, Surface::Core, &wrong).expect_err("W210 is suppressed");
        assert!(failure.contains("inline_noqa_named"), "{failure}");
        assert!(
            check(row, Surface::Lsp, &wrong).is_err(),
            "nothing publishes"
        );
        assert!(check(row, Surface::Lsp, &[]).is_ok());
    }

    #[test]
    fn a_fixed_defect_fails_its_row() {
        let row = ROWS
            .iter()
            .find(|row| row.defect.is_some())
            .expect("a row that records a defect");
        let shown = |code: DiagCode| Observed {
            code,
            line: Some(2),
            state: ObservedState::Shown(Some(Severity::Hint)),
        };
        let today = [shown(DiagCode::W110), shown(DiagCode::O120)];
        assert!(check(row, Surface::Lsp, &today).is_ok());
        let fixed = [shown(DiagCode::W110)];
        let failure = check(row, Surface::Lsp, &fixed).expect_err("the wanted outcome holds");
        assert!(failure.contains("remove the marker"), "{failure}");
        assert!(
            check(row, Surface::Lsp, &[]).is_err(),
            "neither today nor the wanted outcome"
        );
    }

    #[test]
    fn offered_reads_each_subjects_action() {
        let brace = ActionView {
            title: BRACE_TITLE,
            kind: "refactor.rewrite",
            edits: vec![(2, "{$a + 1}")],
        };
        assert!(offered(DiagCode::W100, 2, std::slice::from_ref(&brace)));
        assert!(!offered(DiagCode::W100, 3, std::slice::from_ref(&brace)));
        let noqa = ActionView {
            title: "Suppress S100 with a noqa comment",
            kind: "quickfix",
            edits: vec![(2, "# noqa: S100\n")],
        };
        assert!(offered(DiagCode::S100, 2, std::slice::from_ref(&noqa)));
        let fold = ActionView {
            title: "Fold",
            kind: "quickfix",
            edits: vec![(1, "set x 3")],
        };
        assert!(offered(DiagCode::O101, 1, std::slice::from_ref(&fold)));
        assert!(!offered(DiagCode::O101, 1, &[]));
    }
}
