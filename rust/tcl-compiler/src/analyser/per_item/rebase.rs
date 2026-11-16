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

//! Rebasing a fragment to the body's real position: every span, and every synthetic name that embeds an offset.

use super::super::types::AnalysisResult;
use super::BodyFragment;

pub(super) fn shift(s: tcl_lexer::Span, d: u32) -> tcl_lexer::Span {
    tcl_lexer::Span::new(s.start() + d, s.end() + d)
}

fn rebase_vardef(v: &mut super::super::types::VarDef, d: u32) {
    v.definition_span = shift(v.definition_span, d);
    // The cell-naming word is a span into the same body and rebases with the
    // rest; leaving it body-relative would point a rename at the wrong bytes
    // of the whole document.
    if let Some(span) = v.link_target_span {
        v.link_target_span = Some(shift(span, d));
    }
    for r in &mut v.references {
        *r = shift(*r, d);
    }
}

fn rebase_proc(p: &mut super::super::types::ProcDef, d: u32) {
    p.name_span = shift(p.name_span, d);
    p.body_span = shift(p.body_span, d);
}

fn rebase_method(m: &mut super::super::types::MethodDef, d: u32) {
    m.name_span = shift(m.name_span, d);
    m.body_span = shift(m.body_span, d);
}

fn rebase_class(c: &mut super::super::types::ClassDef, d: u32) {
    c.name_span = shift(c.name_span, d);
    c.body_span = shift(c.body_span, d);
    for m in c.methods.values_mut().chain(c.class_methods.values_mut()) {
        rebase_method(m, d);
    }
    for m in &mut c.constructors {
        rebase_method(m, d);
    }
    if let Some(m) = &mut c.destructor {
        rebase_method(m, d);
    }
    for p in c.properties.values_mut() {
        p.name_span = shift(p.name_span, d);
    }
}

fn rebase_diag(diag: &mut super::super::types::Diagnostic, d: u32) {
    diag.span = shift(diag.span, d);
    for f in &mut diag.fixes {
        f.span = shift(f.span, d);
    }
}

fn rebase_scope(s: &mut super::super::types::Scope, d: u32) {
    if let Some(bs) = &mut s.body_span {
        *bs = shift(*bs, d);
    }
    if let Some(ns) = &mut s.name_span {
        *ns = shift(*ns, d);
    }
    for v in s.variables.values_mut() {
        rebase_vardef(v, d);
    }
    for span in s.original_variable_declaration_spans.values_mut() {
        *span = shift(*span, d);
    }
    for p in s.procs.values_mut() {
        rebase_proc(p, d);
    }
    for c in s.classes.values_mut() {
        rebase_class(c, d);
    }
    for ch in &mut s.children {
        rebase_scope(ch, d);
    }
}

/// Relocate every retained declaration together, including displaced records.
/// Keeping final records and navigation history on one path prevents fragment
/// reuse from assigning an old declaration the coordinates of a new one.
fn rebase_result_declarations(r: &mut AnalysisResult, d: u32) {
    for procedure in r
        .all_procs
        .values_mut()
        .chain(r.superseded_procs.values_mut().flatten())
    {
        rebase_proc(procedure, d);
    }
    for class in r
        .all_classes
        .values_mut()
        .chain(r.superseded_classes.values_mut().flatten())
    {
        rebase_class(class, d);
    }
}

