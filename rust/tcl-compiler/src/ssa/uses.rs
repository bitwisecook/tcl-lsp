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

//! What a statement reads: `uses_of` and the classified uses of calls, barriers, assignments, command words and the scripts a statement keeps inside.

use super::defs::defs_of_in_context;
use super::{SsaInvocationContext, UseClass};
use crate::ir::{CommandTokens, Statement};
use crate::var_refs::VarReferenceScanner;
use std::collections::{BTreeSet, HashSet};
use tcl_registry::CommandRegistry;
use tcl_syntax::naming::unattributed_dynamic_written_variable_target as is_dynamic_write_target;

mod script_reads;
mod word_scan;

use script_reads::switch_reads;
pub(crate) use word_scan::structural_body_indices;
#[cfg(test)]
pub(super) use word_scan::{BracedWordSite, braced_word_owns_name, scan_selected_expression_roles};
use word_scan::{scan_command_words, scan_nested_substitution_words};

/// Extract variable names used (read) by an IR statement.
///
/// Uses a [`VarReferenceScanner`] to find variable references in word texts
/// and expression trees.
///
/// Returns sorted variable names, excluding variables that are defined
/// by this statement (unless they exhibit read-before-write semantics).
///
/// Names only, dropping the [`UseClass`] classification — use
/// [`uses_of_classified`] when the caller must distinguish a substituted read
/// from one carried by an unevaluated brace-quoted word.
pub fn uses_of(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> Vec<String> {
    if !stmt.is_executable_invocation() {
        return Vec::new();
    }
    uses_of_classified(stmt, scanner, registry)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// The classified reads a statement scan accumulates: names substituted (or
/// evaluated in this frame) by the statement, and names mentioned only inside
/// a brace-quoted word it passes through verbatim.
#[derive(Default)]
pub(super) struct ClassifiedUses {
    substituted: BTreeSet<String>,
    quoted: BTreeSet<String>,
    by_name: BTreeSet<String>,
}

impl ClassifiedUses {
    /// Absorb another scan's two halves, keeping each name in its own bucket.
    /// A name that is substituted *somewhere* stays substituted — the final
    /// `quoted.retain` in [`uses_of_classified`] resolves the overlap once,
    /// at the top, so intermediate merges never have to.
    pub(super) fn merge(&mut self, other: ClassifiedUses) {
        self.substituted.extend(other.substituted);
        self.quoted.extend(other.quoted);
        self.by_name.extend(other.by_name);
    }

    /// Absorb a classified name list (as [`uses_of_classified`] returns it).
    fn merge_classified(&mut self, uses: impl IntoIterator<Item = (String, UseClass)>) {
        for (name, class) in uses {
            match class {
                UseClass::Substituted => self.substituted.insert(name),
                UseClass::Quoted => self.quoted.insert(name),
                UseClass::Name => self.by_name.insert(name),
            };
        }
    }

    /// Drop every name a collapsed body defines itself, from both halves.
    fn remove_defs(&mut self, defs: &HashSet<String>) {
        self.substituted.retain(|v| !defs.contains(v));
        self.quoted.retain(|v| !defs.contains(v));
        self.by_name.retain(|v| !defs.contains(v));
    }
}

/// Explicit standalone [`uses_of`] with each name's [`UseClass`].
///
/// A name reached by both a substituted word and a brace-quoted one is
/// [`UseClass::Substituted`] — the definite read wins.
pub fn uses_of_classified(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> Vec<(String, UseClass)> {
    uses_of_classified_in_context(stmt, scanner, SsaInvocationContext::standalone(registry))
}

pub(super) fn uses_of_classified_in_context(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> Vec<(String, UseClass)> {
    if !stmt.is_executable_invocation() {
        return Vec::new();
    }
    if let Statement::Call {
        tokens: Some(tokens),
        ..
    }
    | Statement::Barrier {
        tokens: Some(tokens),
        ..
    } = stmt
    {
        if !tokens.evaluates_words() {
            return Vec::new();
        }
        if tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments) {
            let mut names = Vec::new();
            for (index, text) in tokens.argv_texts.iter().enumerate() {
                if index == 0 || !tokens.arg_is_braced_literal(index - 1) {
                    for name in selection.word_reads(scanner, text) {
                        let read = (name, UseClass::Substituted);
                        if !names.contains(&read) {
                            names.push(read);
                        }
                    }
                }
            }
            return names;
        }
    }
    if stmt.has_opaque_native_accesses() {
        return Vec::new(); // The retained unknown-read flag is independent of names.
    }
    let mut found = ClassifiedUses::default();
    let mut reads_own_def: BTreeSet<String> = BTreeSet::new();

    match stmt {
        Statement::ExprEval { expr, .. } => {
            found.substituted.extend(expr_vars_for(scanner, expr));
        }

        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. } => {
            uses_in_assignment(
                stmt,
                scanner,
                selection,
                &mut found.substituted,
                &mut found.by_name,
                &mut reads_own_def,
            );
        }

        Statement::Call { .. } => {
            uses_in_call(stmt, scanner, selection, &mut found, &mut reads_own_def);
        }

        // A braced return value is literal: `proc f {} { return {$y} }`
        // returns the two characters `$y` and reads nothing.
        // tclsh-proof: tclsh8.6.14 — `proc f {} { return {$y} }; puts [f]`
        // prints `$y` with `y` undefined.
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => {
            if let Some(v) = value {
                let sink = if *braced {
                    &mut found.quoted
                } else {
                    &mut found.substituted
                };
                sink.extend(selection.word_reads(scanner, v));
            }
            if let Some(e) = expr {
                found.substituted.extend(expr_vars_for(scanner, e));
            }
        }

        Statement::Barrier { .. } => {
            uses_in_barrier(stmt, scanner, selection, &mut found, &mut reads_own_def);
        }

        // A non-lowered (glob/regexp/fall-through) `switch` is kept opaque as a
        // single `Statement::Switch` in the block. Recover the subject + arm /
        // default body reads so a variable read only as the subject or only
        // inside an arm body isn't reported unused.
        Statement::Switch {
            subject,
            arms,
            default_body,
            patterns_braced,
            ..
        } => {
            found.merge(switch_reads(
                subject,
                arms,
                default_body.as_ref(),
                *patterns_braced,
                scanner,
                selection,
            ));
            // The subject is read *before* any arm assigns, so it stays a live
            // read even when an arm also defines it (`defs_of` may now report
            // the subject var as switch-defined). Without this the read-before-
            // def of the subject would be filtered out below.
            for v in selection.word_reads(scanner, subject) {
                reads_own_def.insert(v);
            }
        }

        // Other structured IR statements (If, For, While, …) are flattened by
        // the CFG builder before SSA construction, so they never reach here.
        _ => {}
    }

    finish_classified_uses(found, &reads_own_def, stmt, selection)
}

/// Consumed typed statements retain their original roles at the exact source point.
pub(super) fn uses_of_classified_with_source(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    tokens: Option<&CommandTokens>,
) -> Vec<(String, UseClass)> {
    let mut reads = uses_of_classified_in_context(stmt, scanner, selection);
    if !matches!(
        stmt,
        Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
            | Statement::Incr { .. }
    ) {
        return reads;
    }
    let mut selected = ClassifiedUses::default();
    selected_handler_name_reads(tokens, &[], selection, &mut selected, &mut BTreeSet::new());
    for name in selected.by_name {
        if let Some((_, class)) = reads.iter_mut().find(|(existing, _)| *existing == name) {
            if *class == UseClass::Quoted {
                *class = UseClass::Name;
            }
        } else {
            reads.push((name, UseClass::Name));
        }
    }
    reads
}

fn finish_classified_uses(
    found: ClassifiedUses,
    reads_own_def: &BTreeSet<String>,
    stmt: &Statement,
    selection: SsaInvocationContext<'_>,
) -> Vec<(String, UseClass)> {
    // Exclude variables defined by this statement, unless they're
    // read-before-write.  Route through the registry so
    // `trace add variable` defs come from the registry's VarWrite
    // role rather than a string match.
    let defs: HashSet<String> = defs_of_in_context(stmt, Some(selection))
        .into_iter()
        .collect();
    // A name reached more than one way keeps its most definite class, and the
    // three are ordered by what a consumer may do with them: a substituted
    // word is a read *and* an operand something may rewrite; a name argument
    // is a read with nothing to rewrite; a quoted mention is neither. So
    // `lappend a $a` is Substituted (there is an operand), `lappend a {$a}` is
    // Name (the braced mention adds nothing to the name argument), and the
    // weaker classes drop the name the stronger one already asserts.
    let ClassifiedUses {
        substituted,
        mut quoted,
        mut by_name,
    } = found;
    by_name.retain(|v| !substituted.contains(v));
    quoted.retain(|v| !substituted.contains(v) && !by_name.contains(v));
    substituted
        .into_iter()
        .map(|v| (v, UseClass::Substituted))
        .chain(by_name.into_iter().map(|v| (v, UseClass::Name)))
        .chain(quoted.into_iter().map(|v| (v, UseClass::Quoted)))
        .filter(|(v, _)| !defs.contains(v) || reads_own_def.contains(v))
        .collect()
}

/// Variable reads of a [`Statement::Barrier`]: its head + non-body argument
/// words, plus the scope-alias name a `dict with` / `dict update` unpacks.
/// Extracted from [`uses_of_classified`].
fn uses_in_barrier(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    let Statement::Barrier {
        command,
        canonical_command,
        args,
        tokens,
        ..
    } = stmt
    else {
        return;
    };
    scan_command_words(
        command,
        canonical_command.as_deref(),
        args,
        tokens.as_ref(),
        scanner,
        selection,
        found,
    );
    if tokens
        .as_ref()
        .is_some_and(|tokens| selection.source_structure(tokens).is_some())
    {
        selected_handler_name_reads(tokens.as_ref(), &[], selection, found, reads_own_def);
        return;
    }
    // Scope-alias subcommands (`dict with` / `dict update` — any
    // resolved subcommand with `creates_scope_alias`): the aliased
    // variable name is a plain string, not a $-substitution, so
    // scan_word misses it.
    //
    // The variable arg carries both VarRead and VarWrite roles.
    // When barrier defs route through the registry, the same name
    // appears in `defs` from the VarWrite query.  The closing
    // filter in `uses_of_classified`
    // (`!defs.contains(v) || reads_own_def.contains(v)`) would
    // then drop the var unless we mark it as reads-own-def here.
    // Without this, a proc whose only reference to a parameter is
    // `dict with $param {}` would produce a false unused-parameter
    // diagnostic.
    let resolution = tokens.as_ref().and_then(|tokens| selection.resolve(tokens));
    if let Some(crate::registry_invocation::RegistryInvocationResolution::Resolved(facts)) =
        resolution
    {
        if !facts
            .traits
            .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS)
        {
            return;
        }
        let Some(effective) = tokens
            .as_ref()
            .and_then(crate::registry_invocation::effective_command_words)
        else {
            return;
        };
        for &(index, role) in &facts.arg_roles {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            let Some(word) = effective
                .words
                .get(facts.argument_offset + usize::from(index) + 1)
            else {
                continue;
            };
            let Some(name) = crate::registry_invocation::invocation_word(word).literal() else {
                continue;
            };
            let name = tcl_syntax::naming::normalise_var_name_braced(name, true).to_owned();
            found.by_name.insert(name.clone());
            reads_own_def.insert(name);
        }
        return;
    }
    if tokens.is_some() || !selection.standalone {
        return;
    }
    let creates_scope_alias = registry
        .get(command)
        .zip(args.first())
        .and_then(|(spec, sub)| spec.resolve_subcommand(sub))
        .is_some_and(|sub| sub.creates_scope_alias);
    if !creates_scope_alias {
        return;
    }
    let arguments: Vec<_> = args.iter().map(String::as_str).collect();
    for index in registry.arg_indices_for_role(command, &arguments, tcl_registry::ArgRole::VarWrite)
    {
        if let Some(name) = args
            .get(index)
            .map(|name| crate::naming::normalise_var_name(name).to_owned())
        {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name);
        }
    }
}

