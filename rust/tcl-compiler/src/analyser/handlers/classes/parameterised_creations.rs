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

//! Classes a procedure creates under a name computed from its parameters, named by the literal load-time calls that reach it.

use super::super::super::state::Analyser;
use super::super::super::types::{ClassDef, MetaclassProvenance};
use super::{ControlArm, MAX_UNKNOWN_BODY_DEPTH};
use crate::signature_scan::params::bind_proc_formals;
use crate::signature_scan::types::ParamDef;
use tcl_lexer::{Span, TokenType};

/// One creation call inside a proc body whose name word reads a parameter —
/// see [`Analyser::record_literal_parameter_definitions`].
pub(in super::super) struct ParameterisedCreation {
    /// Qualified name of the proc whose body holds the call.
    pub(in super::super) proc_qname: String,
    /// That proc's Tcl formals, including defaults and the trailing `args`
    /// collector.
    pub(in super::super) params: Vec<ParamDef>,
    /// The name word, as written (`${ns}::class`).
    pub(in super::super) name_word: String,
    /// Source offset of the creation call.  Literal-provenance evaluation
    /// executes only dominating assignments before this point.
    pub(in super::super) call_off: u32,
    /// Registry-described control bodies enclosing the creation, from the
    /// proc body inwards.  Each body must be selected by its controller for
    /// the creation to materialise.
    pub(in super::super) control_path: Vec<ControlArm>,
}

#[derive(Clone, Copy)]
struct ParameterisedProc<'a> {
    qname: &'a str,
    pub(in super::super) params: &'a [ParamDef],
}

struct LoadTimeCall {
    pub(in super::super) args: Vec<String>,
    control_path: Vec<ControlArm>,
    call_off: u32,
}

/// Join two observations of the same computed-name class creation.
///
/// The source walk can observe the load-time wrapper with no structural body,
/// while the proc-body walk observes the body under a dynamic placeholder.
/// Empty structural relations are therefore lower bounds at this boundary:
/// the non-empty compatible observation wins, and an already-published
/// factory never disappears behind a later empty observation. Two different
/// concrete superclass relations are not compatible evidence for one class;
/// the caller must publish an abstaining record instead.
///
/// **Only facts a walk actually established take part in the conflict check.**
/// Both sides here are `ClassDef`s, and `ClassDef::default()` fabricates one
/// concrete-looking value — `metaclass: "oo::class"` — that no walk need have
/// read. A record that never saw a creation command's head word carries it
/// while having observed nothing, so it is compared only when
/// [`ClassDef::metaclass_provenance`] says it was read; otherwise the observing
/// side's answer is simply adopted. Without that gate an `oo::define` stub for
/// a class made by a user metaclass contradicted the real metaclass and cost
/// the record its factory and superclasses.
pub(in super::super) fn join_parameterised_class_observations(
    existing: &ClassDef,
    observed: &ClassDef,
) -> Option<ClassDef> {
    let metaclasses_conflict = existing.metaclass_provenance == MetaclassProvenance::Observed
        && observed.metaclass_provenance == MetaclassProvenance::Observed
        && crate::naming::normalise_qualified_name(&existing.metaclass)
            != crate::naming::normalise_qualified_name(&observed.metaclass);
    if existing.qualified_name != observed.qualified_name
        || metaclasses_conflict
        || (!existing.superclasses.is_empty()
            && !observed.superclasses.is_empty()
            && existing.superclasses != observed.superclasses)
    {
        return None;
    }

    let (mut joined, other) = if !existing.superclasses.is_empty()
        && observed.superclasses.is_empty()
        || existing.superclasses == observed.superclasses
            && existing.factory.is_some()
            && observed.factory.is_none()
    {
        (existing.clone(), observed)
    } else {
        (observed.clone(), existing)
    };
    // The stand-in loses to a real reading whichever side won above, so the
    // joined record names the metaclass the source actually wrote.
    if joined.metaclass_provenance != MetaclassProvenance::Observed
        && other.metaclass_provenance == MetaclassProvenance::Observed
    {
        joined.metaclass.clone_from(&other.metaclass);
        joined.metaclass_provenance = MetaclassProvenance::Observed;
    }
    if joined.superclasses.is_empty() {
        joined.superclasses.clone_from(&other.superclasses);
    }
    if joined.factory.is_none() {
        joined.factory.clone_from(&other.factory);
    }
    if joined.class_command_fallback == super::super::super::types::ClassCommandFallback::None {
        joined.class_command_fallback = other.class_command_fallback;
    }
    joined.inheritance_unknown |= other.inheritance_unknown;
    joined.member_set_incomplete |= other.member_set_incomplete;
    // One of these observations *is* the creation, so the joined record is
    // not a cross-file extension stub even when the stub's side won above —
    // go-to-definition must still prefer this file's creation site.
    joined.via_define &= other.via_define;
    // Whichever side lost, its declarations are still declarations: an
    // `oo::define` stub's members belong to this class as surely as the
    // creation body's do. Deliberately after the line above:
    // `absorb_declarations` reads `via_define` on both records to decide who
    // wins a name they both declare, and by here `joined`'s answers "were
    // *both* of these observations stubs" — which is exactly the question
    // that rule needs asked.
    joined.absorb_declarations(other);
    Some(joined)
}

