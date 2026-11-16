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

//! W307: a command word that is not literal, the constant sets that prove it safe and the shapes that excuse it.

use super::{W307Ctx, W307FunctionRange};
use crate::analyser::state::Analyser;
use rustc_hash::FxHashMap;
use std::collections::{HashMap, HashSet};
use tcl_core_types::BodyFrameId;

pub(super) fn w307_function_ranges(
    cu: &crate::compilation_unit::CompilationUnit,
) -> Vec<W307FunctionRange<'_>> {
    let mut func_ranges = vec![W307FunctionRange {
        function: &cu.top_level,
        start: 0,
        end: u32::MAX,
    }];
    for (qname, fu) in &cu.procedures {
        if let Some(ir_proc) = cu.ir_module.procedures.get(qname) {
            func_ranges.push(W307FunctionRange {
                function: fu,
                start: ir_proc.span.start(),
                end: ir_proc.span.end(),
            });
        }
    }
    for (qname, fu) in &cu.methods {
        if let Some(span) = cu
            .ir_module
            .methods
            .get(qname)
            .and_then(|method| method.span)
        {
            func_ranges.push(W307FunctionRange {
                function: fu,
                start: span.start(),
                end: span.end(),
            });
        }
    }
    func_ranges
}

impl Analyser {
    /// Command lookup at the actual dispatch point, including namespace,
    /// imports, aliases, provider state and command allocation lifetime.
    pub(super) fn command_slot_is_present(&self, value: &str, call_offset: u32) -> bool {
        self.head_identities
            .invocation_at_source("", call_offset)
            .lookup_command_word(value)
            .selected_slot_presence()
            == crate::command_binding::SourceCommandSlotPresence::Present
    }

    /// Positioned absence advice, retaining uncertainty about custom fallback.
    fn command_slot_has_absence_advice(&self, value: &str, call_offset: u32) -> bool {
        self.head_identities
            .invocation_at_source("", call_offset)
            .lookup_command_word(value)
            .selected_slot_diagnostic_presence()
            == crate::command_binding::SourceCommandSlotPresence::Absent
    }

