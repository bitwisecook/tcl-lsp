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

//! Substitution templates rebuilt from their parts, against `subst` on every release.

use super::*;

/// `subst`'s literal switches and braced template, as the driver presents a
/// call whose words are all written.
pub(super) struct TemplateInputs<'a> {
    view: tcl_registry::value_transfer::ResolvedInvocationView<'a>,
    context: tcl_registry::value_transfer::AnalysisContext,
}

impl TemplateInputs<'_> {
    /// `subst switches… {template}` under `profile`.
    pub(super) fn new<'a>(
        switches: &[&'a str],
        template: &'a str,
        profile: &str,
    ) -> TemplateInputs<'a> {
        use tcl_registry::value_transfer::{
            AnalysisContext, InvocationLayout, OperandView, ResolvedInvocationView,
        };
        TemplateInputs {
            view: ResolvedInvocationView {
                canonical_command: "subst",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: switches
                    .iter()
                    .chain(std::iter::once(&template))
                    .map(|&text| OperandView {
                        text,
                        kind: tcl_registry::InvocationWordKind::Literal,
                        role: None,
                    })
                    .collect(),
                argument_offset: 0,
                arity: None,
            },
            context: AnalysisContext::detached(tcl_dialect::DialectProfile::find(profile)),
        }
    }
}

impl tcl_registry::value_transfer::AnalysisInputs for TemplateInputs<'_> {
    fn invocation(&self) -> &tcl_registry::value_transfer::ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(
        &self,
        id: tcl_registry::value_transfer::OperandId,
        _domain: tcl_registry::value_transfer::FactDomain,
    ) -> tcl_registry::value_transfer::FactView {
        use tcl_registry::value_transfer::{DeclineReason, ExactValue, FactView};
        self.view
            .operand(id)
            .map_or(FactView::Top(DeclineReason::NotExact), |operand| {
                FactView::Exact(ExactValue::from_literal(operand.text), None)
            })
    }

    fn place(
        &self,
        _id: tcl_registry::value_transfer::OperandId,
    ) -> Result<tcl_registry::value_transfer::PlaceRef, tcl_registry::value_transfer::DeclineReason>
    {
        Err(tcl_registry::value_transfer::DeclineReason::Unsupported)
    }

    fn variable(
        &self,
        _name: &str,
        _domain: tcl_registry::value_transfer::FactDomain,
    ) -> tcl_registry::value_transfer::FactView {
        tcl_registry::value_transfer::FactView::Top(
            tcl_registry::value_transfer::DeclineReason::NotExact,
        )
    }

    fn prior_store(
        &self,
        _place: &tcl_registry::value_transfer::PlaceRef,
        _domain: tcl_registry::value_transfer::FactDomain,
    ) -> tcl_registry::value_transfer::FactView {
        tcl_registry::value_transfer::FactView::Top(
            tcl_registry::value_transfer::DeclineReason::NotExact,
        )
    }

    fn word_structure(
        &self,
        id: tcl_registry::value_transfer::OperandId,
    ) -> Result<
        tcl_registry::value_transfer::WordStructure,
        tcl_registry::value_transfer::DeclineReason,
    > {
        use tcl_registry::value_transfer::{DeclineReason, WordPart, WordStructure};
        let template = self.view.operand(id).ok_or(DeclineReason::NotExact)?.text;
        let end = u32::try_from(template.len() + 1).expect("a short template");
        Ok(WordStructure {
            braced: true,
            quoted: false,
            parts: vec![WordPart::Literal {
                span: tcl_lexer::Span::new(1, end),
                text: template.to_owned(),
            }],
        })
    }

    fn body(
        &self,
        _id: tcl_registry::value_transfer::OperandId,
    ) -> Result<tcl_registry::value_transfer::BodyRegion, tcl_registry::value_transfer::DeclineReason>
    {
        Err(tcl_registry::value_transfer::DeclineReason::Unsupported)
    }

    fn nested(
        &self,
        _script: &str,
        _state: &mut tcl_registry::value_transfer::EvaluationState,
    ) -> tcl_registry::value_transfer::EvalAnswer {
        tcl_registry::value_transfer::EvalAnswer::Declined(
            tcl_registry::value_transfer::DeclineReason::Unsupported,
        )
    }

    fn math_function(
        &self,
        _name: &str,
    ) -> Result<
        tcl_registry::value_transfer::BindingIdentity,
        tcl_registry::value_transfer::DeclineReason,
    > {
        Err(tcl_registry::value_transfer::DeclineReason::Unsupported)
    }

    fn context(&self) -> &tcl_registry::value_transfer::AnalysisContext {
        &self.context
    }
}

/// The variables every template witness reads, set before it runs.
const TEMPLATE_PRELUDE: &str = "set b 5\nset name N\nset c 1\narray set a {5 five}\n";

