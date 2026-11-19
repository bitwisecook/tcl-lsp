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

//! [incr Tcl] class-scoped procedures and the class-definer family they are read under.

use super::call_resolution::resolved_command_name;
use super::namespace_context_at;
use crate::original_context::CurrentSourcePosition;
use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::CommandRegistry;

/// Family metadata from the complete input and command store retained by the
/// analysis. Missing metadata is distinct from a known non-Itcl family; it does
/// not establish native class identity, package loading or edit permission.
pub(crate) fn class_definer_family(
    analysis: &AnalysisResult,
    class: &tcl_compiler::analyser::types::ClassDef,
) -> Option<tcl_registry::definer::DefinerFamily> {
    definer_family_in_registry(class, class_family_registry(analysis)?)
}

/// The definition-body grammar of the family that declared `class`, from the
/// analysis's own registry.
pub(crate) fn class_definition_grammar(
    analysis: &AnalysisResult,
    class: &tcl_compiler::analyser::types::ClassDef,
) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
    class_family_registry(analysis)?
        .get(&class.metaclass)?
        .definition_body
}

fn class_family_registry(analysis: &AnalysisResult) -> Option<&CommandRegistry> {
    if analysis.analysis_context_unavailable.is_some() {
        return None;
    }
    let input = analysis.resolved_input.as_ref()?;
    if !analysis
        .retained_command_realm()?
        .matches_resolved_analysis_input(input)
    {
        return None;
    }
    analysis.resolved_registry()
}

/// Family metadata in an explicitly supplied store. This does not establish a
/// native class, current command table, loader effect or editable identity.
fn definer_family_in_registry(
    class: &tcl_compiler::analyser::types::ClassDef,
    registry: &CommandRegistry,
) -> Option<tcl_registry::definer::DefinerFamily> {
    registry
        .get(&class.metaclass)?
        .definition_body
        .map(|grammar| grammar.family)
}

/// Read `qname` as an [incr Tcl] class-scoped `proc` — itcl's equivalent of
/// `TclOO`'s `classmethod` — returning `(class qualified name, member name)`
/// when its parent namespace names an itcl class that declares that member.
///
/// itcl gives every class a real namespace of the same name and installs its
/// class-scoped `proc`s as ordinary commands inside it, so `::app::Factory`'s
/// `proc make` is genuinely the command `::app::Factory::make`.  Verified on
/// tclsh 8.6.14 with Itcl 3.4: `info commands ::Factory::*` lists it, and
/// `::app::Factory::make`, `app::Factory::make`, and a bare `Factory::make`
/// from inside `::app` all dispatch it.  An *instance* method is not
/// reachable this way — calling `::Factory::inst` errors `namespace "::" is
/// not a class namespace` — so only `class_methods` is consulted.
pub(crate) fn itcl_class_proc_at<'a>(
    analysis: &'a AnalysisResult,
    qname: &str,
) -> Option<(&'a String, String)> {
    itcl_class_proc_at_in_registry(analysis, class_family_registry(analysis)?, qname)
}

/// Standalone constructed metadata can supply its store explicitly. Contextual
/// callers use their analysis-owned Registry and never recapture a profile.
pub(super) fn itcl_class_proc_at_in_registry<'a>(
    analysis: &'a AnalysisResult,
    registry: &CommandRegistry,
    qname: &str,
) -> Option<(&'a String, String)> {
    let (parent, member) = tcl_syntax::naming::key_holder_and_tail(qname);
    if member.is_empty() {
        return None;
    }
    let (class_q, class_def) = analysis.all_classes.get_key_value(parent)?;
    (definer_family_in_registry(class_def, registry)
        == Some(tcl_registry::definer::DefinerFamily::Itcl)
        && class_def.class_methods.contains_key(member))
    .then(|| (class_q, member.to_owned()))
}

/// Resolve a written command word to the [incr Tcl] class-scoped `proc` it
/// dispatches.
///
/// Two resolvers, in the order the interpreter applies them:
///
/// 1. [`itcl_self_qualified_candidate`] — itcl installs a **custom command
///    resolver** on each class namespace, and it is consulted *before* the
///    ordinary namespace lookup, so inside a class's own namespace the class's
///    simple name qualifies its members ahead of every stock candidate.
/// 2. [`resolved_command_name`] — stock Tcl resolution (current namespace,
///    `namespace path`, then global) for every other call site, so a call is
///    attributed to the class the runtime would actually reach and an ordinary
///    proc or class command shadows it exactly as it would at runtime.
pub(crate) fn itcl_class_proc_target<'a>(
    analysis: &'a AnalysisResult,
    namespace: &str,
    word: &str,
) -> Option<(&'a String, String)> {
    let winner = itcl_self_qualified_candidate(analysis, namespace, word)
        // The custom resolver only claims a member the class really declares;
        // anything else falls through to the stock candidates (oracle: a
        // sibling class's `Other::omake`, and an undeclared `Factory::nope`,
        // both raise `invalid command name` rather than resolving).
        .filter(|candidate| itcl_class_proc_at(analysis, candidate).is_some())
        .or_else(|| {
            resolved_command_name(analysis, namespace, word, &|candidate| {
                analysis.all_procs.contains_key(candidate)
                    || analysis.all_classes.contains_key(candidate)
                    || itcl_class_proc_at(analysis, candidate).is_some()
            })
        })?;
    itcl_class_proc_at(analysis, &winner)
}