/// Shift every span in an offset-0 body fragment to the body's real position
/// (`d` bytes; suppressed-line keys by `line_delta` lines).  E2 (offset-shift
/// invariance) guarantees this reproduces an in-place walk exactly.
pub(super) fn rebase_fragment(frag: &mut BodyFragment, d: u32, line_delta: i32) {
    rebase_fragment_synthetic_names(frag, d);
    rebase_scope(&mut frag.proc_scope, d);
    let r = &mut frag.result;
    rebase_result_declarations(r, d);
    // Original producer images cannot be moved by adjusting numeric spans.
    clear_fragment_original_producers(r);
    for v in r.all_variables.values_mut() {
        rebase_vardef(v, d);
    }
    for diag in &mut r.diagnostics {
        rebase_diag(diag, d);
    }
    for inv in &mut r.command_invocations {
        inv.range = shift(inv.range, d);
        inv.lookup = inv.lookup.rebased(d);
    }
    for x in &mut r.package_requires {
        x.range = shift(x.range, d);
    }
    for x in &mut r.package_provides {
        x.range = shift(x.range, d);
    }
    for x in &mut r.package_ifneededs {
        x.range = shift(x.range, d);
    }
    for x in &mut r.package_prefer_latest {
        x.range = shift(x.range, d);
    }
    for x in &mut r.source_targets {
        x.range = shift(x.range, d);
        x.original_interpreter_source_load = None;
    }
    r.original_namespace_patterns.clear();
    for x in &mut r.namespace_imports {
        x.range = shift(x.range, d);
    }
    for x in &mut r.namespace_exports {
        x.range = shift(x.range, d);
    }
    for (_, span) in &mut r.proc_declaration_sites {
        *span = shift(*span, d);
    }
    for (_, span) in &mut r.class_body_spans {
        *span = shift(*span, d);
    }
    for off in r.alias_offsets.values_mut() {
        *off += d;
    }
    for off in r.rename_offsets.values_mut() {
        *off += d;
    }
    for records in r.object_methods.values_mut() {
        for om in records {
            om.objdefine_offset += d;
            om.def.name_span = shift(om.def.name_span, d);
            om.def.body_span = shift(om.def.body_span, d);
        }
    }
    for states in r.object_member_state.values_mut() {
        for st in states {
            st.logical_source_members = None;
            st.anchor_offset += d;
            for m in st.methods.values_mut() {
                m.name_span = shift(m.name_span, d);
                m.body_span = shift(m.body_span, d);
            }
        }
    }
    for x in &mut r.auto_path_entries {
        x.range = shift(x.range, d);
    }
    for x in &mut r.qualified_var_refs {
        x.span = shift(x.span, d);
        x.original_name_input = None;
        x.original_namespace = None;
    }
    for x in &mut r.namespace_refs {
        x.span = shift(x.span, d);
    }
    for span in &mut r.namespace_name_unknowns {
        *span = shift(*span, d);
    }
    for x in &mut r.regex_patterns {
        x.range = shift(x.range, d);
    }
    for (span, _) in &mut r.namespace_overrides {
        *span = shift(*span, d);
    }
    for region in &mut r.scoped_command_regions {
        region.span = shift(region.span, d);
    }
    if !r.suppressed_lines.is_empty() {
        let old = std::mem::take(&mut r.suppressed_lines);
        for (line, codes) in old {
            let nl = if line < 0 { line } else { line + line_delta };
            r.suppressed_lines.entry(nl).or_default().extend(codes);
        }
    }
    rebase_fragment_pending(frag, d);
}

/// The string half of the fragment rebase: shift the `<offset>` of every
/// synthetic identity the isolated pass minted (`@dynns@<off>` /
/// `@dynclass@<off>` / `@autoname@<off>`, all body-relative) to the absolute
/// offset the whole-file walk mints for the same construct — wherever the
/// name is embedded: scope names, map keys, qualified names,
/// `resolution_candidates`, alias targets, call-site namespaces.
///
/// Gated on [`BodyFragment::minted_synthetics`] so only names this pass
/// actually minted are rewritten; a literal source name that merely looks
/// like a marker survives untouched (see
/// [`crate::naming::rebase_synthetic_offset_names`]).  Diagnostic *messages*
/// are deliberately not rewritten: no emitter interpolates a synthetic
/// identity into its message text (the corpus differential gate would catch
/// one that started to).
fn rebase_fragment_synthetic_names(frag: &mut BodyFragment, d: u32) {
    if frag.minted_synthetics.is_empty() {
        return;
    }
    let minted = std::mem::take(&mut frag.minted_synthetics);
    let fix = |s: &mut String| {
        if let Some(new) =
            crate::naming::rebase_synthetic_offset_names(s, d, |t| minted.contains(t))
        {
            *s = new;
        }
    };
    rebase_scope_names(&mut frag.proc_scope, &fix);
    rebase_result_names(&mut frag.result, &fix);
    rebase_pending_names(frag, &fix);
    // Publish the rebased spellings back, so the set can never leak a stale
    // body-relative name to a later consumer.
    frag.minted_synthetics = minted
        .iter()
        .map(|n| {
            crate::naming::rebase_synthetic_offset_names(n, d, |t| minted.contains(t))
                .unwrap_or_else(|| n.clone())
        })
        .collect();
}

