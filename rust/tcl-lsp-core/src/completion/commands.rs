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

//! Command-position candidates: built-ins, math functions, procedures and the fuzzy fallback.

use super::methods::{
    method_items, original_class_method_items, original_instance_method_candidate_items_at,
};
use super::scoped_environments::{scoped_command_completions, scoped_env_at};
use super::{
    CompletionItem, CompletionKind, FilteredCandidates, MIN_FUZZY_FRAGMENT_CHARS,
    command_context_on_line, decorate_fuzzy_items, dispatch_receiver_of, filter_candidates,
    insertable_source_word, item_match_name, logical_completion_receiver_candidate,
    package_version_floor, word_prefix_at_position,
};
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_registry::CommandRegistry;

/// Map a usage count to its sort bucket (lower is better).
pub(super) fn usage_bucket(count: usize) -> u8 {
    match count {
        c if c >= 50 => 0,
        c if c >= 20 => 1,
        c if c >= 8 => 2,
        c if c >= 3 => 3,
        c if c >= 1 => 4,
        _ => 5,
    }
}

/// Build a `HashMap<command_name, usage_count>` from the
/// analyser's per-document `command_invocations`.  Used as a
/// best-effort, document-local proxy for workspace-wide usage
/// counts.
pub(super) fn document_usage_counts(analysis: &AnalysisResult) -> FxHashMap<String, usize> {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    for inv in &analysis.command_invocations {
        *counts.entry(inv.name.clone()).or_insert(0) += 1;
        if let Some(q) = inv.resolved_qualified_name.as_deref()
            && q != inv.name
        {
            *counts.entry(q.to_owned()).or_insert(0) += 1;
        }
    }
    counts
}

/// What kind of `TclOO` frame the completion cursor sits in — which decides
/// whether the `oo::Helpers` family's bare spellings may be offered.
///
/// Thin position-to-offset wrapper over the crate's single frame
/// classifier, so completion, hover, and the analyser's W123 emitter all
/// answer from the same scope walk.
fn oo_frame_at(
    analysis: &AnalysisResult,
    source: &str,
    line: u32,
    character: u32,
    line_index: &tcl_lexer::LineIndex,
) -> crate::oo_dispatch::OoFrame {
    let offset = crate::definition::byte_offset_at(line_index, source, line, character);
    crate::oo_dispatch::OoFrame::at(analysis, offset)
}

fn builtin_sort_text(name: &str, usage: usize) -> String {
    // `B<usage>_<name>` — built-ins sort after user procs (`A…`)
    // but before any item with no `sort_text`.  The two-digit
    // bucket prevents lexicographic confusion between 1 and 10.
    let rank = usage_bucket(usage);
    format!("B{rank:02}_{name}")
}