    /// A formal's original caller operands are diagnostic candidates only.
    /// Unknown external calls remain open; this never changes runtime lookup,
    /// SSA parameter values or the independently selected receiver protocol.
    fn formal_caller_has_absence_advice(&self, offset: u32) -> bool {
        let Some(registry) = self.registry.as_deref() else {
            return false;
        };
        let binding = self.head_identities.invocation_at_source("", offset);
        let Some(site) = binding.invocation_site() else {
            return false;
        };
        let Some(candidates) = self
            .head_identities
            .source_bindings_ref()
            .declaration_formal_call_values(
                offset,
                registry,
                &crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
                    self.result.resolved_input.as_ref(),
                ),
            )
        else {
            return false;
        };
        !candidates.is_empty()
            && candidates.iter().all(|candidate| {
                candidate
                    .value_in_source(&site.source)
                    .is_some_and(|value| self.command_slot_has_absence_advice(value, offset))
            })
    }

    pub(super) fn original_dispatch_word(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<tcl_lexer::NativeWord> {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let (word, offset) = self
            .head_identities
            .original_written_word_at_span_in_source(&image, span, self.file_lexer_config())?;
        (offset == span.start()).then_some(word)
    }

    pub(super) fn original_namespaced_ensemble(
        &self,
        site: &crate::analyser::state::VarCommandSite,
    ) -> Option<(String, String)> {
        parse_namespaced_ensemble(&self.original_dispatch_word(site.cmd_span)?)
    }

    /// Decide whether a `$var <method>` dispatch site is suppressed (no W307).
    ///
    /// Suppressed when SCCP proves the value is a known command; when an
    /// `in_method` dispatch lacks a proven non-command value; when the var is a
    /// proc parameter or is dispatched ≥2 times in scope (and not tainted /
    /// SCCP-non-command); when a namespaced-ensemble composition resolves; when
    /// the var is an object-factory local or a class instance member; or when
    /// it's a callback-array slot without SCCP non-command evidence.
    pub(super) fn w307_site_suppressed(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        ctx: &W307Ctx<'_>,
    ) -> bool {
        // Prefer the source owner's executed lookup before advisory value
        // candidates or the diagnostic's object-usage heuristics.
        if self
            .head_identities
            .invocation_at_source("", site.cmd_span.start())
            .selected_slot_presence()
            == crate::command_binding::SourceCommandSlotPresence::Present
        {
            return true;
        }
        // Resolve the dispatch's value first: prefer the exact SSA use-version,
        // falling back to the merged constset.  This drops the merged-set false
        // positive on a variable reassigned from a non-command to a known
        // command before the dispatch (`set c x; set c puts; $c ...`).
        let precise = w307_precise_cmd_values(
            ctx.func_ranges,
            site.cmd_span.start(),
            &site.var_name,
            self.registry.as_deref(),
        );
        let effective = precise
            .as_ref()
            .or_else(|| ctx.all_constsets.get(&site.var_name));
        // Method-local advice needs this declaration's exact original read;
        // aggregate values from another function cannot establish its value.
        if site.in_method && precise.as_ref().is_none_or(HashSet::is_empty) {
            return true;
        }
        let call_off = site.cmd_span.start();
        // SCCP concrete evidence the value IS a known command — suppress.
        if effective.is_some_and(|v| {
            !v.is_empty() && v.iter().all(|x| self.command_slot_is_present(x, call_off))
        }) {
            return true;
        }
        // Closed absence advice for every feasible literal overrides the
        // object-usage suppressions below. Initial autoload may still handle
        // the command; a custom fallback or incomplete lookup declines this
        // advice through the shared source owner.
        let sccp_absence_advice = effective.is_some_and(|v| {
            !v.is_empty()
                && v.iter()
                    .all(|x| self.command_slot_has_absence_advice(x, call_off))
        });
        // Proc-parameter / multi-dispatch object-dispatch suppression: a
        // dispatch on a parameter of the enclosing proc (any count), or on a
        // non-parameter local dispatched ≥2 times in the same scope, is
        // evidenced object usage — suppress unless the var is tainted.
        let idx = w307_enclosing_idx(ctx.proc_body_ranges, site.cmd_span.start());
        let enclosing = idx.map_or(ctx.top_scope, |i| ctx.proc_body_ranges[i].2);
        let is_param = idx.is_some_and(|i| ctx.proc_body_ranges[i].3.contains(&site.var_name));
        let dispatch_count = ctx
            .dispatch_counts
            .get(&(enclosing, site.var_name.clone()))
            .copied()
            .unwrap_or(0);
        let dispatcher_suppressed = is_param || dispatch_count >= 2;
        let tainted = ctx
            .tainted_by_scope
            .get(&enclosing)
            .is_some_and(|s| s.contains(&site.var_name));
        let original_caller_absence = is_param && self.formal_caller_has_absence_advice(call_off);
        if dispatcher_suppressed && !tainted && !sccp_absence_advice && !original_caller_absence {
            return true;
        }
        // A genuine scalar substitution followed by literal namespace text
        // retains its own full grammar; value and target checks stay separate.
        if let Some((prefix, tail)) = self.original_namespaced_ensemble(site)
            && let Some(values) = ctx.all_constsets.get(&prefix)
            && !values.is_empty()
            && values.iter().all(|v| {
                let suffix = tcl_syntax::naming::root_unrooted_key(&tail);
                self.command_slot_is_present(&format!("{v}{suffix}"), call_off)
            })
        {
            return true;
        }
        // Object-factory provenance: `$var` holds a factory result in this scope
        // — a designed object handle, so the dispatch is not a static error.
        if ctx
            .factory_object_ranges
            .iter()
            .any(|(owner, s, e, names)| {
                *owner == enclosing
                    && *s <= call_off
                    && call_off <= *e
                    && names.contains(&site.var_name)
            })
        {
            return true;
        }
        // Class instance-variable dispatch inside the class body (component /
        // sub-object) — W307 exemption.
        if ctx
            .snit_var_ranges
            .iter()
            .any(|(s, e, vars)| *s <= call_off && call_off <= *e && vars.contains(&site.var_name))
        {
            return true;
        }
        // Callback-registration array slot: `$state(-command)` /
        // `$state(doneCallback)` dispatches a command the user registered into a
        // switch-style option / callback slot. Unless SCCP has concrete evidence
        // the slot has absence advice (handled above via `sccp_absence_advice`,
        // e.g. `array set state {-command notACommand}` or
        // `set state(-command) notACommand`), treat it as a designed callback
        // dispatch (FP-OBJ-10).
        if !sccp_absence_advice && is_callback_array_slot(&site.var_name) {
            return true;
        }
        false
    }

    /// Whether `site` is a `[cmd]::method` namespaced-ensemble dispatch
    /// (FP-OBJ-07): a command-substitution head composed with a literal
    /// `::method` tail.
    ///
    /// The literal tail is static method-name evidence — the dispatch is
    /// well-formed, with only the namespace prefix computed at run time — so
    /// W307 must not fire. A bare `[cmd] arg` dispatch with no `::method`
    /// tail has no such evidence and still fires.
    pub(super) fn is_namespaced_ensemble_dispatch(
        &self,
        site: &crate::analyser::state::CmdCommandSite,
    ) -> bool {
        self.original_dispatch_word(site.cmd_span)
            .as_ref()
            .is_some_and(command_namespaced_ensemble)
    }
}