/// Variable reads of an assignment-style statement (`set` / `set =expr` /
/// `set =value` / `incr`). A *dynamic* target name (`set $p …`) reads its
/// name-bearing variable(s) (the genuine read the dead-store / unused checks
/// need — `defs_of` withholds the static def); a static target that the RHS
/// also reads is recorded as read-before-write. Extracted from [`uses_of`].
/// The value word an aliased / renamed `set` (`interp alias {} myset {} set` /
/// `rename set myset`) stores, or `None` when the `Call` is not a
/// value-passthrough store in the `set VAR VALUE` shape.
///
/// Keyed off the *canonical* command's `Set` lowering hook — the registry's
/// own "this is a value-passthrough store" fact — so the read scan matches the
/// un-aliased `set`, never the source spelling. The two-arg / single-def guard
/// restricts it to the setter shape (no `interp alias` prepended args shifting
/// the value word out of `args[1]`, and not the one-arg getter, which has no
/// def).
fn canonical_set_value<'a>(
    command: &str,
    canonical_command: Option<&str>,
    args: &'a [String],
    defs: &[String],
    selection: SsaInvocationContext<'_>,
    tokens: Option<&CommandTokens>,
) -> Option<&'a str> {
    let registry = selection.registry;
    if defs.len() != 1 {
        return None;
    }
    if let Some(tokens) = tokens {
        let invocation = selection.normal(tokens)?;
        let value_argument = invocation.stored_value_argument()?;
        return args
            .get(invocation.written_argument(value_argument)?)
            .map(String::as_str);
    }
    if !selection.standalone || args.len() != 2 {
        return None;
    }
    let canon = canonical_command.unwrap_or(command);
    let is_set = registry.get(canon).and_then(|s| s.lowering_hook)
        == Some(tcl_registry::hooks::LoweringHookId::Set);
    is_set.then(|| args[1].as_str())
}