impl Analyser {
    pub(in super::super::super) fn record_literal_parameter_definitions(&mut self) {
        self.record_parameterised_class_definitions();
        // The join has settled, so the heads the walk could not judge can be
        // judged now. Paired here rather than at each of the
        // four entry points that run this tail: the order matters — a
        // creation call can only be classified after the metaclass it names
        // is proved — and pairing them is what makes that order impossible to
        // get wrong. It also guarantees the buffer is drained on every path,
        // so a reused analyser never carries one document's deferrals into
        // the next.
        self.replay_deferred_class_creations();
    }

    fn record_parameterised_class_definitions(&mut self) {
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let candidates = self.parameterised_creation_sites(registry);
        if candidates.is_empty() {
            return;
        }
        let calls = self.load_time_call_arguments();
        for candidate in candidates {
            let Some(values) = calls.get(&candidate.proc_qname) else {
                continue;
            };
            // The ordinary source walk has already parsed this creation body
            // under its unresolved written name.  Re-dispatching the same
            // source span can legitimately be deduplicated by the body
            // walker, so retain that structural record and transplant it
            // onto every name the call-site provenance proves.
            let Some(template) = self.unresolved_class(&candidate).cloned() else {
                continue;
            };
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for call in values {
                if !self.load_time_call_is_reachable(call) {
                    continue;
                }
                let Some(resolved) =
                    self.resolve_parameterised_creation_name(&candidate, &call.args)
                else {
                    continue;
                };
                // Only an absolute name is homed without a namespace model —
                // see the doc comment.
                if !tcl_syntax::naming::is_absolute(&resolved) || !seen.insert(resolved.clone()) {
                    continue;
                }
                self.materialise_parameterised_class(&resolved, &template);
            }
            if !seen.is_empty() {
                self.retract_unresolved_class(&candidate);
            }
        }
    }

    fn load_time_call_is_reachable(&mut self, call: &LoadTimeCall) -> bool {
        let mut body_span = Span::new(0, u32::try_from(self.source.len()).unwrap_or(u32::MAX));
        let env = std::collections::HashMap::new();
        for arm in &call.control_path {
            if !self.static_prefix_falls_through(body_span, arm.controller.span.start(), 0)
                || self.control_body_is_selected("", arm, &env, 0) != Some(true)
            {
                return false;
            }
            body_span = arm.body_span;
        }
        self.static_prefix_falls_through(body_span, call.call_off, 0)
    }