/// Retain one original scalar reference and an unchanged namespace suffix.
/// A bare `$prefix::tail` is one variable, not this concatenation.
pub(super) fn parse_namespaced_ensemble(word: &tcl_lexer::NativeWord) -> Option<(String, String)> {
    let (variable, tail) = original_namespace_suffix(word)?;
    if !matches!(
        variable.part,
        tcl_lexer::ExecutablePart::Variable { index: None, .. }
    ) {
        return None;
    }
    let reference = std::str::from_utf8(word.executable_parts().bytes(variable.span)?).ok()?;
    let prefix =
        tcl_syntax::naming::whole_scalar_variable_reference_name(reference, word.config())?;
    Some((prefix.to_owned(), tail.to_owned()))
}

pub(super) fn command_namespaced_ensemble(word: &tcl_lexer::NativeWord) -> bool {
    original_namespace_suffix(word).is_some_and(|(prefix, tail)| {
        matches!(prefix.part, tcl_lexer::ExecutablePart::Command { .. })
            && tail
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
    })
}

fn original_namespace_suffix(
    word: &tcl_lexer::NativeWord,
) -> Option<(&tcl_lexer::SpannedExecutablePart, &str)> {
    if word.group().expand || word.group().kind != tcl_lexer::WordKind::Bare {
        return None;
    }
    let arena = word.executable_parts();
    let [prefix, suffix] = arena.list(arena.root()) else {
        return None;
    };
    if !matches!(
        suffix.part,
        tcl_lexer::ExecutablePart::Text(tcl_lexer::ExecutableText::Original)
    ) || prefix.span.start() != word.span().start()
        || prefix.span.end() != suffix.span.start()
        || suffix.span.end() != word.span().end()
    {
        return None;
    }
    let tail = tcl_syntax::naming::unroot_rooted_key(
        std::str::from_utf8(arena.bytes(suffix.span)?).ok()?,
    )?;
    (!tail.is_empty()).then_some((prefix, tail))
}

/// Harvest `array set arr {k1 v1 k2 v2 …}` literal element values into the
/// constset map keyed by `arr(key)`, so the W307 callback-array suppression
/// can check the *actual* value of `$arr(-command)` against the known-command
/// set.  Without this, the dash-prefixed / callback-suffixed array-key
/// heuristic fires even when SCCP-equivalent literal evidence proves the value
/// is (or isn't) a command.
/// True when `var_name` is an array element `base(key)` whose key denotes a
/// switch-style callback / option registration slot: a dash-prefixed option
/// key (`-command`) or a callback-shaped suffix word
/// (`cmd`/`command`/`callback`/`handler`/`hook`/`proc`). Dispatching such a
/// slot is a designed callback invocation, not a stray non-literal command
/// (FP-OBJ-10); the caller still fires W307 when SCCP proves the slot holds a
/// concrete non-command value.
pub(super) fn is_callback_array_slot(var_name: &str) -> bool {
    // This is a resolved name, so a leading dollar is literal.
    // naming.diagnostics.original-variable-name-anchor
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
    let Some((_base, key)) = crate::naming::split_element_ref(var_name) else {
        return false;
    };
    if key.starts_with('-') {
        return true;
    }
    let k = key.to_ascii_lowercase();
    ["cmd", "command", "callback", "handler", "hook", "proc"]
        .iter()
        .any(|suffix| k.ends_with(suffix))
}