/// What `subst switches… {template}` prints after the prelude, or `None`
/// when it raises (a piped script's error leaves the exit status alone).
fn subst_oracle(tclsh: &str, switches: &[&str], template: &str) -> Option<String> {
    let mut script = format!("{TEMPLATE_PRELUDE}if {{[catch {{subst");
    for switch in switches {
        script.push(' ');
        script.push_str(switch);
    }
    script.push_str(" {");
    script.push_str(template);
    script.push_str("}} __out]} {exit 1}\nputs -nonewline $__out\n");
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// The template rebuilt from its plan: each escape decoded under
/// `profile`'s grammar, each read of `b` or `name` its prelude value, and
/// each region's script run after the prelude; `None` for any other read,
/// or a region that raises.
fn template_rebuilt(
    tclsh: &str,
    template: &str,
    plan: &tcl_registry::value_transfer::TemplateWordPlan,
    profile: &str,
) -> Option<String> {
    let escapes = tcl_dialect::DialectProfile::find(profile)
        .map_or_default(|profile| profile.grammar.escapes);
    // The plan's spans count the opening brace; the template's text does not.
    let content = |span: tcl_lexer::Span| (span.start() as usize - 1, span.end() as usize - 1);
    let mut pieces: Vec<(usize, usize, String)> = Vec::new();
    for &escape in &plan.escapes {
        let (start, end) = content(escape);
        let decoded = tcl_lexer::backslash_subst_in(&template[start..end], escapes);
        pieces.push((start, end, decoded.into_owned()));
    }
    for read in &plan.reads {
        let value = match (read.name.as_str(), &read.element) {
            ("b", None) => "5",
            ("name", None) => "N",
            _ => return None,
        };
        let (start, end) = content(read.span);
        pieces.push((start, end, value.to_owned()));
    }
    for region in &plan.script_regions {
        let script = format!(
            "{TEMPLATE_PRELUDE}if {{[catch {{{}}} __out]}} {{exit 1}}\nputs -nonewline $__out\n",
            region.script.script
        );
        let (true, value) = run_tcl(tclsh, &script)? else {
            return None;
        };
        let (start, end) = content(region.span);
        pieces.push((start, end, value));
    }
    pieces.sort();
    let mut out = String::new();
    let mut at = 0;
    for (start, end, value) in pieces {
        assert!(start >= at, "{template:?}: overlapping pieces in {plan:?}");
        out.push_str(&template[at..start]);
        out.push_str(&value);
        at = end;
    }
    out.push_str(&template[at..]);
    Some(out)
}

/// `subst`'s template-word plan against the real `tclsh`, per
/// release on `PATH` and under that release's profile, over the page's
/// fourteen programs (`docs/design/compiler/value-transfers.md` § *The
/// template-word plan*; the two procedure programs as the templates they
/// substitute, the computed switch as its proven spelling) and four more —
/// an array index, a backslash `-nobackslashes` leaves, and an unclosed
/// bracket with and without `-nocommands`: where `tclsh` raises — the 9.1
/// positive family below 9.1, the two families together everywhere, the
/// unclosed bracket it substitutes — the plan is the command's error; where it answers, the plan's kinds are the ones `tclsh`
/// runs, probed one kind at a time, and the output rebuilt from the plan's
/// escapes, reads and script regions is the output `tclsh` prints.
#[test]
fn template_witnesses_match_every_release_on_path() {
    use tcl_registry::value_transfer::{DeclineReason, PlanAnswer, resolve_semantics};
    const WITNESSES: &[(&[&str], &str)] = &[
        (&["-novariables"], "a$b[set b]"),
        (&["-nocommands"], "a$b[set b]"),
        (&["-novariables"], "x[expr {$b+1}]"),
        (&["-novariables"], "a[string length $b]"),
        (&["-novariables", "-nocommands"], "a$b[set b]\\x41"),
        (&["-nobackslashes"], "a\\tb"),
        (&[], "a\\$b[set b]"),
        (&["-novariables"], "hello $name"),
        (&["-nocommands"], "a$b"),
        (&[], "[set c 2]"),
        (&["-novariables"], "[incr c]"),
        (&["-variables"], "a$b[set b]"),
        (&["-backslashes"], "a$b[set b]\\x41"),
        (&["-nocommands", "-variables"], "a$b"),
        (&["-nocommands"], "$a([set b])"),
        (&["-nobackslashes"], "a\\$b"),
        (&[], "a[set b"),
        (&["-nocommands"], "a[set b"),
    ];
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("subst").expect("subst"), None, None);
    let semantics = semantics.semantics().expect("the template plan");
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = version.dialect_profile_name();
        for &(switches, template) in WITNESSES {
            let plan = semantics.structure(&TemplateInputs::new(switches, template, profile));
            let label = format!(
                "tclsh{}: subst {switches:?} {{{template}}}",
                version.version_string()
            );
            let Some(printed) = subst_oracle(&tclsh, switches, template) else {
                assert_eq!(
                    plan,
                    PlanAnswer::Declined(DeclineReason::WrongRepresentation),
                    "{label}: tclsh raises"
                );
                continue;
            };
            let PlanAnswer::TemplateWord(plan) = plan else {
                panic!("{label}: tclsh prints {printed:?}, the plan is {plan:?}");
            };
            let runs = |probe: &str, ran: &str| {
                subst_oracle(&tclsh, switches, probe).as_deref() == Some(ran)
            };
            assert_eq!(
                (
                    plan.kinds.backslashes,
                    plan.kinds.commands,
                    plan.kinds.variables
                ),
                (runs("\\x41", "A"), runs("[set b]", "5"), runs("$b", "5")),
                "{label}: the kinds"
            );
            if let Some(rebuilt) = template_rebuilt(&tclsh, template, &plan, profile) {
                assert_eq!(rebuilt, printed, "{label}: rebuilt from {plan:?}");
            }
        }
    }
}