/// [incr Tcl]'s own class-namespace command resolver: written *inside* a
/// class's namespace — any method or class-`proc` body — the class's own
/// **simple** name qualifies its members, so `Factory::make` reaches
/// `::app::Factory::make` even though stock Tcl resolution would try only
/// `::app::Factory::Factory::make` and `::Factory::make`, neither of which
/// exists.
///
/// This resolver **pre-empts** the stock candidates rather than backing them
/// up: itcl installs a custom command resolver (`Itcl_ClassCmdResolver`) on
/// every class namespace, and Tcl consults a namespace's resolver *before* its
/// ordinary command lookup.  A real command at a stock candidate — even the
/// current-namespace-relative one — does **not** win.
///
/// Pinned against tclsh 8.6 + Itcl 3.4 (probes `itcl2.tcl`, `itcl2b.tcl`,
/// `itcl2c.tcl`), with a class `::app::Factory` declaring `proc make`:
///
/// | Written          | Called from            | Also defined                      | Dispatches |
/// |------------------|------------------------|-----------------------------------|------------|
/// | `Factory::make`  | the class's own bodies | `::Factory::make` (a plain proc)   | the **class proc** |
/// | `Factory::make`  | the class's own bodies | `::app::Factory::Factory::make`    | the **class proc** |
/// | `Factory::make`  | `::app`                | —                                 | the class proc, by stock relative lookup |
/// | `Factory::make`  | `::` or `::other`      | `::Factory::make`                 | the **plain proc** |
/// | `Other::omake`   | `::app::Factory`       | class `::app::Other` with `omake`  | nothing — `invalid command name` |
/// | `Factory::nope`  | `::app::Factory`       | —                                 | nothing — `invalid command name` |
///
/// So the rule covers the *enclosing* class only, and only for members it
/// declares; the ordinary spellings (`app::Factory::make`,
/// `::app::Factory::make`) keep resolving through
/// [`resolved_command_name`].
pub(super) fn itcl_self_qualified_candidate(
    analysis: &AnalysisResult,
    namespace: &str,
    word: &str,
) -> Option<String> {
    let class_def = analysis.all_classes.get(namespace)?;
    if class_definer_family(analysis, class_def) != Some(tcl_registry::definer::DefinerFamily::Itcl)
    {
        return None;
    }
    let member =
        tcl_syntax::naming::unroot_rooted_key(word.strip_prefix(class_def.name.as_str())?)?;
    (!member.is_empty() && !tcl_syntax::naming::is_qualified(member.as_bytes()))
        .then(|| tcl_syntax::naming::qualify(&class_def.qualified_name, member))
}

/// The declaration name span of the [incr Tcl] class-scoped `proc` that the
/// written word `word`, called from `namespace`, dispatches — go-to-definition's
/// half of [`itcl_class_proc_target`].
///
/// Family metadata comes from the analysis-owned command store, independently
/// of the written member prefix and the original declaration-name span.
pub(super) fn itcl_class_proc_declaration(
    analysis: &AnalysisResult,
    namespace: &str,
    word: &str,
) -> Option<tcl_lexer::Span> {
    let (class_q, member) = itcl_class_proc_target(analysis, namespace, word)?;
    analysis
        .all_classes
        .get(class_q)
        .and_then(|cd| cd.class_methods.get(&member))
        .map(|decl| decl.name_span)
}

/// [`itcl_class_proc_target`] for the word under the cursor: the
/// `(class qualified name, member name)` of the [incr Tcl] class-scoped
/// `proc` a `Factory::make` call site at `(line, character)` dispatches.
///
/// The one cursor entry point shared by go-to-definition, Find All
/// References, rename, and the cross-file rename seed, so none of them can
/// disagree about which class-proc a given call site names.  Returns `None`
/// off any word, and for every spelling that is not this dispatch shape.
#[must_use]
pub fn itcl_class_proc_target_at(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<(String, String)> {
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let cursor = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let word = crate::source_structure::original_cursor_word_value(source, analysis, cursor)?;
    let namespace = namespace_context_at(
        &analysis.global_scope,
        cursor,
        &analysis.namespace_overrides,
    );
    itcl_class_proc_target(analysis, &namespace, &word)
        .map(|(class_q, member)| (class_q.clone(), member))
}