/// Literal element assignment candidates from the retained source May
/// envelope. They suppress W307 only; they are not SSA or executed-store facts.
fn harvest_array_element_set_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
    registry: &tcl_registry::CommandRegistry,
) {
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        let Some(metadata) = fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
        else {
            continue;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&fu.source_lexer_config());
        for (&block, body) in &fu.cfg.blocks {
            for (index, _) in body.statements.iter().enumerate() {
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
                else {
                    continue;
                };
                for assignment in
                    crate::registry_invocation::advisory_value_assignments_with_metadata_context(
                        registry,
                        Some(metadata),
                        tokens,
                    )
                {
                    if tcl_syntax::naming::split_element_ref(&assignment.name).is_none() {
                        continue;
                    }
                    if let crate::registry_invocation::EffectiveInvocationWord::Literal(value) =
                        crate::registry_invocation::effective_invocation_word(
                            &assignment.value,
                            fu.source_lexer_config().escapes,
                            rules,
                        )
                    {
                        out.entry(assignment.name).or_default().insert(value);
                    }
                }
            }
        }
    }
}

pub(super) fn harvest_array_set_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
    context: &tcl_registry::model::ContextRegistry,
) {
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        let Some(metadata) =
            fu.invocation_metadata_context_for_module(context.commands(), &cu.ir_module)
        else {
            continue;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&fu.source_lexer_config());
        for (&block, body) in &fu.cfg.blocks {
            for (index, _) in body.statements.iter().enumerate() {
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
                else {
                    continue;
                };
                let Some(invocation) =
                    crate::registry_invocation::original_callback_invocation_with_metadata_context(
                        context.commands(),
                        metadata,
                        tokens,
                    )
                else {
                    continue;
                };
                let Some(tcl_registry::VarElementsEffect::SetsArrayElementsFromList { values_at }) =
                    invocation.facts.var_elements_effect
                else {
                    continue;
                };
                let target = invocation
                    .facts
                    .arg_roles
                    .iter()
                    .find(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
                    .map(|(index, _)| invocation.facts.argument_offset + usize::from(*index));
                let Some(arr_name) = target.and_then(|index| invocation.argument_literal(index))
                else {
                    continue;
                };
                let Some(list) = invocation
                    .argument_literal(invocation.facts.argument_offset + usize::from(values_at))
                else {
                    continue;
                };
                let items = crate::tcl_expr_eval::split_tcl_list(&list, rules);
                if !items.len().is_multiple_of(2) {
                    continue;
                }
                for pair in items.as_chunks::<2>().0 {
                    let elem_name = format!("{arr_name}({})", pair[0]);
                    out.entry(elem_name).or_default().insert(pair[1].clone());
                }
            }
        }
    }
}

/// Conditional dictionary-body values from the statement's original
/// dictionary read and its represented SSA use version. The binder plan keeps
/// alias captures and the actual retained availability and grammar together.
fn harvest_dict_with_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
    registry: &tcl_registry::CommandRegistry,
) {
    use crate::value_transfer::{
        DictBinder, dict_body_operand_with_metadata_context, dict_body_with_metadata_context,
    };
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        let Some(metadata) = fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
        else {
            continue;
        };
        for (&block_id, block) in &fu.cfg.blocks {
            let Some(ssa_block) = fu.ssa.blocks.get(&block_id) else {
                continue;
            };
            for (index, _) in block.statements.iter().enumerate() {
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block_id, index)
                        .source_tokens()
                else {
                    continue;
                };
                let Some((_, name)) =
                    dict_body_operand_with_metadata_context(registry, metadata, tokens)
                else {
                    continue;
                };
                let root = tcl_syntax::naming::split_element_ref(&name)
                    .map_or(name.as_str(), |(root, _)| root);
                let dictionary = fu
                    .ssa
                    .var_symbol_at(block_id, index, root)
                    .and_then(|symbol| {
                        let version = *ssa_block.statements.get(index)?.uses.get(&symbol)?;
                        match fu
                            .diagnostic_value_facts()
                            .values()
                            .get(&(symbol, version))?
                        {
                            crate::analyses::LatticeValue::Const(value) => {
                                crate::value_transfer::const_text(value)
                            }
                            _ => None,
                        }
                    });
                let Some(binders) = dictionary.as_deref().and_then(|dictionary| {
                    dict_body_with_metadata_context(registry, metadata, tokens, dictionary)
                }) else {
                    continue;
                };
                for binder in binders {
                    if let DictBinder::Key { name, value } = binder {
                        out.entry(name).or_default().insert(value);
                    }
                }
            }
        }
    }
}

