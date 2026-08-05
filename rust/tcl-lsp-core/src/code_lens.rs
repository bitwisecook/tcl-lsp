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

//! Code-lens provider.
//!
//! Surfaces a reference-count lens above every user-proc
//! definition: `N references` at the proc's name span,
//! showing how many call sites target it in the current
//! document.
//!
//! Provided lenses:
//!
//! * Per-proc lenses — `N references` per user proc.
//! * Class lenses — `N references` per `oo::class create`
//!   declaration, counting `ClassName new`, `ClassName create
//!   <inst>`, and inheritance references in
//!   `analysis.command_invocations`.
//!
//! Per-method / classmethod reference-count lenses: each member
//! declaration's name span gets a `N references` lens whose count comes
//! from [`crate::references::method_references_for_class`] — the same
//! resolver Find All References and rename use — so the lens and the peek
//! always agree.  That resolver counts intra-class `my method` dispatch,
//! external `$obj method` call sites (matched through the analyser's
//! `instance_classes` variable-type tracking), and the sites of any
//! subclass that inherits the definition (issue #864).  Each member lens
//! carries a `qname` ([`tcl_compiler::analyser::class_member_key`]) just
//! like the proc / class lenses, so it resolves to a clickable
//! `tcl-lsp.showReferences` command the same way (issue #956).
//!
//! Per-property reference-count lenses: each `property` declaration gets
//! the same treatment, sourced from
//! [`crate::references::property_references_for_class`] instead — a
//! class-local `my <property>` scan, since properties have no `$obj
//! property` dispatch shape and no inheritance model. Carries a
//! `{class}::property::{name}` qname
//! ([`tcl_compiler::analyser::class_property_key`]) so it resolves through
//! the same click-to-references flow (issue #992).
//!
//! Constructor / destructor next-chain lenses: a class's own explicit
//! `constructor` / `destructor` also gets a lens, but a conventional
//! dispatch-count has no general meaning for either (both are invoked
//! positionally — `ClassName new`/`create`/`destroy` — never by name). The
//! one name-independent relationship that *is* meaningful: an overriding
//! subclass's own constructor/destructor chaining up to this one via
//! `next` / `nextto`. Sourced from
//! [`crate::references::constructor_next_chain_references`] /
//! [`crate::references::destructor_next_chain_references`], which resolve
//! the chain through the full class hierarchy (via
//! [`tcl_compiler::analyser::class_hierarchy::ClassHierarchy::constructor_next_provider`]
//! / `destructor_next_provider`), not just the immediate superclass.
//! Carries a `{class}::constructor` / `{class}::destructor` qname
//! ([`tcl_compiler::analyser::class_constructor_key`] /
//! `class_destructor_key`) for the same click-to-references flow (issue
//! #992).
//!
//! Cross-document reference counts: when the
//! caller threads a [`crate::workspace_index::WorkspaceIndex`]
//! and the document's URI, the proc / class lens count
//! includes call sites in sibling documents.
//!
//! Scope of the counts this module computes:
//!
//! * Class-member counts (method / classmethod / property, and the
//!   constructor / destructor next-chain) are **current-document only**.
//!   Resolving a cross-file `$obj method` site needs each sibling
//!   document's own analysis, which this pure, single-document provider
//!   has no way to obtain.  Proc / class counts do fold in cross-document
//!   command-head call sites, since the workspace index carries those
//!   directly.
//! * That is not what the editor ends up showing for a member lens.  Every
//!   lens carrying a `qname` is returned to the client *without* a command,
//!   so the client must call `codeLens/resolve` before rendering it — and
//!   the server relabels it there from the workspace-wide site set it
//!   resolves for the click, via [`reference_count_title`].  The number
//!   shown and the locations the click opens are therefore one and the same
//!   set, and both match Find All References on the declaration (issue
//!   #991).  The counts here are the single-document floor under that, and
//!   what a caller with no server gets.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;

/// One code-lens entry — anchor range plus a command label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLens {
    /// Anchor range for the lens.
    pub range: LspRange,
    /// Command label shown to the user.
    pub command_title: String,
    /// Command identifier sent on click.
    pub command: String,
    /// Qualified name of the symbol the lens annotates (`::greet`),
    /// surfaced in the LSP lens `data` for the references-jump command.
    pub qname: String,
}