/// Re-key a `String`-keyed map through `fix` (map keys can embed synthetic
/// names — `"::tool::@dynclass@656"`, `"@dynns@1191::state"`).
fn fix_string_keys<V>(map: &mut std::collections::HashMap<String, V>, fix: &impl Fn(&mut String)) {
    if map.is_empty() {
        return;
    }
    let entries: Vec<(String, V)> = map.drain().collect();
    for (mut k, v) in entries {
        fix(&mut k);
        map.insert(k, v);
    }
}

/// Re-map a `String` set through `fix`.
fn fix_string_set(set: &mut std::collections::HashSet<String>, fix: &impl Fn(&mut String)) {
    if set.is_empty() {
        return;
    }
    *set = set
        .drain()
        .map(|mut s| {
            fix(&mut s);
            s
        })
        .collect();
}

fn fix_proc_names(p: &mut super::super::types::ProcDef, fix: &impl Fn(&mut String)) {
    fix(&mut p.name);
    fix(&mut p.qualified_name);
}

fn fix_class_names(c: &mut super::super::types::ClassDef, fix: &impl Fn(&mut String)) {
    fix(&mut c.name);
    fix(&mut c.qualified_name);
    fix(&mut c.metaclass);
    for s in &mut c.superclasses {
        fix(s);
    }
    for m in &mut c.mixins {
        fix(m);
    }
}

fn fix_var_names(v: &mut super::super::types::VarDef, fix: &impl Fn(&mut String)) {
    fix(&mut v.name);
    if let Some(lt) = &mut v.link_target {
        fix(lt);
    }
}

/// [`rebase_fragment_synthetic_names`] over the fragment's scope tree.
fn rebase_scope_names(s: &mut super::super::types::Scope, fix: &impl Fn(&mut String)) {
    fix(&mut s.name);
    fix_string_keys(&mut s.variables, fix);
    fix_string_keys(&mut s.original_variable_declaration_spans, fix);
    for v in s.variables.values_mut() {
        fix_var_names(v, fix);
    }
    fix_string_keys(&mut s.procs, fix);
    for p in s.procs.values_mut() {
        fix_proc_names(p, fix);
    }
    fix_string_keys(&mut s.classes, fix);
    for c in s.classes.values_mut() {
        fix_class_names(c, fix);
    }
    for sym in &mut s.defined_symbols {
        fix(&mut sym.name);
        fix(&mut sym.qualified_name);
    }
    if let Some(cls) = &mut s.oo_defining_class {
        fix(cls);
    }
    for ch in &mut s.children {
        rebase_scope_names(ch, fix);
    }
}