/// Per-body tainted variable names: any
/// tainted SSA version of a name disqualifies it from W307 dispatcher
/// suppression.
pub(super) fn build_tainted_by_scope(
    cu: &crate::compilation_unit::CompilationUnit,
) -> FxHashMap<BodyFrameId, HashSet<String>> {
    let tainted_names_of = |fu: &crate::compilation_unit::FunctionUnit| -> HashSet<String> {
        fu.taints
            .iter()
            .filter(|(_, tl)| tl.is_tainted())
            .map(|((sym, _ver), _)| fu.ssa.var_name(*sym).to_owned())
            .collect()
    };
    let mut tainted_by_scope: FxHashMap<BodyFrameId, HashSet<String>> = FxHashMap::default();
    let top_tainted = tainted_names_of(&cu.top_level);
    if !top_tainted.is_empty() {
        tainted_by_scope.insert(
            BodyFrameId::Top(cu.ir_module.procedure_ids.module_id()),
            top_tainted,
        );
    }
    for (id, fu) in cu
        .procedures
        .iter()
        .chain(&cu.methods)
        .chain(cu.body_lowering_functions())
    {
        let names = tainted_names_of(fu);
        if !names.is_empty() {
            tainted_by_scope
                .entry(BodyFrameId::Body(*id))
                .or_default()
                .extend(names);
        }
    }
    tainted_by_scope
}

/// Aggregate constant-string knowledge (var name → flat CONST/CONSTSET value
/// set) across the top level, every proc, and every method body of `cu`,
/// then fold in the element writes each statement states and the keys a
/// `dict with` binds.  Used by the W307 non-literal-command-name check.
pub(super) fn aggregate_constsets(
    cu: &crate::compilation_unit::CompilationUnit,
    context: &tcl_registry::model::ContextRegistry,
) -> std::collections::HashMap<String, HashSet<String>> {
    let mut all_constsets: std::collections::HashMap<String, HashSet<String>> =
        std::collections::HashMap::new();
    let collect_from =
        |fu: &crate::compilation_unit::FunctionUnit,
         out: &mut std::collections::HashMap<String, HashSet<String>>| {
            for ((sym, _ver), lv) in fu.diagnostic_value_facts().values() {
                let Some(values) = lattice_command_values(lv) else {
                    continue;
                };
                let entry = out.entry(fu.ssa.var_name(*sym).to_owned()).or_default();
                for v in values {
                    entry.insert(v);
                }
            }
        };
    collect_from(&cu.top_level, &mut all_constsets);
    for fu in cu.procedures.values() {
        collect_from(fu, &mut all_constsets);
    }
    // A literal `set cmd nope` inside an `oo::class` method body must be
    // captured so SCCP can prove `$cmd` is a non-command — defeating the
    // blanket `in_method` W307 suppression.
    for fu in cu.methods.values() {
        collect_from(fu, &mut all_constsets);
    }

    harvest_array_set_constants(cu, &mut all_constsets, context);
    harvest_array_element_set_constants(cu, &mut all_constsets, context.commands());
    harvest_dict_with_constants(cu, &mut all_constsets, context.commands());
    all_constsets
}