    /// The placeholder record made by the ordinary walk for `candidate`.
    ///
    /// A dynamic written name is intentionally retained until at least one
    /// load-time call proves a real name.  Besides making the earlier walk
    /// visible to this pass, the record is the authoritative parse of the
    /// creation body's members: a second walk over the identical source span
    /// may be suppressed by definition-body deduplication.
    fn unresolved_class(&self, candidate: &ParameterisedCreation) -> Option<&ClassDef> {
        let unresolved = Self::unresolved_class_name(candidate)?;
        self.result.all_classes.get(&unresolved)
    }

    /// Move the already-parsed structure of a computed-name creation onto a
    /// name proved by a literal call site, then derive identity-dependent
    /// facts against the now-complete class lattice.
    fn materialise_parameterised_class(&mut self, resolved: &str, template: &ClassDef) {
        let mut observed = template.clone();
        crate::naming::key_tail(resolved).clone_into(&mut observed.name);
        resolved.clone_into(&mut observed.qualified_name);
        let (mut class, compatible) = if let Some(existing) = self.result.all_classes.get(resolved)
        {
            join_parameterised_class_observations(existing, &observed).map_or_else(
                || {
                    let mut abstaining = existing.clone();
                    abstaining.factory = None;
                    abstaining.inheritance_unknown = true;
                    (abstaining, false)
                },
                |joined| (joined, true),
            )
        } else {
            (observed, true)
        };
        if compatible {
            class.factory = self.class_factory_of(&class);
        }

        let simple = class.name.clone();
        if !self
            .result
            .class_body_spans
            .iter()
            .any(|(qname, span)| qname == resolved && *span == class.body_span)
        {
            self.result
                .class_body_spans
                .push((resolved.to_owned(), class.body_span));
        }
        self.result
            .all_classes
            .insert(resolved.to_owned(), class.clone());
        self.result.global_scope.classes.insert(simple, class);
    }

    /// Resolve one computed creation name by executing the small, proven
    /// constant/provenance slice that dominates it inside its procedure.
    ///
    /// Direct parameter interpolation is the zero-statement case.  Local
    /// assignments may additionally call a user procedure when that call can
    /// be evaluated from registry `const_fold` callbacks and literal inputs.
    /// This also carries a local assigned by a namespace-normalising helper:
    /// its false branch is selected through one registry fold and its final
    /// replacement result through another registry fold.
    /// A renamed, aliased, unknown, effectful, or non-constant call yields no
    /// value and the analysis abstains.
    fn resolve_parameterised_creation_name(
        &mut self,
        candidate: &ParameterisedCreation,
        binding: &[String],
    ) -> Option<String> {
        let actuals: Vec<Option<String>> = binding.iter().cloned().map(Some).collect();
        let mut env: std::collections::HashMap<String, String> =
            bind_proc_formals(&candidate.params, &actuals, self.word_rules())?
                .into_iter()
                .filter_map(|(name, value)| value.map(|value| (name, value)))
                .filter(|(_, value)| !tcl_syntax::naming::is_dynamic_word(value))
                .collect();
        let proc_def = self.result.all_procs.get(&candidate.proc_qname)?.clone();
        let mut body_span = proc_def.body_span;
        for arm in &candidate.control_path {
            self.apply_static_path_prefix(
                &candidate.proc_qname,
                body_span,
                arm.controller.span.start(),
                &mut env,
            )?;
            if !self.control_body_is_selected(&candidate.proc_qname, arm, &env, 0)? {
                return None;
            }
            body_span = arm.body_span;
        }
        self.apply_static_path_prefix(
            &candidate.proc_qname,
            body_span,
            candidate.call_off,
            &mut env,
        )?;
        self.static_word_value(
            &candidate.name_word,
            &candidate.proc_qname,
            &env,
            &mut Vec::new(),
            0,
        )
    }