/// [`rebase_fragment_synthetic_names`] over the fragment's `AnalysisResult`
/// fields the graft merges into the shell.
fn rebase_result_names(r: &mut AnalysisResult, fix: &impl Fn(&mut String)) {
    fix_string_keys(&mut r.all_procs, fix);
    for p in r.all_procs.values_mut() {
        fix_proc_names(p, fix);
    }
    fix_string_keys(&mut r.superseded_procs, fix);
    for defs in r.superseded_procs.values_mut() {
        for p in defs.iter_mut() {
            fix_proc_names(p, fix);
        }
    }
    fix_string_keys(&mut r.all_classes, fix);
    for c in r.all_classes.values_mut() {
        fix_class_names(c, fix);
    }
    fix_string_keys(&mut r.superseded_classes, fix);
    for definitions in r.superseded_classes.values_mut() {
        for class in definitions {
            fix_class_names(class, fix);
        }
    }
    fix_string_keys(&mut r.all_variables, fix);
    for v in r.all_variables.values_mut() {
        fix_var_names(v, fix);
    }
    for (name, _) in &mut r.proc_declaration_sites {
        fix(name);
    }
    for (name, _) in &mut r.class_body_spans {
        fix(name);
    }
    for sym in &mut r.all_defined_symbols {
        fix(&mut sym.name);
        fix(&mut sym.qualified_name);
    }
    for inv in &mut r.command_invocations {
        if let Some(q) = &mut inv.resolved_qualified_name {
            fix(q);
        }
        for cand in &mut inv.resolution_candidates {
            fix(cand);
        }
    }
    fix_string_keys(&mut r.command_aliases, fix);
    for a in r.command_aliases.values_mut() {
        fix(&mut a.qualified_name);
        a.target.rebase(fix);
    }
    fix_string_keys(&mut r.alias_offsets, fix);
    fix_string_keys(&mut r.renamed_commands, fix);
    for target in r.renamed_commands.values_mut() {
        fix(target);
    }
    fix_string_keys(&mut r.rename_offsets, fix);
    fix_string_keys(&mut r.ensemble_subcommand_targets, fix);
    // Same key space as the map's outer keys, so it rebases with it.
    fix_string_set(&mut r.prefixless_ensembles, fix);
    for subs in r.ensemble_subcommand_targets.values_mut() {
        for entry in subs.values_mut() {
            fix(&mut entry.target);
        }
    }
    r.original_namespace_patterns.clear();
    for imp in &mut r.namespace_imports {
        fix(&mut imp.ns);
        if let Some(source) = &mut imp.source {
            fix(&mut source.namespace);
            imp.pattern = source.constructed_pattern();
        }
    }
    for exp in &mut r.namespace_exports {
        fix(&mut exp.ns);
        fix(&mut exp.pattern);
    }
    for (_, ns) in &mut r.namespace_overrides {
        fix(ns);
    }
    clear_fragment_original_producers(r);
    for qref in &mut r.qualified_var_refs {
        fix(&mut qref.qualified_name);
        qref.original_name_input = None;
        qref.original_namespace = None;
    }
    for nref in &mut r.namespace_refs {
        fix(&mut nref.qualified_name);
    }
    fix_string_keys(&mut r.instance_classes, fix);
    for cls in r.instance_classes.values_mut() {
        fix(cls);
    }
    fix_string_set(&mut r.created_instance_commands, fix);
    fix_string_set(&mut r.ambiguous_instance_names, fix);
    fix_string_keys(&mut r.object_methods, fix);
}

/// [`rebase_fragment_synthetic_names`] over the fragment's pending state —
/// the call-site namespaces and qualified targets the tail resolves against
/// the merged (absolute-named) maps.
fn rebase_pending_names(frag: &mut BodyFragment, fix: &impl Fn(&mut String)) {
    fix_string_set(&mut frag.ensembles, fix);
    for (_, ns, _, _) in &mut frag.pending_arity {
        fix(ns);
    }
    for conflict in &mut frag.pending_option_conflicts {
        fix(&mut conflict.namespace);
    }
    for (_, ns, _, _) in &mut frag.disabled_commands {
        fix(ns);
    }
    // Original word/input receipts require the whole-source owner, not rebasing.
    assert_eq!(
        frag.private_ns_calls,
        [] as [super::super::diagnostics::PendingPrivateTclSourceAdvice; 0]
    );
    for cand in &mut frag.pending_user_call_arity {
        fix(&mut cand.cmd_name);
    }

    for (_, _, creation_ns, _) in &mut frag.instances {
        fix(creation_ns);
    }
    for site in &mut frag.const_dispatches {
        fix(&mut site.ns);
    }
    fix_string_keys(&mut frag.ensemble_offsets, fix);
    fix_string_keys(&mut frag.walk_aliases, fix);
    for (target, _) in frag.walk_aliases.values_mut() {
        fix(target);
    }
    fix_string_keys(&mut frag.walk_renames, fix);
    for target in frag.walk_renames.values_mut() {
        fix(target);
    }
    fix_string_keys(&mut frag.walk_alias_offsets, fix);
    fix_string_keys(&mut frag.walk_rename_offsets, fix);
    fix_string_keys(&mut frag.walk_deleted, fix);
}