/// Expand a CONST / CONSTSET lattice value into the flat set of its
/// string values, or `None` for any non-string-constant lattice state.
fn lattice_command_values(lv: &crate::analyses::LatticeValue) -> Option<Vec<String>> {
    use crate::analyses::{ConstValue, LatticeValue};
    match lv {
        LatticeValue::Const(ConstValue::String(s)) => Some(vec![s.clone()]),
        LatticeValue::ConstSet(set) => set
            .iter()
            .map(|cv| match cv {
                ConstValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>(),
        _ => None,
    }
}

/// The SCCP value set of `var_name` at the SSA use-version that reaches
/// the dispatch statement at `offset` (W307 per-SSA-version refinement).
///
/// The merged `all_constsets` map unions every version of a variable,
/// so `set c notacommand; set c parse; $c x` wrongly keeps
/// `notacommand` in the set even though only the `parse` version
/// reaches the dispatch. Reading the value at the use site's exact
/// version removes that false positive.
///
/// Purely additive: returns a set only when a CFG statement containing
/// `offset` that *uses* `var_name` is found and its version has a
/// concrete CONST / CONSTSET value — otherwise `None`, and the caller
/// falls back to the merged-set logic. Never broadens a fire into a
/// suppression unsoundly — the value is the exact one flowing into the
/// dispatch.
/// Index of the innermost proc body range containing `off`, or `None` for the
/// `::top` sentinel scope.  `proc_body_ranges` is sorted largest-start-first,
/// so the reverse scan finds the innermost enclosing range (procs don't nest,
/// but `namespace eval` bodies can wrap several, so this stays robust).
pub(super) fn w307_enclosing_idx(
    proc_body_ranges: &[(u32, u32, BodyFrameId, HashSet<String>)],
    off: u32,
) -> Option<usize> {
    proc_body_ranges
        .iter()
        .enumerate()
        .rev()
        .find(|(_, (s, e, _, _))| *s <= off && off <= *e)
        .map(|(i, _)| i)
}

fn w307_precise_cmd_values(
    func_ranges: &[W307FunctionRange<'_>],
    offset: u32,
    var_name: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> Option<HashSet<String>> {
    // Narrowest function range containing `offset`.
    let mut best = None;
    for range in func_ranges {
        if range.start <= offset && offset <= range.end {
            let width = range.end - range.start;
            if best.is_none_or(|(bw, _)| width < bw) {
                best = Some((width, range.function));
            }
        }
    }
    let fu = best?.1;
    // A command-head variable that is not an SSA variable of `fu` has no
    // precise per-version value here.
    let sym = fu.ssa.var_symbol(var_name)?;

    // Narrowest CFG statement containing `offset` that uses `var_name`,
    // reading its SSA use-version (CFG / SSA blocks are parallel-indexed).
    let mut best_width: Option<u32> = None;
    let mut best_version: Option<u32> = None;
    for (block_name, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_name) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let span = fu.abs_span(stmt.span());
            if !(span.start() <= offset && offset <= span.end()) {
                continue;
            }
            let Some(ssa_stmt) = ssa_block.statements.get(idx) else {
                continue;
            };
            let version = ssa_stmt.uses.get(&sym).copied().or_else(|| {
                declaration_head_value_version(
                    fu,
                    *block_name,
                    idx,
                    offset,
                    var_name,
                    sym,
                    registry?,
                )
            });
            let Some(version) = version else {
                continue;
            };
            let width = span.end() - span.start();
            if best_width.is_none_or(|bw| width < bw) {
                best_width = Some(width);
                best_version = Some(version);
            }
        }
    }
    let version = best_version?;
    let lv = fu.diagnostic_value_facts().values().get(&(sym, version))?;
    Some(lattice_command_values(lv)?.into_iter().collect())
}

/// Conditional declaration-local value advice for this exact original head read.
/// No represented SSA use, executed lookup or physical read is created.
fn declaration_head_value_version(
    fu: &crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    offset: u32,
    name: &str,
    symbol: crate::ssa::Symbol,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::ssa::Version> {
    let tokens = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()?;
    let head = tokens.words().first()?;
    if head.source().span.start() != offset {
        return None;
    }
    let crate::ir::WordExpr::Variable { spelling, .. } = head else {
        return None;
    };
    let binding = tokens.source_binding.as_ref()?;
    let reads = binding.declaration_read_occurrences(registry, tokens)?;
    let mut exact = reads.iter().filter(|read| {
        read.name() == name && read.source() == head.source() && read.spelling() == spelling
    });
    let read = exact.next()?;
    if exact.next().is_some() {
        return None;
    }
    let (selected, version) = read.diagnostic_version(&fu.ssa, block, index, registry)?;
    (selected == symbol).then_some(version)
}