/// Compute code lenses for the document.
///
/// `analysis` is the analyser result for the document; when
/// `None`, returns an empty vector (preserves the stub call
/// shape for callers that haven't yet plumbed analysis
/// through).  `workspace` / `current_uri`, when provided, add
/// cross-document call sites to the proc / class reference
/// counts so the lens reflects workspace-wide usage.
#[must_use]
pub fn code_lenses(
    source: &str,
    dialect: &str,
    analysis: Option<&AnalysisResult>,
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
    current_uri: &str,
) -> Vec<CodeLens> {
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut lenses: Vec<CodeLens> = Vec::new();

    for (qname, proc_def) in &analysis.all_procs {
        // Derive the local count from the *same* matching the peek (Find All
        // References) uses, so the lens title and the peek can never drift.
        // The earlier name-only tally could disagree
        // with the namespace-aware resolver (e.g. a same-named proc in
        // another namespace).  `proc_reference_spans` takes the resolved
        // proc directly, so iterating every proc here doesn't rebuild a
        // `LineIndex` or rescan the proc table per definition.
        let mut count = crate::references::proc_reference_spans(analysis, qname, proc_def).len();
        if let Some(index) = workspace {
            count += index
                .invocations_of(&proc_def.qualified_name, current_uri)
                .len();
        }
        let title = reference_count_title(count);
        let start = line_index.position_at_utf16(proc_def.name_span.start(), source);
        let end = line_index.position_at_utf16(proc_def.name_span.end(), source);
        lenses.push(CodeLens {
            range: LspRange {
                start_line: start.line,
                start_character: start.character.get(),
                end_line: end.line,
                end_character: end.character.get(),
            },
            command_title: title,
            // Empty command — the lens is informational only.
            // Editors render the title as text; clicking is a
            // no-op until the references-jump command is wired
            // up.
            command: String::new(),
            qname: proc_def.qualified_name.clone(),
        });
    }

    // Class lenses.  Surface a reference-
    // count lens at each `oo::class create ClassName` site.
    // The count includes constructor calls (`ClassName new`,
    // `ClassName create instance`) and references in
    // inheritance chains (`oo::class create Sub { superclass
    // ClassName ... }`).
    for (qname, class_def) in &analysis.all_classes {
        let mut count = count_class_references(qname, class_def, analysis);
        if let Some(index) = workspace {
            count += index
                .invocations_of(&class_def.qualified_name, current_uri)
                .len();
        }
        let title = reference_count_title(count);
        let start = line_index.position_at_utf16(class_def.name_span.start(), source);
        let end = line_index.position_at_utf16(class_def.name_span.end(), source);
        lenses.push(CodeLens {
            range: LspRange {
                start_line: start.line,
                start_character: start.character.get(),
                end_line: end.line,
                end_character: end.character.get(),
            },
            command_title: title,
            command: String::new(),
            qname: class_def.qualified_name.clone(),
        });
        // Per-method / classmethod lenses inside the class body.  The count
        // is derived from the *same* resolver the peek (Find All References)
        // uses — `references::method_references_for_class` — so the lens
        // title and the peek can never drift (issue #864).  That resolver
        // counts both intra-class `my method` dispatch and external
        // `$obj method` call sites (via `instance_classes` plus the
        // object-type lattice's scoped facts — issue #994 C5b), plus the
        // call sites of any subclass that inherits this definition.
        emit_class_member_lenses(
            source,
            dialect,
            qname,
            class_def,
            analysis,
            &line_index,
            &mut lenses,
        );
    }

    lenses
}

/// Emit per-member reference-count lenses for every method, classmethod, and
/// property in `class_def`.  The count for a method / classmethod is the
/// number of call sites [`references::method_references_for_class`]
/// resolves; for a property it's [`references::property_references_for_class`]
/// instead (a class-local `my <property>` scan — properties have no `$obj
/// property` dispatch shape and no inheritance model).  Both are the same
/// single source of truth also used by Find All References and rename, so
/// the lens title and the peek can never drift.  The method/classmethod
/// resolver covers intra-class `my method` dispatch, external `$obj method`
/// sites (resolved through the analyser's `instance_classes` variable-type
/// tracking plus the object-type lattice's scope-keyed facts — issue #994
/// C5b), and the call sites of any subclass that inherits (does not
/// override) this definition.
///
/// Each lens carries a `{class}::method::{name}` / `{class}::classmethod::{name}`
/// / `{class}::property::{name}` `qname`
/// ([`tcl_compiler::analyser::class_member_key`] /
/// [`tcl_compiler::analyser::class_property_key`]) — the same
/// `qname`-present shape as the proc / class lenses — so the server's
/// `code_lens_resolve` treats it as resolvable and attaches the *clickable*
/// `tcl-lsp.showReferences` command instead of leaving it an inert bare
/// title (issue #956: the `#724` "reference is not active" defect,
/// previously fixed for proc/class lenses, recurring for methods; issue
/// #992 extends the same treatment to properties).
fn emit_class_member_lenses(
    source: &str,
    dialect: &str,
    class_q: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    lenses: &mut Vec<CodeLens>,
) {
    // Reference count for one member, matching the peek exactly: the number
    // of call sites (declaration excluded) the shared resolver returns.
    // `is_classmethod` is passed explicitly rather than inferred, so a
    // `method` and `classmethod` sharing a name each count and resolve only
    // their own dispatch shape (Codex review on #971, P2).
    let member_ref_count = |name: &str, is_classmethod: bool| -> usize {
        crate::references::method_references_for_class(
            source,
            dialect,
            analysis,
            class_q,
            name,
            is_classmethod,
        )
        .map_or(0, |(_decl, calls)| calls.len())
    };
    let push_lens =
        |name_span: tcl_lexer::Span, qname: String, title: String, lenses: &mut Vec<CodeLens>| {
            let start = line_index.position_at_utf16(name_span.start(), source);
            let end = line_index.position_at_utf16(name_span.end(), source);
            lenses.push(CodeLens {
                range: LspRange {
                    start_line: start.line,
                    start_character: start.character.get(),
                    end_line: end.line,
                    end_character: end.character.get(),
                },
                command_title: title,
                command: String::new(),
                qname,
            });
        };
    for (name, m) in &class_def.methods {
        if m.name_span.is_empty() {
            continue;
        }
        push_lens(
            m.name_span,
            tcl_compiler::analyser::class_member_key(class_q, name, false),
            reference_count_title(member_ref_count(name, false)),
            lenses,
        );
    }
    for (name, m) in &class_def.class_methods {
        if m.name_span.is_empty() {
            continue;
        }
        push_lens(
            m.name_span,
            tcl_compiler::analyser::class_member_key(class_q, name, true),
            reference_count_title(member_ref_count(name, true)),
            lenses,
        );
    }
    for (name, p) in &class_def.properties {
        if p.name_span.is_empty() {
            continue;
        }
        let count = crate::references::property_references_for_class(
            source, dialect, analysis, class_q, name,
        )
        .map_or(0, |(_decl, calls)| calls.len());
        push_lens(
            p.name_span,
            tcl_compiler::analyser::class_property_key(class_q, name),
            reference_count_title(count),
            lenses,
        );
    }
    // Constructor / destructor next-chain lenses (issue #992): unlike a
    // method/classmethod/property, neither has a name to dispatch on, so a
    // conventional reference count has no general meaning — the one
    // meaningful, name-independent relationship is an overriding subclass's
    // own constructor/destructor chaining up to this one via `next`/`nextto`.
    // Only a class that declares its own effective constructor/destructor
    // gets one of these lenses, same "declared members only" rule as every
    // other lens kind.
    if let Some(ctor) = class_def.constructors.last()
        && !ctor.name_span.is_empty()
    {
        let count = crate::references::constructor_next_chain_references(
            source, dialect, analysis, class_q,
        )
        .map_or(0, |(_decl, calls)| calls.len());
        push_lens(
            ctor.name_span,
            tcl_compiler::analyser::class_constructor_key(class_q),
            reference_count_title(count),
            lenses,
        );
    }
    if let Some(dtor) = &class_def.destructor
        && !dtor.name_span.is_empty()
    {
        let count =
            crate::references::destructor_next_chain_references(source, dialect, analysis, class_q)
                .map_or(0, |(_decl, calls)| calls.len());
        push_lens(
            dtor.name_span,
            tcl_compiler::analyser::class_destructor_key(class_q),
            reference_count_title(count),
            lenses,
        );
    }
}

