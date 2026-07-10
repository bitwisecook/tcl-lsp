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
//! subclass that inherits the definition (issue #864).
//!
//! Cross-document reference counts: when the
//! caller threads a [`crate::workspace_index::WorkspaceIndex`]
//! and the document's URI, the proc / class lens count
//! includes call sites in sibling documents.
//!
//! Limitations:
//!
//! * Method lens counts are current-document only (matching the method
//!   peek, which is also single-document): an `$obj method` site in a
//!   *sibling* document is not counted, because resolving a cross-file
//!   object's class needs whole-workspace instance-type flow the index
//!   does not yet carry.  Proc / class lenses do fold in cross-document
//!   command-head call sites via the workspace index.
//! * The lens carries a static label rather than an inline
//!   "show references" jump-out; the editor's built-in
//!   references command can be invoked from the lens itself.

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
                .invocations_of(&proc_def.name, &proc_def.qualified_name, current_uri)
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
                .invocations_of(&class_def.name, &class_def.qualified_name, current_uri)
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
        // `$obj method` call sites (via `instance_classes` type tracking),
        // plus the call sites of any subclass that inherits this definition.
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

/// Emit per-member reference-count lenses for every method and classmethod
/// in `class_def`.  The count for each member is the number of call sites
/// [`references::method_references_for_class`] resolves — the single source
/// of truth also used by Find All References and rename — so the lens title
/// and the peek can never drift.  That resolver covers intra-class
/// `my method` dispatch, external `$obj method` sites (resolved through the
/// analyser's `instance_classes` variable-type tracking), and the call sites
/// of any subclass that inherits (does not override) this definition.
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
    let member_ref_count = |name: &str| -> usize {
        crate::references::method_references_for_class(source, dialect, analysis, class_q, name)
            .map_or(0, |(_decl, calls)| calls.len())
    };
    let push_lens = |name_span: tcl_lexer::Span, title: String, lenses: &mut Vec<CodeLens>| {
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
            // Method lenses are informational; the references-jump `data`
            // is populated for proc / class lenses (where it's exercised).
            qname: String::new(),
        });
    };
    for m in class_def
        .methods
        .values()
        .chain(class_def.class_methods.values())
    {
        if m.name_span.is_empty() {
            continue;
        }
        push_lens(
            m.name_span,
            reference_count_title(member_ref_count(&m.name)),
            lenses,
        );
    }
}

fn reference_count_title(count: usize) -> String {
    match count {
        0 => "0 references".to_string(),
        1 => "1 reference".to_string(),
        n => format!("{n} references"),
    }
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
    let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname);
    analysis
        .command_invocations
        .iter()
        .filter(|inv| {
            inv.name == class_def.name
                || inv.name == class_def.qualified_name
                || inv.name == qname_no_prefix
        })
        .filter(|inv| {
            !(inv.range.start() <= class_def.name_span.start()
                && class_def.name_span.end() <= inv.range.end())
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
}