pub(super) fn builtin_completions(
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
    partial: &str,
    usage: &FxHashMap<String, usize>,
    tk_advice: bool,
    analysis: &AnalysisResult,
    oo_frame: crate::oo_dispatch::OoFrame,
) -> Vec<CompletionItem> {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let config = input.lexer_config();
    if analysis.body_lexer_config != Some(config) {
        return Vec::new();
    }
    // Availability-gate the command list through the dialect profile: a
    // command whose spec restricts itself to later dialects (`try` is Tcl
    // 8.6+, `lseq` is 9.0+, …) must not be offered in an earlier-dialect
    // buffer, a vendor profile's composed mask admits its embedded Tcl core
    // (8.5 `dict` under f5-iapps), and the subtractive iRules disable list
    // filters the banned commands (§9 of the dialect-profile model).
    // `load_dialect` only *adds* dialect command sets (iRules / Tk / …); it
    // never removes a version-gated core command, so `command_names()`
    // still lists `try` under `tcl8.4`/`tcl8.5` — filter here via the same
    // profile resolution the analyser's W123 uses. An unknown dialect
    // (custom / non-Tcl) resolves to the permissive fallback environment.
    let profile = dialect;
    let Some(context) = crate::document_floor::DocumentFloor::new(analysis, profile).context()
    else {
        return Vec::new();
    };
    // naming.consumer.selected-catalogue-completion-provider
    // docs/design/analysis/name-resolution-proofs/selected-catalogue-completion-provider.md
    // Retain one descriptor from the actual document availability context.
    // A same-name row from another provider cannot decide any later filter.
    let mut candidates: Vec<_> = registry
        .command_names()
        .filter(|name| partial.is_empty() || name.starts_with(partial))
        .filter_map(|name| {
            context
                .resolve_spec(registry, name)
                .map(|spec| (name, spec))
        })
        .filter(|(_, spec)| {
            !spec.traits.contains(tcl_registry::Traits::OPERATOR_COMMAND)
                && !spec
                    .traits
                    .contains(tcl_registry::Traits::DEFINITION_BODY_MEMBER_ONLY)
        })
        .filter(|(_, spec)| oo_frame.admits_selected_spec(spec))
        .filter(|(_, spec)| {
            (tk_advice && context.can_host_package("Tk")) || spec.required_package != Some("Tk")
        })
        .filter(|(_, spec)| {
            let floor = package_version_floor(analysis, spec, profile);
            match (context.keyed_version_range(spec), floor) {
                (Some((min, max)), Some(floor)) => {
                    tcl_registry::version::within_range(floor, min, max)
                }
                _ => spec.available_for_version(floor),
            }
        })
        .collect();
    candidates.sort_unstable_by_key(|(name, _)| *name);
    candidates
        .into_iter()
        .filter_map(|(name, spec)| {
            let insert_text = tcl_syntax::word_rules::lexical_unicode_source_word(name, config)?;
            let count = usage.get(name).copied().unwrap_or(0);
            Some(CompletionItem {
                label: name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: Some(command_detail(spec, registry)),
                sort_text: Some(builtin_sort_text(name, count)),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: spec.hover.as_ref().map(|hover| hover.summary.to_owned()),
            })
        })
        .collect()
}

/// `expr` math-function completions — the bare names
/// (`sin`, `max`, …) that are callable *only* inside an expression.
///
/// Sourced from the registry's own `::tcl::mathfunc::*` specs
/// ([`CommandRegistry::math_function_names`] / `math_function_spec`), which
/// are themselves derived from `tcl_syntax::expr::mathfunc` — one canonical
/// table, no second list here — and gated on the profile's `expr`-grammar
/// version, so `max` never appears under `tcl8.4` and `gamma` only under
/// `tcl9.1`.
///
/// Ranked in the local-symbol tier (`A000_…`, alongside the most-used local
/// procs and ahead of the `B…` built-ins) because at this position they are
/// the candidates that can actually be called: an ordinary command is
/// reachable inside an expression only through a `[…]` substitution, which is
/// a different cursor context.
pub(super) fn math_function_completions(
    registry: &CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
    partial: &str,
) -> Vec<CompletionItem> {
    registry
        .math_function_names(profile)
        .into_iter()
        .filter(|name| partial.is_empty() || name.starts_with(partial))
        .map(|name| {
            let spec = registry.math_function_spec(name, profile);
            CompletionItem {
                label: name.to_owned(),
                insert_text: name.to_owned(),
                kind: CompletionKind::Function,
                detail: Some("expr math function".to_owned()),
                sort_text: Some(format!("A000_{name}")),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: spec
                    .and_then(|s| s.hover.as_ref())
                    .map(|h| h.summary.to_owned()),
            }
        })
        .collect()
}

/// Completion-detail provenance string for a built-in command:
/// `tcllib (PKG)` / `stdlib (PKG)` / `Tk` / `built-in`.  Tcllib takes
/// precedence over a plain `required_package`; a package the profile
/// ships ambiently (an F5 surface — §7.1 axis C) is part of the runtime
/// and reads `built-in`, not like a require-gated stdlib package.
pub(super) fn command_detail(
    spec: &tcl_registry::CommandSpec,
    registry: &tcl_registry::registry::CommandRegistry,
) -> String {
    if let Some(pkg) = spec.tcllib_package {
        format!("tcllib ({pkg})")
    } else if let Some(pkg) = spec.required_package
        && !registry.is_ambient_package(pkg)
    {
        if pkg == "Tk" {
            "Tk".to_string()
        } else {
            format!("stdlib ({pkg})")
        }
    } else {
        "built-in".to_string()
    }
}