/// The lens label for `count` call sites — `"0 references"` /
/// `"1 reference"` / `"N references"`.
///
/// Public so the server can relabel a lens at `codeLens/resolve` time from
/// the workspace-wide site set it resolves there (issue #991): the count
/// shown and the locations the click opens are then one number by
/// construction, not two independently-derived ones.
#[must_use]
pub fn reference_count_title(count: usize) -> String {
    match count {
        0 => "0 references".to_string(),
        1 => "1 reference".to_string(),
        n => format!("{n} references"),
    }
}

/// Whether `qname` names a symbol [`code_lenses`] would emit a lens for in
/// this document — a proc, a class, or one of a class's members (method /
/// classmethod / property / constructor / destructor), matching exactly the
/// same "declared members only" guards `code_lenses` /
/// `emit_class_member_lenses` apply (a member with an empty `name_span`
/// gets no lens, so it must not answer `true` here either).
///
/// `codeLens/resolve` (issue #1152) used this existence check to decide
/// whether a lens is genuinely one of this document's own lenses before
/// resolving its click locations — previously by calling [`code_lenses`] in
/// full and searching its output for a matching `qname`, which recomputed
/// every *other* proc's / class's / member's reference count (each itself a
/// resolver walk) just to answer one boolean. This answers the same
/// question with direct `HashMap`/`Vec` lookups: O(1) for a plain proc or
/// class, O(the document's own class count) for a member qname (bounded by
/// how many classes share its `{class}::…` prefix, never by proc count or
/// by any reference-resolution work).
#[must_use]
pub fn lens_qname_exists(analysis: &AnalysisResult, qname: &str) -> bool {
    if analysis.all_procs.contains_key(qname) || analysis.all_classes.contains_key(qname) {
        return true;
    }
    for (class_q, class_def) in &analysis.all_classes {
        let Some(rest) = qname.strip_prefix(class_q.as_str()) else {
            continue;
        };
        // A shorter `class_q` can still produce a `rest` that happens to
        // start with one of these markers (e.g. a class literally named
        // `method` nested under this one) — only trust a *hit*, and keep
        // checking the other classes on a miss rather than trusting a wrong
        // split's "not found" as the final answer.
        let found = if let Some(name) = rest.strip_prefix("::method::") {
            class_def
                .methods
                .get(name)
                .is_some_and(|m| !m.name_span.is_empty())
        } else if let Some(name) = rest.strip_prefix("::classmethod::") {
            class_def
                .class_methods
                .get(name)
                .is_some_and(|m| !m.name_span.is_empty())
        } else if let Some(name) = rest.strip_prefix("::property::") {
            class_def
                .properties
                .get(name)
                .is_some_and(|p| !p.name_span.is_empty())
        } else if rest == "::constructor" {
            class_def
                .constructors
                .last()
                .is_some_and(|c| !c.name_span.is_empty())
        } else if rest == "::destructor" {
            class_def
                .destructor
                .as_ref()
                .is_some_and(|d| !d.name_span.is_empty())
        } else {
            false
        };
        if found {
            return true;
        }
    }
    false
}