    /// Drop the record the walk made for a creation whose name it could not
    /// resolve, now that the call sites have named the classes it really
    /// makes.
    ///
    /// The walk records `::T::${ns}::class` verbatim — a class of that name
    /// exists in no interpreter, and leaving it would show a phantom entry in
    /// the outline beside the real one and put a nonsense key in the
    /// workspace class index. Only removed when at least one real name was
    /// proved for that same site, so a creation nothing could settle keeps
    /// exactly the record it had before.
    fn retract_unresolved_class(&mut self, candidate: &ParameterisedCreation) {
        let Some(unresolved) = Self::unresolved_class_name(candidate) else {
            return;
        };
        self.result.all_classes.remove(&unresolved);
        self.result
            .class_body_spans
            .retain(|(qname, _)| qname != &unresolved);
    }

    pub(in super::super) fn unresolved_class_name(
        candidate: &ParameterisedCreation,
    ) -> Option<String> {
        let (owner_ns, _) = crate::naming::key_holder_and_tail(&candidate.proc_qname);
        let unresolved = crate::naming::qualify(owner_ns, &candidate.name_word);
        tcl_syntax::naming::is_dynamic_word(&unresolved).then_some(unresolved)
    }

    /// Every creation call written inside a proc body whose **name word**
    /// interpolates one of that proc's parameters — the sites
    /// [`Self::record_literal_parameter_definitions`] can settle.
    ///
    /// Cheap by construction: the manufacturer-word test is a registry
    /// lookup on the call's first argument, so a body with no `create` /
    /// `new` / `createWithNamespace` in it is rejected before anything is
    /// segmented twice.
    fn parameterised_creation_sites(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<ParameterisedCreation> {
        let mut out = Vec::new();
        for (qname, proc_def) in &self.result.all_procs {
            if proc_def.params_computed || proc_def.params.is_empty() {
                continue;
            }
            self.collect_parameterised_creation_sites(
                registry,
                ParameterisedProc {
                    qname,
                    params: &proc_def.params,
                },
                proc_def.body_span,
                &mut Vec::new(),
                0,
                &mut out,
            );
        }
        out
    }

    fn collect_parameterised_creation_sites(
        &self,
        registry: &tcl_registry::CommandRegistry,
        proc_context: ParameterisedProc<'_>,
        body_span: Span,
        control_path: &mut Vec<ControlArm>,
        depth: u32,
        out: &mut Vec<ParameterisedCreation>,
    ) {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return;
        }
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return;
        };
        for seg in statements {
            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            if let Some(first) = seg.args().first()
                && registry.is_manufacturer_method(first)
                && let Some(name_arg) = registry.uniform_manufacturer_names_instance_at(first)
                && let Some(name_word) = seg.args().get(name_arg)
                && !crate::var_refs::scan_var_ref_forms_with_config(name_word, self.lexer_config())
                    .is_empty()
            {
                out.push(ParameterisedCreation {
                    proc_qname: proc_context.qname.to_owned(),
                    params: proc_context.params.to_vec(),
                    name_word: name_word.clone(),
                    call_off: seg.span.start(),
                    control_path: control_path.clone(),
                });
            }

            let case_call = registry
                .case_invocation(
                    seg.name(),
                    &args,
                    Some(self.analysis_context().context().authoring_query()),
                )
                .and_then(|(_, invocation)| invocation.clause_list_index);
            let mut body_indices =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
            body_indices.sort_unstable();
            body_indices.dedup();
            for body_idx in body_indices {
                let (Some(word), Some(token)) = (
                    seg.args().get(body_idx),
                    seg.arg_tokens().get(body_idx).copied(),
                ) else {
                    continue;
                };
                if case_call == Some(body_idx) {
                    let Some((case, _)) = registry.case_invocation(
                        seg.name(),
                        &args,
                        Some(self.analysis_context().context().authoring_query()),
                    ) else {
                        continue;
                    };
                    for (_, (_, body_token)) in crate::segmenter::flatten_case_list_clauses(
                        &self.source,
                        word,
                        token,
                        &case,
                        self.lexer_config(),
                    ) {
                        if body_token.kind != TokenType::Str {
                            continue;
                        }
                        control_path.push(ControlArm {
                            controller: seg.clone(),
                            body_span: body_token.span,
                        });
                        self.collect_parameterised_creation_sites(
                            registry,
                            ParameterisedProc {
                                qname: proc_context.qname,
                                params: proc_context.params,
                            },
                            body_token.span,
                            control_path,
                            depth + 1,
                            out,
                        );
                        control_path.pop();
                    }
                    continue;
                }
                if token.kind != TokenType::Str {
                    continue;
                }
                control_path.push(ControlArm {
                    controller: seg.clone(),
                    body_span: token.span,
                });
                self.collect_parameterised_creation_sites(
                    registry,
                    ParameterisedProc {
                        qname: proc_context.qname,
                        params: proc_context.params,
                    },
                    token.span,
                    control_path,
                    depth + 1,
                    out,
                );
                control_path.pop();
            }
        }
    }

    /// The **load-time** call sites in this document, keyed by the qualified
    /// name the head word spells, each carrying its argument words.
    ///
    /// A call inside a proc or class body is excluded: it runs at call time,
    /// if ever, so it proves nothing about what sourcing this file creates —
    /// the same rule [`Self::publish_load_level_destructions`] applies.
    fn load_time_call_arguments(&self) -> std::collections::HashMap<String, Vec<LoadTimeCall>> {
        let mut out = std::collections::HashMap::new();
        self.collect_load_time_calls(
            Span::new(0, u32::try_from(self.source.len()).unwrap_or(u32::MAX)),
            &mut Vec::new(),
            0,
            &mut out,
        );
        out
    }

    fn collect_load_time_calls(
        &self,
        body_span: Span,
        control_path: &mut Vec<ControlArm>,
        depth: u32,
        out: &mut std::collections::HashMap<String, Vec<LoadTimeCall>>,
    ) {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return;
        }
        let Some(context) = self.retained_static_source_context() else {
            return;
        };
        let registry = context.commands().as_ref();
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return;
        };
        for seg in statements {
            if !self
                .result
                .offset_is_inside_any_definition_body(seg.span.start())
                && !seg.name().is_empty()
            {
                let qualified = crate::naming::qualify("::", seg.name());
                out.entry(qualified).or_default().push(LoadTimeCall {
                    args: seg.args().to_vec(),
                    control_path: control_path.clone(),
                    call_off: seg.span.start(),
                });
            }

            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            let mut body_indices =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
            body_indices.sort_unstable();
            body_indices.dedup();
            let case_call = registry
                .case_invocation(seg.name(), &args, Some(context.context().authoring_query()))
                .and_then(|(_, invocation)| invocation.clause_list_index);
            for body_idx in body_indices {
                let (Some(word), Some(token)) = (
                    seg.args().get(body_idx),
                    seg.arg_tokens().get(body_idx).copied(),
                ) else {
                    continue;
                };
                if case_call == Some(body_idx) {
                    let Some((case, _)) = registry.case_invocation(
                        seg.name(),
                        &args,
                        Some(context.context().authoring_query()),
                    ) else {
                        continue;
                    };
                    for (_, (_, body_token)) in crate::segmenter::flatten_case_list_clauses(
                        &self.source,
                        word,
                        token,
                        &case,
                        self.lexer_config(),
                    ) {
                        if body_token.kind != TokenType::Str {
                            continue;
                        }
                        control_path.push(ControlArm {
                            controller: seg.clone(),
                            body_span: body_token.span,
                        });
                        self.collect_load_time_calls(body_token.span, control_path, depth + 1, out);
                        control_path.pop();
                    }
                    continue;
                }
                if token.kind != TokenType::Str {
                    continue;
                }
                control_path.push(ControlArm {
                    controller: seg.clone(),
                    body_span: token.span,
                });
                self.collect_load_time_calls(token.span, control_path, depth + 1, out);
                control_path.pop();
            }
        }
    }
}