/// Render a parameter-list summary for a proc completion's
/// `detail` field.
/// Returns `"(no args)"` for paramless procs, otherwise a
/// space-separated list with `{name default}` for optional
/// params.
pub(super) fn proc_signature_str(proc_def: &ProcDef) -> String {
    if proc_def.params.is_empty() {
        return "(no args)".to_string();
    }
    proc_def
        .params
        .iter()
        .map(|p| {
            if p.has_default {
                let default = p.default_value.as_deref().unwrap_or("");
                format!("{{{} {}}}", p.name, default)
            } else {
                p.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn proc_sort_text(name: &str, usage: usize) -> String {
    // `A<tier><usage>_<name>` — `tier = 0` reserved for
    // single-document user procs; `tier = 1` differentiates
    // same-file procs from workspace ones.
    let rank = usage_bucket(usage);
    format!("A0{rank:02}_{name}")
}

pub(super) fn proc_completions(
    analysis: &AnalysisResult,
    partial: &str,
    usage: &FxHashMap<String, usize>,
) -> Vec<CompletionItem> {
    // naming.source.original-procedure-publications
    // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
    let source_inventory = analysis
        .retained_command_realm()
        .and_then(|realm| realm.original_source_image())
        .and_then(|image| image.try_text().ok())
        .and_then(|source| {
            tcl_compiler::registry_invocation::source_structure::source_procedure_publications(
                source, analysis,
            )
        });
    let mut names = Vec::new();
    if let Some(world) = analysis.original_completed_command_world() {
        // Current publication is independent of canonical declaration metadata.
        // A move changes the candidate spelling; deletion removes it here.
        for publication in world.declarations() {
            for declaration in analysis.original_procedure_declarations() {
                if publication.declaration_site() == declaration.declaration_site()
                    && publication.policy() == declaration.name().policy()
                {
                    names.push((declaration, publication.slot(), publication.policy()));
                }
            }
        }
    } else if let Some(inventory) = source_inventory.as_ref() {
        names.extend(
            inventory
                .candidates(analysis.original_procedure_declarations())
                .into_iter()
                .map(|candidate| {
                    (
                        candidate.declaration(),
                        candidate.source_slot(),
                        candidate.policy(),
                    )
                }),
        );
    } else {
        // Genuine declaration cards remain independent of a represented graph.
        names.extend(
            analysis
                .original_procedure_declarations()
                .map(|declaration| {
                    (
                        declaration,
                        declaration.name().slot(),
                        declaration.name().policy(),
                    )
                }),
        );
    }
    let mut items = Vec::new();
    for (declaration, slot, policy) in names {
        push_native_proc_candidate(declaration, slot, policy, partial, usage, &mut items);
    }
    extend_authored_proc_items(analysis, partial, usage, &mut items);
    items.sort_unstable_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then_with(|| left.insert_text.cmp(&right.insert_text))
    });
    items.dedup_by(|left, right| {
        left.insert_text == right.insert_text && left.detail == right.detail
    });
    items
}

/// Proc-name completions for an `INVOKES_USER_PROC` argument (iRules
/// `call PROC_NAME`), with the fuzzy fallback: this list is the whole
/// response for that cursor context, so when no proc prefix-matches a
/// two-plus-character fragment the closest proc names are offered
/// instead of an empty list.
pub(super) fn invoked_proc_completions(
    analysis: &AnalysisResult,
    partial: &str,
) -> Vec<CompletionItem> {
    let usage = document_usage_counts(analysis);
    let items = proc_completions(analysis, partial, &usage);
    if !items.is_empty() || partial.is_empty() {
        return items;
    }
    let FilteredCandidates {
        candidates: mut fallback,
        fuzzy,
    } = filter_candidates(partial, proc_completions(analysis, "", &usage), |item| {
        item_match_name(item, partial)
    });
    if fuzzy {
        decorate_fuzzy_items(&mut fallback, partial);
    }
    fallback
}

/// Response-level fuzzy fallback for the command-position path: called
/// by [`completions`] only when procs, built-ins, scoped heads,
/// workspace procs, and snippets all produced nothing for the typed
/// fragment.  Re-enumerates the same candidate universe the prefix
/// path consults — `$obj` methods when the command head is a known
/// instance, then user procs, built-ins, and scoped command heads —
/// and offers the closest names via [`filter_candidates`].
pub(super) fn fuzzy_command_fallback(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    dialect: &'static tcl_dialect::DialectProfile,
    line_index: &tcl_lexer::LineIndex,
) -> Vec<CompletionItem> {
    let Some(prefix) = word_prefix_at_position(source, line, character, analysis) else {
        return Vec::new();
    };
    let command_head = prefix.word_index() == 0;
    let Some(partial) = tcl_syntax::word_rules::original_word_cursor_literal_prefix(&prefix) else {
        return Vec::new();
    };
    if partial.chars().count() < MIN_FUZZY_FRAGMENT_CHARS {
        return Vec::new();
    }
    let usage = document_usage_counts(analysis);
    let mut universe: Vec<CompletionItem> = Vec::new();
    // Receiver-method context — the method universe the instance branch of
    // `context_aware_completions` declined to fuzzy-match (see
    // `method_completions`) joins the ranking here, resolved with the same
    // `$var`-vs-bareword gate that branch applies. This candidate universe
    // grants no method navigation or rename identity.
    let cursor = crate::definition::byte_offset_at(line_index, source, line, character);
    let original_context = crate::original_oo::class_completion_context(analysis, source, cursor);
    if let std::ops::ControlFlow::Break(Some(record)) = original_context {
        universe.extend(original_class_method_items(analysis, record, ""));
    } else if let Some(record) =
        crate::original_oo::instance_method_completion_record(analysis, source, cursor)
    {
        universe.extend(original_instance_method_candidate_items_at(
            analysis, record, source, cursor, "", None,
        ));
    }
    if analysis.allows_lexical_declaration_advice()
        && let Some(command) = command_context_on_line(source, line, character, analysis)
        && command.word_index == 1
        && let Some((recv, is_dollar)) = dispatch_receiver_of(&command.head)
        && let Some(registry) = registry
        && let Some((class_q, bucket)) = logical_completion_receiver_candidate(
            analysis,
            registry,
            &command.head,
            &recv,
            is_dollar,
        )
    {
        let object_state =
            crate::definition::object_member_state_at(analysis, source, &recv, line, character);
        let methods = method_items(
            analysis,
            Some(registry),
            class_q,
            bucket,
            object_state,
            dialect,
        );
        if let Some(methods) = methods {
            universe.extend(methods);
        }
    }
    if command_head {
        universe.extend(proc_completions(analysis, "", &usage));
    }
    if command_head && let Some(registry) = registry {
        // Both halves are placement facts — the
        // environment ships Tk ambient (`wish`), or this document required
        // it. No `SpecSurface` bit and no environment *name* is consulted.
        let tk_advice =
            crate::document_floor::DocumentFloor::new(analysis, dialect).advises_package("Tk");
        let oo_frame = oo_frame_at(analysis, source, line, character, line_index);
        universe.extend(builtin_completions(
            registry, dialect, "", &usage, tk_advice, analysis, oo_frame,
        ));
    }
    if command_head && let Some(env) = scoped_env_at(analysis, source, line, character, line_index)
    {
        let present: FxHashSet<String> = universe.iter().map(|i| i.label.clone()).collect();
        universe.extend(
            scoped_command_completions(env, "", prefix.config())
                .into_iter()
                .filter(|item| !present.contains(&item.label)),
        );
    }
    let FilteredCandidates {
        candidates: mut items,
        fuzzy,
    } = filter_candidates(&partial, universe, |item| item_match_name(item, &partial));
    if fuzzy {
        decorate_fuzzy_items(&mut items, &partial);
    }
    items
}

fn extend_authored_proc_items(
    analysis: &AnalysisResult,
    partial: &str,
    usage: &FxHashMap<String, usize>,
    items: &mut Vec<CompletionItem>,
) {
    if let Some(config) = analysis.body_lexer_config {
        for declaration in analysis.original_vendor_procedure_declarations() {
            let input = declaration.name_input();
            let Some(spelling) =
                crate::vendor_declaration::source_word(input, declaration.purpose(), config)
            else {
                continue;
            };
            let Some(label) = crate::vendor_declaration::source_label(input, declaration.purpose())
            else {
                continue;
            };
            if !label.starts_with(partial) && !spelling.starts_with(partial) {
                continue;
            }
            let count = usage.get(&label).copied().unwrap_or(0);
            items.push(CompletionItem {
                label: label.clone(),
                insert_text: spelling,
                kind: CompletionKind::Function,
                detail: Some(format!(
                    "Source declaration: {}",
                    proc_signature_str(declaration.metadata())
                )),
                sort_text: Some(proc_sort_text(&label, count)),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            });
        }
    }
    if analysis.allows_lexical_declaration_advice()
        && let Some(config) = analysis.body_lexer_config
    {
        // Original Logical headers supply insertion data without Native publications.
        for proc_def in analysis
            .all_procs
            .values()
            .filter(|proc_def| proc_def.source_name.is_none())
        {
            let Some(declaration) = analysis.logical_source_procedure_declaration(proc_def) else {
                continue;
            };
            let Some(name) = declaration.name() else {
                continue;
            };
            let Some(spelling) = declaration.source_word(config) else {
                continue;
            };
            if !name.starts_with(partial) && !spelling.starts_with(partial) {
                continue;
            }
            let count = usage.get(name).copied().unwrap_or(0);
            items.push(CompletionItem {
                label: name.to_owned(),
                insert_text: spelling,
                kind: CompletionKind::Function,
                detail: Some(proc_signature_str(declaration.metadata())),
                sort_text: Some(proc_sort_text(name, count)),
                is_snippet: false,
                filter_text: None,
                text_edit: None,
                documentation: None,
            });
        }
    }
}

pub(super) fn extend_builtin_completion_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    position: (&str, u32, u32, &tcl_lexer::LineIndex),
    partial: &str,
    usage: &FxHashMap<String, usize>,
    items: &mut Vec<CompletionItem>,
) {
    let (source, line, character, line_index) = position;
    if let Some(registry) = registry {
        // Tk commands are dialect-gated to Tcl/`tk` already, but they are also
        // offered as package advice from ambient placement — the `tk` dialect (a
        // `wish` document) or a `package require Tk` in this file.  Without
        // that, a plain `.tcl` script must not be offered `button`/`pack`/… .
        // Both halves are placement facts — the
        // environment ships Tk ambient (`wish`), or this document required
        // it. No `SpecSurface` bit and no environment *name* is consulted.
        let tk_advice =
            crate::document_floor::DocumentFloor::new(analysis, profile).advises_package("Tk");
        let oo_frame = oo_frame_at(analysis, source, line, character, line_index);
        items.extend(builtin_completions(
            registry, profile, partial, usage, tk_advice, analysis, oo_frame,
        ));
    }
}

fn push_native_proc_candidate(
    declaration: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ProcDef,
    >,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    partial: &str,
    usage: &FxHashMap<String, usize>,
    items: &mut Vec<CompletionItem>,
) {
    let proc_def = declaration.metadata();
    let original = declaration.name_input();
    let Some(spelling) = tcl_syntax::naming::native_command_source_word(
        policy.recipe(),
        slot,
        original.source_image().channel(),
        original.lexer_config(),
    ) else {
        return;
    };
    let spelling = insertable_source_word(
        spelling,
        &tcl_syntax::naming::native_command_full_name_bytes(slot),
        original.lexer_config(),
    );
    let label = if slot.simple == declaration.name().slot().simple && original.display().is_some() {
        proc_def.name.clone()
    } else {
        slot.simple
            .try_utf8()
            .map_or_else(|_| spelling.clone(), str::to_owned)
    };
    let report = String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(slot)).ok();
    if !label.starts_with(partial)
        && !report
            .as_ref()
            .is_some_and(|name| name.starts_with(partial))
        && !spelling.starts_with(partial)
    {
        return;
    }
    let count = usage
        .get(&label)
        .copied()
        .max(report.as_ref().and_then(|name| usage.get(name)).copied())
        .unwrap_or(0);
    items.push(CompletionItem {
        label: label.clone(),
        insert_text: spelling,
        kind: CompletionKind::Function,
        detail: Some(proc_signature_str(proc_def)),
        sort_text: Some(proc_sort_text(&label, count)),
        is_snippet: false,
        filter_text: None,
        text_edit: None,
        documentation: None,
    });
}