/// The [`rebase_fragment`] half that shifts the fragment's own *pending*
/// state — the deferred diagnostics and arity / site candidates the
/// aggregator flushes later — as opposed to its `AnalysisResult`.  Split out
/// only to keep each function within the line budget.
pub(super) fn rebase_fragment_pending(frag: &mut BodyFragment, d: u32) {
    for (_, _, _, diag) in &mut frag.pending_arity {
        rebase_diag(diag, d);
    }
    for conflict in &mut frag.pending_option_conflicts {
        rebase_diag(&mut conflict.diagnostic, d);
    }
    for (_, _, _, diag) in &mut frag.disabled_commands {
        rebase_diag(diag, d);
    }
    assert_eq!(
        frag.private_ns_calls,
        [] as [super::super::diagnostics::PendingPrivateTclSourceAdvice; 0]
    );
    for cand in &mut frag.pending_user_call_arity {
        cand.call_off += d;
    }
    assert_eq!(
        frag.pending_ctor_arity,
        [] as [super::super::types::PendingCtorArity; 0]
    );
    // The whole-source owner reconstructs retained lexical calls before grafting.
    // Their image, declaration and word receipts cannot acquire a new source by
    // shifting coordinates; fragment_needs_original_source_owner rejects them.
    assert_eq!(
        frag.pending_next_arity,
        [] as [super::super::types::PendingNextArity; 0]
    );
    // Every dispatch site rebases through its own `rebase`, which owns the
    // full list of spans it carries.  Spelling the fields out here is what
    // let `method_span` go un-rebased on two of these three lists, so W308
    // inside any proc or method body reported — and anchored its quick-fix
    // on — the *fragment's* offsets once the per-item path was in use.
    for s in &mut frag.var_sites {
        s.rebase(d);
    }
    for s in &mut frag.bareword_dispatch_sites {
        s.rebase(d);
    }
    for s in &mut frag.cmd_sites {
        s.rebase(d);
    }
    for s in &mut frag.widget_sites {
        s.cmd_span = shift(s.cmd_span, d);
    }
    for s in &mut frag.const_dispatches {
        s.span = shift(s.span, d);
    }
    for s in &mut frag.instance_class_sites {
        s.span = shift(s.span, d);
    }
    for site in &mut frag.proven_sites {
        site.rebase(d);
    }
    for (_, _, _, off) in &mut frag.instances {
        *off += d;
    }
    for (_, tok, span) in &mut frag.global_defs {
        tok.span = shift(tok.span, d);
        *span = shift(*span, d);
    }
    for off in frag.ensemble_offsets.values_mut() {
        *off += d;
    }
    for candidate in &mut frag.loop_candidates {
        candidate.condition_span = shift(candidate.condition_span, d);
    }
    for off in frag.walk_alias_offsets.values_mut() {
        *off += d;
    }
    for off in frag.walk_rename_offsets.values_mut() {
        *off += d;
    }
    for off in frag.walk_deleted.values_mut() {
        *off += d;
    }
}

fn clear_fragment_original_producers(r: &mut AnalysisResult) {
    for state in r.object_member_state.values_mut().flatten() {
        state.logical_source_members = None;
    }
    r.source_words_memo.clear();
    r.original_vendor_source_names.clear();
    r.original_vendor_procedure_metadata.clear();
    r.original_vendor_class_metadata.clear();
    r.original_vendor_symbol_metadata.clear();
    r.original_symbol_metadata.clear();
    r.original_scoped_bodies.clear();
    r.original_conditional_registry_metadata.clear();
    r.original_vendor_variable_advice.clear();
    r.original_vendor_variable_bodies.clear();
    r.original_variable_symbols.clear();
    r.original_variable_write_advice.clear();
    r.original_variable_roots.clear();
    r.original_variable_symbol_conflicts.clear();
    r.original_variable_alias_sites.clear();
    r.original_variable_name_unknowns.clear();
    r.original_variable_alias_obligations.clear();
    r.original_variable_alias_operands.clear();
    r.original_variable_alias_source_operands.clear();
    r.original_variable_alias_receipts.clear();
}