/// Reads of a `set`-style value word, matching what the un-aliased `set`
/// lowering captures: an `[expr {…}]` value is parsed as an expression (so a
/// braced `$x`, which plain word scanning would miss, is seen); any other value
/// word is scanned for `$`-substitutions, recursing into `[...]` command
/// substitutions. Alias-derived nested expression reads already retained in
/// [`Statement::Call::reads`] complement this fallback; this path preserves
/// the scanner's requested base/element qualification for direct registry
/// spellings.
pub(super) fn set_value_reads(
    value: &str,
    tokens: Option<&CommandTokens>,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> BTreeSet<String> {
    let registry = selection.registry;
    // naming.variable.selected-setter-expression-reads
    // docs/design/analysis/name-resolution-proofs/variable-selected-setter-expression-reads.md
    // Queried commands already use original entered/conditional child owners
    // in scan_nested_substitution_words. A written head cannot add another
    // evaluator after that lookup has selected a different or unknown handler.
    if tokens.is_some_and(|tokens| tokens.source_binding.is_some()) {
        return scanner.scan_lexical_word(value, registry);
    }
    let trimmed = value.trim();
    // The registry carries the expression-language profile; the scanner owns
    // the exact word lexer config used to recover the `[expr …]` interior.
    let profile = registry.profile();
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
        && let Some((_, _, expr_arg, _)) =
            crate::lowering_hooks::extract_single_expr_arg_with_config(
                inner,
                &crate::alias::CommandAliasMap::new(),
                "::",
                registry,
                None,
                scanner.lexer_config(),
            )
    {
        let expr = crate::parse_expr_for_profile(&expr_arg, profile);
        return if scanner.element_qualified() {
            expr.vars_element_qualified_with_config(scanner.lexer_config())
                .into_iter()
                .collect()
        } else {
            expr.vars_with_config(scanner.lexer_config())
                .into_iter()
                .collect()
        };
    }
    selection.word_reads(scanner, value)
}

/// Named reads from the independently selected Normal or conditional Logical
/// role owner. Literal operand data keeps empty names and dollar sigils.
fn selected_handler_name_reads(
    tokens: Option<&CommandTokens>,
    defs: &[String],
    selection: SsaInvocationContext<'_>,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    if let Some(source) = tokens.and_then(|tokens| selection.source_structure(tokens)) {
        let Some(traits) = source.traits() else {
            return;
        };
        for (argument, role) in source.written_roles() {
            if role != tcl_registry::ArgRole::VarRead
                && !(role == tcl_registry::ArgRole::VarWrite
                    && traits.contains(tcl_registry::Traits::READS_BEFORE_WRITE))
            {
                continue;
            }
            if let Some(name) = source.argument_literal(argument) {
                let name = tcl_syntax::naming::normalise_var_name_braced(&name, true).to_owned();
                found.by_name.insert(name.clone());
                if defs.contains(&name) {
                    reads_own_def.insert(name);
                }
            }
        }
        return;
    }
    if let Some(invocation) = tokens.and_then(|tokens| selection.normal(tokens)) {
        for (index, role) in invocation.variable_roles() {
            if role != tcl_registry::ArgRole::VarRead
                && !(role == tcl_registry::ArgRole::VarWrite
                    && invocation
                        .variable_traits()
                        .contains(tcl_registry::Traits::READS_BEFORE_WRITE))
            {
                continue;
            }
            if let Some(name) = invocation.argument_literal(index) {
                let name = tcl_syntax::naming::normalise_var_name_braced(&name, true).to_owned();
                found.by_name.insert(name.clone());
                if defs.contains(&name) {
                    reads_own_def.insert(name);
                }
            }
        }
    }
}