/// Count invocations targeting the given class — `ClassName
/// new ...`, `ClassName create instance`, and inheritance
/// references in `oo::class create Sub { superclass ClassName
/// ... }`.  Excludes the class's own `oo::class create
/// ClassName` declaration site.
fn count_class_references(
    qname: &str,
    class_def: &tcl_compiler::analyser::ClassDef,
    analysis: &AnalysisResult,
) -> usize {
    // Derive the count from the *same* namespace-aware matching the peek
    // (Find All References) uses — `references::class_reference_spans` — so
    // the lens title and the peek can never drift (mirrors the proc lens's
    // `proc_reference_spans` reuse above).
    crate::references::class_reference_spans(analysis, qname, class_def)
        .into_iter()
        .filter(|span| {
            !(span.start() <= class_def.name_span.start()
                && class_def.name_span.end() <= span.end())
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn empty_lenses_when_analysis_is_none() {
        assert!(code_lenses("proc foo {} {}\n", "tcl", None, None, "").is_empty());
    }

    #[test]
    fn lens_per_user_proc() {
        let src = "proc foo {} {}\nproc bar {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        assert_eq!(lenses.len(), 2, "{lenses:?}");
    }

    #[test]
    fn lens_shows_zero_references_for_unused_proc() {
        let src = "proc lonely {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].command_title, "0 references");
    }

    #[test]
    fn lens_shows_singular_for_one_reference() {
        let src = "proc helper {} {}\nhelper\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let helper = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("helper lens");
        assert_eq!(helper.command_title, "1 reference");
    }

    #[test]
    fn lens_counts_multiple_references() {
        let src = "proc tool {} {}\ntool\ntool\ntool\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let tool = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("tool lens");
        assert_eq!(tool.command_title, "3 references");
    }

    #[test]
    fn lens_anchors_at_proc_name_span() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        assert_eq!(lenses.len(), 1);
        // `greet` starts at column 5 (after `proc `).
        assert_eq!(lenses[0].range.start_character, 5);
    }

    #[test]
    fn method_lens_counts_a_method_return_captured_dispatch_site() {
        // Issue #994 C5b: the lens count comes from the same resolver Find
        // References uses, so a `$b greet` site typed only by the lattice's
        // method-return edge must be counted too.
        let src = "oo::class create A { method make {} { return [B new] } }\n\
                   oo::class create B { method greet {} { return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let greet = lenses
            .iter()
            .find(|l| l.qname.contains("::method::greet"))
            .expect("greet method lens");
        assert_eq!(
            greet.command_title, "1 reference",
            "the lattice-typed `$b greet` site must be counted: {lenses:?}"
        );
    }

    // class lenses

    #[test]
    fn lens_per_class() {
        let src = "oo::class create Greeter {}\noo::class create Helper {}\n";
        let analysis = analyse(src);
        // Skip if the analyser doesn't track classes for this
        // tcl version / shape — the assertion below should still
        // succeed when the analyser does record them.
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let class_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| l.command_title.contains("reference"))
            .collect();
        assert!(class_lenses.len() >= 2, "{lenses:?}");
    }

    #[test]
    fn lens_counts_class_constructor_calls() {
        // `MyClass new` and `MyClass create instance` are
        // constructor calls — both contribute to the reference
        // count.
        let src = concat!(
            "oo::class create MyClass {}\n",
            "MyClass new\n",
            "MyClass create instance\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let myclass_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("class lens at line 0");
        // Two constructor calls.
        assert_eq!(
            myclass_lens.command_title, "2 references",
            "got {:?}",
            myclass_lens.command_title,
        );
    }

    // class-member lenses

    #[test]
    fn lens_counts_method_calls_within_class_body() {
        // Intra-class self-dispatch in TclOO is `my greet`, not a bare
        // `greet` (a bare word resolves as a normal command, never the
        // object's method).  `greet` is dispatched twice from `twice`.
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        // Find the lens anchored on line 1 (the `greet`
        // declaration's name span).
        let greet_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("greet lens");
        assert_eq!(greet_lens.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn lens_bare_word_in_method_body_is_not_a_self_dispatch() {
        // FP guard: a bare `greet` inside a sibling method body is a normal
        // command call, NOT TclOO self-dispatch (that is `my greet`), so it
        // must not be counted as a reference to the method.  The old
        // head-token heuristic wrongly counted these.
        let src = "oo::class create C {\n    method greet {} {}\n    method other {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let greet_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("greet lens");
        assert_eq!(greet_lens.command_title, "0 references", "{lenses:?}");
    }

    #[test]
    fn lens_reports_zero_for_uncalled_method() {
        let src = "oo::class create C {\n    method orphan {} {}\n}\n";
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let orphan_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("orphan lens");
        assert_eq!(orphan_lens.command_title, "0 references", "{lenses:?}");
    }

    // qname wiring (issue #956): a method / classmethod lens must carry a
    // non-empty `qname` matching `tcl_compiler::analyser::class_member_key`
    // so the server treats it as resolvable (`has_qname` in
    // `tcl-lsp-server`'s `code_lens` handler) and attaches a clickable
    // `tcl-lsp.showReferences` command via `codeLens/resolve`, instead of
    // leaving it an inert bare title (the `#724` defect recurring for
    // methods).  The wire-level resolve round-trip itself is covered by
    // `tcl-lsp-server`'s `code_lens_resolve_wires_show_references_command_for_method`
    // / `..._for_classmethod` tests; these lock in the `qname` this crate
    // hands the server.

    #[test]
    fn method_lens_carries_class_member_key_qname() {
        // FN→TP regression for the exact issue #956 shape: a `variable` and
        // `constructor` declared before the `method` in the class body.
        let src = "oo::class create Bar {\n   variable _options\n    constructor {args} {\n         set _options $args\n    }\n\n    method get {key} {\n        return [dict get $_options $key]\n    }\n\n}\nset b [Bar new]\nputs [$b get foo]\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let get_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 6)
            .expect("get lens");
        assert_eq!(
            get_lens.qname,
            tcl_compiler::analyser::class_member_key("::Bar", "get", false),
            "{lenses:?}"
        );
        assert!(!get_lens.qname.is_empty(), "empty qname stays inert");
    }

    #[test]
    fn classmethod_lens_carries_class_member_key_qname() {
        let src =
            "oo::class create Factory {\n    classmethod make {} { return [Factory new] }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let make_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("make lens");
        assert_eq!(
            make_lens.qname,
            tcl_compiler::analyser::class_member_key("::Factory", "make", true),
            "{lenses:?}"
        );
    }

    #[test]
    fn method_and_classmethod_qnames_never_collide_on_same_name() {
        // FP guard: a class with both a `method foo` and a `classmethod foo`
        // must get two distinct, disambiguated qnames — a naive
        // `{class}::{name}` key would collide and let `codeLens/resolve`
        // attach one member's locations to the other's lens.
        let src = "oo::class create C {\n    method foo {} {}\n    classmethod foo {} {}\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let qnames: std::collections::HashSet<&str> = lenses
            .iter()
            .filter(|l| l.range.start_line == 1 || l.range.start_line == 2)
            .map(|l| l.qname.as_str())
            .collect();
        assert_eq!(qnames.len(), 2, "{lenses:?}");
        assert!(qnames.contains("::C::method::foo"), "{qnames:?}");
        assert!(qnames.contains("::C::classmethod::foo"), "{qnames:?}");
    }

    // property-member lenses (issue #992)

    /// `property` is Tcl 9.0+ — the shared `analyse()` helper above fixes the
    /// dialect at 8.6, so these tests analyse at 9.0 directly (mirrors
    /// `rename.rs`'s property-rename tests).
    fn analyse90(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl9.0").clone()
    }

    #[test]
    fn lens_counts_property_my_dispatch_within_class_body() {
        // A property's auto-generated accessor dispatches through `my
        // <property>`, exactly like a method — `bump` reads/writes `size`
        // twice, so its lens must read "2 references" the same way a
        // twice-dispatched method would (`lens_counts_method_calls_within_class_body`).
        let src = "oo::class create Widget {\n    property size -get {return $mySize} -set {set mySize $value}\n    method bump {} { my size ; my size }\n}\n";
        let analysis = analyse90(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl9.0", Some(&analysis), None, "");
        let size_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("size lens");
        assert_eq!(size_lens.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn lens_reports_zero_for_unused_property() {
        let src = "oo::class create Widget {\n    property size\n}\n";
        let analysis = analyse90(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl9.0", Some(&analysis), None, "");
        let size_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("size lens");
        assert_eq!(size_lens.command_title, "0 references", "{lenses:?}");
    }

    #[test]
    fn property_lens_carries_class_property_key_qname() {
        let src = "oo::class create Widget {\n    property size\n}\n";
        let analysis = analyse90(src);
        let lenses = code_lenses(src, "tcl9.0", Some(&analysis), None, "");
        let size_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("size lens");
        assert_eq!(
            size_lens.qname,
            tcl_compiler::analyser::class_property_key("::Widget", "size"),
            "{lenses:?}"
        );
        assert!(!size_lens.qname.is_empty(), "empty qname stays inert");
    }

    #[test]
    fn property_qname_never_collides_with_a_same_named_method() {
        // FP guard mirroring `method_and_classmethod_qnames_never_collide_on_same_name`:
        // methods and properties are independent tables, so a `property
        // color` and a `method color` on the same class must still get two
        // distinct, disambiguated lenses.
        let src = "oo::class create C {\n    property color\n    method color {} {}\n}\n";
        let analysis = analyse90(src);
        let lenses = code_lenses(src, "tcl9.0", Some(&analysis), None, "");
        let qnames: std::collections::HashSet<&str> = lenses
            .iter()
            .filter(|l| l.range.start_line == 1 || l.range.start_line == 2)
            .map(|l| l.qname.as_str())
            .collect();
        assert_eq!(qnames.len(), 2, "{lenses:?}");
        assert!(qnames.contains("::C::property::color"), "{qnames:?}");
        assert!(qnames.contains("::C::method::color"), "{qnames:?}");
    }

    #[test]
    fn lens_counts_each_name_independently_in_a_multi_name_property_declaration() {
        // `property x y` declares two independent properties from one command
        // (verified structurally by `extract_property_defs`'s own
        // `property_subcommand_records_multiple_names` test) — each name must
        // get its own lens with its own independently-counted `my <name>`
        // dispatch sites, not a shared or merged count.
        let src = "oo::class create Widget {\n    property x y\n    method bump {} { my x ; my y ; my y }\n}\n";
        let analysis = analyse90(src);
        let lenses = code_lenses(src, "tcl9.0", Some(&analysis), None, "");
        let x_lens = lenses
            .iter()
            .find(|l| l.qname == "::Widget::property::x")
            .expect("x lens");
        let y_lens = lenses
            .iter()
            .find(|l| l.qname == "::Widget::property::y")
            .expect("y lens");
        assert_eq!(x_lens.command_title, "1 reference", "{lenses:?}");
        assert_eq!(y_lens.command_title, "2 references", "{lenses:?}");
    }

    // constructor / destructor next-chain lenses (issue #992)

    #[test]
    fn constructor_lens_counts_subclass_next_chain() {
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let base_ctor_lens = lenses
            .iter()
            .find(|l| l.qname == "::Base::constructor")
            .expect("Base constructor lens");
        assert_eq!(base_ctor_lens.command_title, "1 reference", "{lenses:?}");
        // `Sub` also declares its own constructor, so it gets a lens too —
        // nothing chains *into* Sub's, so it reads zero.
        let sub_ctor_lens = lenses
            .iter()
            .find(|l| l.qname == "::Sub::constructor")
            .expect("Sub constructor lens");
        assert_eq!(sub_ctor_lens.command_title, "0 references", "{lenses:?}");
    }

    #[test]
    fn destructor_lens_counts_subclass_next_chain() {
        let src = "oo::class create Base {\n    destructor { }\n}\noo::class create Sub {\n    superclass Base\n    destructor { next }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let base_dtor_lens = lenses
            .iter()
            .find(|l| l.qname == "::Base::destructor")
            .expect("Base destructor lens");
        assert_eq!(base_dtor_lens.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn constructor_lens_reports_zero_when_no_subclass_chains() {
        let src = "oo::class create Base {\n    constructor {} { }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let ctor_lens = lenses
            .iter()
            .find(|l| l.qname == "::Base::constructor")
            .expect("constructor lens");
        assert_eq!(ctor_lens.command_title, "0 references", "{lenses:?}");
    }

    #[test]
    fn class_with_no_explicit_constructor_or_destructor_gets_neither_lens() {
        let src = "oo::class create Plain {\n    method use {} {}\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        assert!(
            lenses
                .iter()
                .all(|l| !l.qname.ends_with("::constructor") && !l.qname.ends_with("::destructor")),
            "{lenses:?}"
        );
    }

    #[test]
    fn constructor_lens_carries_class_constructor_key_qname() {
        let src = "oo::class create Bar {\n    constructor {} { }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let ctor_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("constructor lens");
        assert_eq!(
            ctor_lens.qname,
            tcl_compiler::analyser::class_constructor_key("::Bar"),
            "{lenses:?}"
        );
    }

    // workspace-index: cross-document reference counts

    #[test]
    fn proc_lens_counts_cross_document_calls() {
        use crate::workspace_index::WorkspaceIndex;
        // lib.tcl defines `helper` with no local callers;
        // consumer.tcl calls it twice.
        let lib_src = "proc helper {} {}\n";
        let lib = analyse(lib_src);
        let consumer = analyse("helper\nhelper\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///lib.tcl", &lib),
            ("file:///consumer.tcl", &consumer),
        ]);
        // The lens on lib.tcl's `helper` counts the two
        // cross-document calls.
        let lenses = code_lenses(lib_src, "tcl", Some(&lib), Some(&index), "file:///lib.tcl");
        let helper = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("helper lens");
        assert_eq!(helper.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn proc_lens_without_workspace_counts_local_only() {
        let src = "proc helper {} {}\nhelper\n";
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let helper = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("helper lens");
        assert_eq!(helper.command_title, "1 reference", "{lenses:?}");
    }

    // -- lens count must equal the reference list -----

    /// Parse a `"N reference(s)"` title back into its integer count.
    fn title_count(title: &str) -> usize {
        title
            .split_once(' ')
            .and_then(|(n, _)| n.parse().ok())
            .unwrap_or_else(|| panic!("unparseable lens title: {title:?}"))
    }

    /// The lens count for the proc at `name_line` must equal the number of
    /// call sites `references` (the peek / Find All References) resolves
    /// with `include_declaration = false`.
    fn assert_lens_matches_references(src: &str, name_line: u32) {
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let lens = lenses
            .iter()
            .find(|l| l.range.start_line == name_line)
            .unwrap_or_else(|| panic!("no lens on line {name_line}: {lenses:?}"));
        let refs = crate::references::references(
            src,
            "tcl8.6",
            lens.range.start_line,
            lens.range.start_character,
            &analysis,
            false,
        );
        assert_eq!(
            title_count(&lens.command_title),
            refs.len(),
            "lens {:?} disagrees with references {:?} for source {src:?}",
            lens.command_title,
            refs,
        );
    }

    #[test]
    fn lens_count_matches_references_forward_ref() {
        // Headline symptom: a call *before* the definition. The
        // name-only resolved-name tally reported 0 here; the lens must
        // agree with the resolver, which finds the forward call.
        let src = "foo\nproc foo {} {}\n";
        assert_lens_matches_references(src, 1);
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let foo = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("foo lens");
        assert_eq!(foo.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_count_matches_references_after_def() {
        assert_lens_matches_references("proc foo {} {}\nfoo\n", 0);
    }

    #[test]
    fn lens_count_matches_references_qualified_call() {
        assert_lens_matches_references("proc foo {} {}\n::foo\n", 0);
    }

    #[test]
    fn lens_count_matches_references_ns_qualified_call() {
        assert_lens_matches_references("namespace eval ns { proc foo {} {} }\nns::foo\n", 0);
    }

    #[test]
    fn lens_count_matches_references_call_inside_body() {
        assert_lens_matches_references("proc foo {} {}\nproc bar {} { foo }\n", 0);
    }

    #[test]
    fn lens_count_matches_references_cmd_substitution() {
        assert_lens_matches_references("proc foo {} {}\nset x [foo]\n", 0);
    }

    #[test]
    fn lens_count_matches_references_unused_proc() {
        assert_lens_matches_references("proc foo {} {}\n", 0);
    }

    #[test]
    fn lens_count_matches_references_same_name_other_namespace() {
        // The namespace-aware resolver must not count a same-named proc
        // call from a different namespace; the lens count follows it so the
        // two can never drift (the precise divergence this guards against).
        let src = concat!(
            "namespace eval a { proc helper {} {} }\n",
            "namespace eval b { proc helper {} {} ; helper }\n",
        );
        assert_lens_matches_references(src, 0); // a::helper
        assert_lens_matches_references(src, 1); // b::helper
    }

    #[test]
    fn lens_does_not_credit_ambiguous_forward_ref_to_other_namespace() {
        // A forward `foo` call inside namespace
        // `a`, with both `::a::foo` and `::b::foo` defined, must be credited
        // only to `::a::foo`.  A tail-name resolver would attribute the
        // unresolved call to *both* procs, falsely showing `1 reference`
        // above `::b::foo`.  The namespace-aware resolver the lens count is
        // derived from scopes the call to its own namespace.
        let src = concat!(
            "namespace eval a { foo ; proc foo {} {} }\n",
            "namespace eval b { proc foo {} {} }\n",
        );
        let analysis = analyse(src);
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let by_qname = |q: &str| {
            lenses
                .iter()
                .find(|l| l.qname == q)
                .unwrap_or_else(|| panic!("no lens for {q}: {lenses:?}"))
        };
        assert_eq!(
            by_qname("::a::foo").command_title,
            "1 reference",
            "{lenses:?}"
        );
        assert_eq!(
            by_qname("::b::foo").command_title,
            "0 references",
            "{lenses:?}"
        );
    }

    // -- issue #864: method lens must count external `$obj method` sites --
    //
    // TP  — a real reference (`$obj method`, `my method`) is counted.
    // FP  — a non-reference (`dict get`, a bare word) is *not* counted.
    // TN  — a method with genuinely no references reads `0 references`.
    // FN  — the regression: the external `$obj method` call the old
    //       body-head heuristic missed is now counted.

    /// The exact source from issue #864.
    const ISSUE_864_SRC: &str = concat!(
        "oo::class create Bar {\n",
        "   variable _options\n",
        "    constructor {args} {\n",
        "         set _options $args\n",
        "    }\n",
        "\n",
        "    method get {key} {\n",
        "        return [dict get $_options $key]\n",
        "    }\n",
        "\n",
        "}\n",
        "set b [Bar new]\n",
        "puts [$b get foo]\n",
    );

    #[test]
    fn lens_counts_external_obj_method_call_issue_864() {
        // FN-regression + TP: `puts [$b get foo]` references `Bar`'s `get`
        // via an instance whose class is known (`set b [Bar new]`).  The
        // lens on `method get` (line 6) must read `1 reference`, not the
        // `0 references` the head-scan heuristic produced.
        let analysis = analyse(ISSUE_864_SRC);
        assert!(!analysis.all_classes.is_empty(), "class not analysed");
        let lenses = code_lenses(ISSUE_864_SRC, "tcl", Some(&analysis), None, "");
        let get_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 6)
            .expect("method get lens on line 6");
        assert_eq!(get_lens.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_matches_peek_for_issue_864_method() {
        // The lens count and Find All References must agree exactly.
        assert_lens_matches_references(ISSUE_864_SRC, 6);
    }

    #[test]
    fn lens_does_not_count_dict_get_as_method_ref() {
        // FP guard: the method is named `get` and its own body contains
        // `dict get $_options $key`.  The `get` word there is the `dict`
        // ensemble subcommand, not a dispatch of the method, so it must not
        // inflate the count.  With only the external `$b get foo`, the count
        // is exactly 1.
        let analysis = analyse(ISSUE_864_SRC);
        let lenses = code_lenses(ISSUE_864_SRC, "tcl", Some(&analysis), None, "");
        let get_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 6)
            .expect("method get lens");
        assert_eq!(get_lens.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_counts_multiple_external_obj_method_calls() {
        // TP: two distinct instances each dispatch `bark` once.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} {}\n",
            "}\n",
            "set a [Dog new]\n",
            "set b [Dog new]\n",
            "$a bark\n",
            "puts [$b bark]\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let bark = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("bark lens");
        assert_eq!(bark.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn lens_counts_obj_method_from_create_instance() {
        // TP: `Dog create rex` binds `rex` as a Dog instance command, so
        // `rex bark` is a method dispatch.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} {}\n",
            "}\n",
            "Dog create rex\n",
            "rex bark\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let bark = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("bark lens");
        assert_eq!(bark.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_counts_inherited_method_on_subclass_instance() {
        // TP + inheritance: `Base`'s `speak` is inherited by `Derived`; a
        // `Derived` instance dispatching `speak` resolves to `Base`'s copy,
        // so it counts toward `Base::speak`'s lens.
        let src = concat!(
            "oo::class create Base {\n",
            "    method speak {} {}\n",
            "}\n",
            "oo::class create Derived {\n",
            "    superclass Base\n",
            "}\n",
            "set d [Derived new]\n",
            "$d speak\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let speak = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("speak lens");
        assert_eq!(speak.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_matches_peek_for_inherited_method_with_subclass_my_dispatch() {
        // Codex #881: the lens uses `method_references_for_class`, which counts
        // a `my speak` call in an *inheriting* subclass body; the peek from the
        // declaration must count it too (it now shares that resolver), so lens
        // and peek can't drift.  Here `Base::speak` is referenced by `Derived`'s
        // `my speak` and by `$d speak` — two sites.
        let src = concat!(
            "oo::class create Base {\n",
            "    method speak {} {}\n",
            "}\n",
            "oo::class create Derived {\n",
            "    superclass Base\n",
            "    method twice {} { my speak }\n",
            "}\n",
            "set d [Derived new]\n",
            "$d speak\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let speak = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("speak lens");
        assert_eq!(speak.command_title, "2 references", "{lenses:?}");
        // Lens and peek (from the `Base::speak` declaration) must agree.
        assert_lens_matches_references(src, 1);
    }

    #[test]
    fn lens_counts_namespaced_class_method() {
        // TP: a class defined inside a namespace, instance created and its
        // method dispatched — the qualified-name resolution must hold up.
        let src = concat!(
            "namespace eval app {\n",
            "    oo::class create Widget {\n",
            "        method render {} {}\n",
            "    }\n",
            "}\n",
            "set w [app::Widget new]\n",
            "$w render\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        // `method render` sits on line 2.
        let render = lenses
            .iter()
            .find(|l| l.range.start_line == 2)
            .expect("render lens");
        assert_eq!(render.command_title, "1 reference", "{lenses:?}");
        assert_lens_matches_references(src, 2);
    }

    #[test]
    fn lens_counts_obj_method_call_inside_proc_body() {
        // TP: the dispatch lives inside a proc body, which the top-level
        // segment scan skips — the resolver descends proc bodies explicitly.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} {}\n",
            "}\n",
            "set d [Dog new]\n",
            "proc run {} {\n",
            "    global d\n",
            "    $d bark\n",
            "}\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let bark = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("bark lens");
        assert_eq!(bark.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_unknown_instance_method_is_not_counted() {
        // TN/FP guard: `$x bark` where `x` has no recorded class is not a
        // resolvable dispatch, so it must not count toward `Dog::bark`.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} {}\n",
            "}\n",
            "$x bark\n",
        );
        let analysis = analyse(src);
        if analysis.all_classes.is_empty() {
            return;
        }
        let lenses = code_lenses(src, "tcl", Some(&analysis), None, "");
        let bark = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("bark lens");
        assert_eq!(bark.command_title, "0 references", "{lenses:?}");
    }

    #[test]
    fn lens_classmethod_count_matches_peek() {
        // A class-side method (`classmethod`) gets a lens, and its count must
        // equal the peek exactly.  External class-side dispatch (`Factory
        // build`) is not an instance-receiver form, so the count is 0 here —
        // and the peek agrees, which is the invariant under test.
        let src = concat!(
            "oo::class create Factory {\n",
            "    classmethod build {} {}\n",
            "}\n",
            "Factory build\n",
        );
        let analysis = analyse(src);
        // Locate the classmethod lens by name span; skip if the analyser
        // build for this dialect didn't record the class-side method.
        let build_span = match analysis
            .all_classes
            .values()
            .find_map(|c| c.class_methods.get("build"))
        {
            Some(m) if !m.name_span.is_empty() => m.name_span,
            _ => return,
        };
        let li = LineIndex::new(src);
        let name_line = li.position_at_utf16(build_span.start(), src).line;
        assert_lens_matches_references(src, name_line);
    }

    // lens_qname_exists (issue #1152: codeLens/resolve's targeted existence
    // check, replacing a full `code_lenses` walk)

    #[test]
    fn lens_qname_exists_finds_a_plain_proc() {
        let analysis = analyse("proc foo {} {}\n");
        assert!(lens_qname_exists(&analysis, "::foo"));
        assert!(!lens_qname_exists(&analysis, "::bar"));
    }

    #[test]
    fn lens_qname_exists_finds_a_plain_class() {
        let analysis = analyse("oo::class create Widget {}\n");
        if analysis.all_classes.is_empty() {
            return;
        }
        assert!(lens_qname_exists(&analysis, "::Widget"));
        assert!(!lens_qname_exists(&analysis, "::Other"));
    }

    #[test]
    fn lens_qname_exists_finds_a_method_and_classmethod_and_disambiguates() {
        let src = "oo::class create C {\n    method foo {} {}\n    classmethod foo {} {}\n}\n";
        let analysis = analyse(src);
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_member_key("::C", "foo", false)
        ));
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_member_key("::C", "foo", true)
        ));
        // A name neither table declares must not exist.
        assert!(!lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_member_key("::C", "missing", false)
        ));
    }

    #[test]
    fn lens_qname_exists_finds_a_property() {
        let src = "oo::class create Widget {\n    property size\n}\n";
        let analysis = analyse90(src);
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_property_key("::Widget", "size")
        ));
        assert!(!lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_property_key("::Widget", "missing")
        ));
    }

    #[test]
    fn lens_qname_exists_finds_constructor_and_destructor() {
        let src = "oo::class create Bar {\n    constructor {} {}\n    destructor {}\n}\n";
        let analysis = analyse(src);
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_constructor_key("::Bar")
        ));
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_destructor_key("::Bar")
        ));
        // A class with no explicit constructor/destructor gets neither.
        let plain = analyse("oo::class create Plain {\n    method use {} {}\n}\n");
        assert!(!lens_qname_exists(
            &plain,
            &tcl_compiler::analyser::class_constructor_key("::Plain")
        ));
        assert!(!lens_qname_exists(
            &plain,
            &tcl_compiler::analyser::class_destructor_key("::Plain")
        ));
    }

    #[test]
    fn lens_qname_exists_rejects_an_unrelated_qname() {
        let src = "proc foo {} {}\noo::class create Bar {\n    method m {} {}\n}\n";
        let analysis = analyse(src);
        assert!(!lens_qname_exists(&analysis, "::nope"));
        assert!(!lens_qname_exists(&analysis, "::Bar::method::nope"));
        assert!(!lens_qname_exists(&analysis, "::NoSuchClass::method::m"));
    }

    #[test]
    fn lens_qname_exists_agrees_with_code_lenses_output() {
        // Structural parity: every qname `code_lenses` actually emits a lens
        // for must be found by the targeted check, and nothing else is.
        let src = concat!(
            "proc helper {} {}\n",
            "oo::class create Bar {\n",
            "    variable _x\n",
            "    constructor {} { set _x 0 }\n",
            "    destructor {}\n",
            "    method get {} { return $_x }\n",
            "    classmethod make {} { return [Bar new] }\n",
            "    property colour\n",
            "}\n",
            "helper\n",
            "set b [Bar new]\n",
            "$b get\n",
        );
        let analysis = analyse90(src);
        let lenses = code_lenses(src, "tcl9.0", Some(&analysis), None, "");
        assert!(!lenses.is_empty(), "sanity: some lenses expected");
        for lens in &lenses {
            assert!(
                lens_qname_exists(&analysis, &lens.qname),
                "code_lenses emitted a qname the targeted check rejects: {}",
                lens.qname,
            );
        }
    }
}