fn uses_in_call(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    let Statement::Call {
        command,
        canonical_command,
        args,
        defs,
        reads,
        reads_own_defs,
        tokens,
        ..
    } = stmt
    else {
        return;
    };
    scan_command_words(
        command,
        canonical_command.as_deref(),
        args,
        tokens.as_ref(),
        scanner,
        selection,
        found,
    );
    let vars_found = &mut found.substituted;
    if let Some(value) = canonical_set_value(
        command,
        canonical_command.as_deref(),
        args,
        defs,
        selection,
        tokens.as_ref(),
    ) {
        for v in set_value_reads(value, tokens.as_ref(), scanner, selection) {
            if defs.iter().any(|d| d.as_str() == v) {
                reads_own_def.insert(v.clone());
            }
            vars_found.insert(v);
        }
    }
    let retained_roles = selection.standalone
        || tokens.as_ref().is_some_and(|tokens| {
            tokens.carries_proved_names() || selection.normal(tokens).is_some()
        });
    // `reads` are the registry's `ArgRole::VarRead` positions and
    // `reads_own_defs` its `READS_BEFORE_WRITE` targets: both name the cell
    // rather than substituting it, so they are `UseClass::Name` — a real read
    // with no operand word behind it.
    for name in reads.iter().filter(|_| retained_roles) {
        found.by_name.insert(name.clone());
        // A name this statement both reads and defines is read *before* it is
        // written — the same rule `uses_in_barrier` applies to a `dict with`
        // scope alias. Without it the closing def-filter in
        // `uses_of_classified` drops the read, and the store feeding
        // `puts [incr n]` looks overwritten-before-read (#2050).
        if defs.contains(name) {
            reads_own_def.insert(name.clone());
            // …and the call's embedded commands write it: its words read it
            // before, between and after that write, and which version a word
            // names cannot be told, so no word of it is an operand to
            // forward. The read stays one by name, of the version before the
            // write.
            found.substituted.remove(name);
        }
    }
    if *reads_own_defs && retained_roles {
        for name in defs {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
    selected_handler_name_reads(tokens.as_ref(), defs, selection, found, reads_own_def);
    // A destroying command (`unset a(k)`) consumes the target's *existence*:
    // the killed store is not dead — deleting it would make the unset error
    // on every call. Record the prior version as a read (DESTROYS_VARIABLE,
    // matching the pre-per-element behaviour the base-level use gave).
    let facts = tokens.as_ref().and_then(|tokens| selection.resolve(tokens));
    let traits = match facts.as_ref() {
        Some(crate::registry_invocation::RegistryInvocationResolution::Resolved(facts)) => {
            facts.traits
        }
        _ if tokens.is_none() && selection.standalone => registry
            .get(canonical_command.as_deref().unwrap_or(command))
            .map_or(tcl_registry::Traits::empty(), |spec| spec.traits),
        _ => tcl_registry::Traits::empty(),
    };
    let destroys = traits.contains(tcl_registry::Traits::DESTROYS_VARIABLE);
    if destroys {
        for name in defs {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
    // A conditional writer (`regexp`, `scan`, `binary scan`) stores into its
    // targets only on the match / conversion path; on the other path tclsh
    // leaves each previous value in place and never creates a target that did
    // not exist (8.4.20 through 9.1b0, all identical). So the definition this
    // statement appears to kill is still live, and deleting the store feeding
    // it changes the program: `set a before; regexp {(x)(y)} zz a b; puts $a`
    // prints `before`, and with the store gone it failed outright with
    // `can't read "a"` (#2051).
    //
    // Modelled exactly like the destroyer above, and for the same reason: the
    // def stays, and the prior version becomes a read. A def whose evaluated
    // outcome preserves its place holds that prior version
    // (`SccpResult::preserved`), which is how W210 sees a no-match read.
    // The same commands nested in a word or a condition take the same read
    // from `ir_helpers::variable_write_effects_from_commands`.
    //
    // `invocation_traits` rather than `get`, because `binary scan` carries the
    // trait on its *subcommand*.
    let conditionally_writes = traits.contains(tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE);
    if conditionally_writes {
        for name in defs {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
}

fn uses_in_assignment(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    vars_found: &mut BTreeSet<String>,
    by_name: &mut BTreeSet<String>,
    reads_own_def: &mut BTreeSet<String>,
) {
    // A brace-quoted target's `$` is part of a literal name, not a
    // substitution: it is neither a dynamic target nor a read of the
    // `$`-less lookalike.
    let note_reads_own = |name: &str,
                          braced: bool,
                          scanner: &VarReferenceScanner,
                          vars_found: &BTreeSet<String>,
                          rod: &mut BTreeSet<String>| {
        let norm = scanner.canonical_name_braced(name, braced);
        if vars_found.contains(norm) {
            rod.insert(norm.to_owned());
        }
    };
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        } => {
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(selection.word_reads(scanner, name));
            }
        }
        Statement::AssignExpr {
            name,
            name_braced,
            expr,
            ..
        } => {
            vars_found.extend(expr_vars_for(scanner, expr));
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(selection.word_reads(scanner, name));
            } else {
                note_reads_own(name, *name_braced, scanner, vars_found, reads_own_def);
            }
        }
        Statement::AssignValue {
            name,
            name_braced,
            value,
            tokens,
            ..
        } => {
            vars_found.extend(selection.word_reads(scanner, value));
            // `scan_word` declines to substitute inside braces, exactly as the
            // *word* parser does — but a nested `[expr {$x + 1}]` substitutes
            // its own braced body, and that read is real. The call path has
            // recovered it; an assignment value substitutes by the same
            // rules, so `set r [list [expr {0 in $x}]]` reads `x` just as
            // `puts [list [expr {0 in $x}]]` does. Same owner, one more
            // statement kind — not a second recovery.
            scan_nested_substitution_words(
                tokens.as_ref(),
                scanner,
                selection,
                vars_found,
                by_name,
            );
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(selection.word_reads(scanner, name));
            } else {
                note_reads_own(name, *name_braced, scanner, vars_found, reads_own_def);
            }
        }
        Statement::Incr {
            name,
            name_braced,
            amount,
            amount_braced,
            ..
        } => {
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(selection.word_reads(scanner, name));
            } else {
                // `incr a` names the cell it mutates; there is no `$a` word.
                let norm = scanner.canonical_name_braced(name, *name_braced);
                by_name.insert(norm.to_owned());
                reads_own_def.insert(norm.to_owned());
            }
            // A braced amount substitutes nothing: `incr x {$n}` reads no
            // `n` (it raises `expected integer but got "$n"`).
            if let Some(amt) = amount.as_ref().filter(|_| !*amount_braced) {
                vars_found.extend(selection.word_reads(scanner, amt));
            }
        }
        _ => {}
    }
}

/// The expression-AST variable reads, named per the scanner's qualification
/// mode — element-qualified for the SSA build, base names otherwise.
fn expr_vars_for(
    scanner: &VarReferenceScanner,
    expr: &crate::expr_ast::ExprNode,
) -> BTreeSet<String> {
    if scanner.element_qualified() {
        expr.vars_element_qualified_with_config(scanner.lexer_config())
            .into_iter()
            .collect()
    } else {
        expr.vars_with_config(scanner.lexer_config())
            .into_iter()
            .collect()
    }
}
