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

//! Namespaces as **navigable symbols** — the one entry point every provider
//! resolves a namespace name through (issue #1088).
//!
//! Before this, a namespace existed in the model only as a *container*: a
//! [`ScopeKind::Namespace`](tcl_compiler::analyser::ScopeKind) node holding
//! variables and procs, and a `Namespace` entry in the document outline.  A
//! namespace *name* written as an argument — `namespace children ::tomato`,
//! `namespace exists ::x`, `namespace delete ::a`, the `::app` of a second
//! `namespace eval ::app { … }` block — resolved to nothing at all, in any
//! provider, even single-file.
//!
//! ## What a namespace's definition is
//!
//! Pinned on tclsh 9.0.4 and 8.6.16, byte-identically:
//!
//! ```text
//! namespace eval ::a { variable x 1 }
//! namespace eval ::a { variable y 2 }
//! namespace exists ::a            -> 1
//! info vars ::a::*                -> ::a::x ::a::y
//! ```
//!
//! Both blocks are the same namespace: the first creates it, the second
//! extends it.  So a namespace has a **set** of declaring sites, not one, and
//! go-to-definition answers with all of them in source order — the same shape
//! find-references already takes for a proc declared twice (issue #923
//! idx 31), and the reason [`namespace_declaration_spans`] returns a `Vec`.
//!
//! `namespace eval` is also the **only** declaring form.  A qualified `proc`
//! does *not* create the namespace it names:
//!
//! ```text
//! proc ::nope::gone::p {} {}  -> can't create procedure "::nope::gone::p": unknown namespace
//! set ::brandnew::v 1         -> can't set "::brandnew::v": parent namespace doesn't exist
//! ```
//!
//! which is why [`tcl_registry::Traits::DECLARES_NAMESPACE`] sits on
//! `namespace eval` alone — not on `namespace inscope`, whose argument layout
//! and analyser hook are identical but which requires the namespace to exist.
//!
//! ## Relative names
//!
//! A namespace word roots against the command-resolution namespace in force
//! where it is written, exactly as Tcl resolves it (both interpreters: inside
//! `namespace eval ::outer`, `namespace exists inner` is `1` for
//! `::outer::inner`, and the same words at global scope are `0`).  The
//! analyser does that rooting once, when it records the occurrence, so every
//! consumer here compares fully-qualified names and none of them re-derives
//! the rule.
//!
//! ## Bounds
//!
//! * A namespace created only **implicitly**, as a parent (`namespace eval
//!   ::p::q::r {}` creates `::p` and `::p::q` too, both interpreters), has no
//!   declaring site of its own.  Asking for `::p::q` answers nothing rather
//!   than jumping into the middle of the `::p::q::r` word, which is not that
//!   namespace's name.
//! * A **computed** target (`namespace eval $ns { … }`) names no static
//!   namespace and is never recorded, so it neither answers nor pollutes
//!   another namespace's reference set.
//! * Inert text is excluded: a `namespace exists ::x` that is really comment
//!   prose or a braced data word is not code, so it must not resolve
//!   (issue #923 idx 24).  Here that falls out of the model rather than
//!   needing a textual gate — a `NamespaceRef` exists only because the
//!   analyser walked the command as live code, and the walk descends neither
//!   comments nor braced data words.  See [`namespace_cell_at_offset`] for
//!   why [`crate::inert_text::offset_in_data_brace`] specifically must *not*
//!   be applied: a namespace name may legitimately be a braced word
//!   (`namespace eval {my ns} { … }` creates a real namespace on both
//!   interpreters).

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::Span;

/// The `::`-rooted **namespace** the cursor names, when it names one.
///
/// The single entry point definition / hover / find-references (and their
/// cross-document tiers) resolve a namespace through, so they cannot disagree
/// about which position is about a namespace at all.  Deliberately
/// span-driven rather than word-driven: the answer is `Some` only when the
/// cursor sits inside a word the registry marked
/// [`tcl_registry::ArgRole::NamespaceName`] and the analyser recorded, so a
/// bare word that merely *looks* like a namespace never claims a position a
/// proc / class / variable resolver owns.
///
/// Both cursor shapes answer, and they are the same shape:
///
/// 1. A **reference** — the `::tomato` of `namespace children ::tomato`.
/// 2. A **declaration** — the `::tomato` of `namespace eval ::tomato { … }`,
///    which is also a `NamespaceName` word.  That is what lets
///    find-references run *from* the declaration.
#[must_use]
pub fn namespace_cell_at(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<String> {
    let line_index = tcl_lexer::LineIndex::new(source);
    let cursor_off = crate::definition::byte_offset_at(&line_index, source, line, character);
    namespace_cell_at_offset(source, analysis, cursor_off)
}

/// [`namespace_cell_at`] keyed on a byte offset the caller already has.
#[must_use]
pub fn namespace_cell_at_offset(
    source: &str,
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<String> {
    let hit = analysis
        .namespace_refs
        .iter()
        .find(|r| r.span.start() <= cursor_off && cursor_off < r.span.end())?;
    // The recording itself is the liveness proof, and it is a *stronger* one
    // than the textual data-brace test: a `NamespaceRef` exists only because
    // the analyser walked that command as live code, and the walk never
    // descends a comment or a braced data word (`set d {namespace children
    // ::mypkg}` records nothing at all — see the TN test).
    //
    // Applying [`crate::inert_text::offset_in_data_brace`] here would be
    // actively wrong, because a namespace-name argument may legitimately *be*
    // a braced word.  Pinned on tclsh 9.0.4 and 8.6.16, byte-identical:
    // `namespace eval {my ns} { variable v 7; proc p {} {return P} }` creates
    // a real namespace — `namespace exists {my ns}` is `1`, `namespace
    // children ::` lists `{::my ns}`, `set {::my ns::v}` is `7`, `{::my
    // ns::p}` runs — and `namespace eval {} { … }` reopens the global one.
    // The brace-role test sees a word whose `ArgRole` does not carry script
    // and calls it data, which would silently un-resolve every such name.
    //
    // The comment test stays: it is conservative (it answers "inert" only
    // when the position provably is), so it can only ever agree with the
    // walk, and it costs a cheap scan (issue #923 idx 24).
    if crate::inert_text::offset_in_comment(source, cursor_off) {
        return None;
    }
    Some(hit.qualified_name.clone())
}

/// Whether `a` and `b` name the same namespace.
///
/// Both are `::`-rooted by the analyser, so this is an exact comparison —
/// with the one normalisation the **root** needs: the global namespace's real
/// name is the empty string and `::` is its accepted synonym everywhere
/// (`namespace current` at top level prints `::` on both interpreters, and
/// `namespace eval :: {…}` / `namespace eval {} {…}` reopen the same
/// namespace — `namespace current` inside each is `::`).
///
/// Only the root collapses.  A *trailing* separator elsewhere is a different
/// namespace, not a spelling of its parent: `::outer::` is the empty-named
/// child of `::outer`, which Tcl refuses to create at all (`can't create
/// namespace "": only global namespace can have empty name`, and `namespace
/// exists {}` inside `namespace eval ::outer` answers `0` — tclsh 9.0.4 and
/// 8.6.16, byte-identical). Folding it into `::outer` would answer a query
/// about a namespace that cannot exist with the enclosing namespace's
/// declarations.
fn normalise_namespace(name: &str) -> &str {
    match name.trim_start_matches("::") {
        // The root, however it was spelled (`::`, `` ``, `::::`).
        root if root.is_empty() || root.chars().all(|c| c == ':') => "",
        other => other,
    }
}

fn same_namespace(a: &str, b: &str) -> bool {
    normalise_namespace(a) == normalise_namespace(b)
}

/// Every **declaring** site of `cell` in this document, in source order — the
/// name token of each `namespace eval` block that creates or extends it.
///
/// Empty when the document declares the namespace nowhere, including the
/// implicit-parent case described in the module docs.
#[must_use]
pub fn namespace_declaration_spans(analysis: &AnalysisResult, cell: &str) -> Vec<Span> {
    analysis
        .namespace_refs
        .iter()
        .filter(|r| r.declares && same_namespace(&r.qualified_name, cell))
        .map(|r| r.span)
        .collect()
}

/// Every **non-declaring** occurrence of `cell` in this document, in source
/// order — the reference set find-references reports when the declarations
/// are not asked for.
#[must_use]
pub fn namespace_reference_spans(analysis: &AnalysisResult, cell: &str) -> Vec<Span> {
    analysis
        .namespace_refs
        .iter()
        .filter(|r| !r.declares && same_namespace(&r.qualified_name, cell))
        .map(|r| r.span)
        .collect()
}

/// Every occurrence of `cell` in this document — declarations included when
/// `include_declaration`, in source order.
#[must_use]
pub fn namespace_all_spans(
    analysis: &AnalysisResult,
    cell: &str,
    include_declaration: bool,
) -> Vec<Span> {
    analysis
        .namespace_refs
        .iter()
        .filter(|r| (include_declaration || !r.declares) && same_namespace(&r.qualified_name, cell))
        .map(|r| r.span)
        .collect()
}

/// What is known about a namespace, over whatever document set the caller
/// gathered — one document for the in-document provider, the whole workspace
/// index for the server's cross-document tier.
///
/// The counts and the document tally travel together so
/// [`namespace_hover_markdown`] can say *which* set it is describing. A
/// namespace reopened in three files is a single symbol with three declaring
/// blocks (tclsh 9.0.4 / 8.6.16: `namespace eval ::a {}` twice leaves one
/// namespace holding both blocks' contents), so a hover that counted only the
/// blocks in the file under the cursor would contradict go-to-definition,
/// which offers all of them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NamespaceFacts {
    /// Declaring `namespace eval` blocks.
    pub declarations: usize,
    /// Occurrences that are not declarations.
    pub references: usize,
    /// How many documents contributed — `1` for a single-document tally.
    pub documents: usize,
}

impl NamespaceFacts {
    /// Fold another document's counts in.
    #[must_use]
    pub fn merged(self, other: Self) -> Self {
        Self {
            declarations: self.declarations + other.declarations,
            references: self.references + other.references,
            documents: self.documents + other.documents,
        }
    }
}

/// What one document knows about `cell`.
#[must_use]
pub fn namespace_facts(analysis: &AnalysisResult, cell: &str) -> NamespaceFacts {
    let (declarations, references) = analysis
        .namespace_refs
        .iter()
        .filter(|r| same_namespace(&r.qualified_name, cell))
        .fold(
            (0, 0),
            |(d, r), row| {
                if row.declares { (d + 1, r) } else { (d, r + 1) }
            },
        );
    NamespaceFacts {
        declarations,
        references,
        documents: usize::from(declarations + references > 0),
    }
}

/// Render hover markdown for the namespace `cell` from `facts`.
///
/// `None` when nothing in the gathered set **declares** it — hover then
/// abstains rather than describing a namespace nothing in view creates, the
/// same rule the variable tier follows
/// ([`crate::hover::qualified_variable_hover`]).  The abstention is what lets
/// the server's cross-document tier answer; it must never become a
/// fall-through to *command* hover, because the cursor is provably on a
/// namespace-name argument (issue #1088 review, finding 1).
///
/// The one renderer, so the in-document provider and the workspace tier
/// cannot word the same fact differently — they differ only in how wide a set
/// they counted, which the text states.
#[must_use]
pub fn namespace_hover_markdown(cell: &str, facts: NamespaceFacts) -> Option<String> {
    if facts.declarations == 0 {
        return None;
    }
    let blocks = if facts.declarations == 1 {
        "1 `namespace eval` block".to_owned()
    } else {
        format!("{} `namespace eval` blocks", facts.declarations)
    };
    let scope = if facts.documents > 1 {
        format!(" across {} documents", facts.documents)
    } else {
        " in this document".to_owned()
    };
    Some(format!(
        "**Namespace** `{cell}`\n\nDeclared by {blocks}{scope}; {} other reference(s)",
        facts.references,
    ))
}

/// Hover text for the namespace `cell` from a single document's analysis.
#[must_use]
pub fn namespace_hover_text(analysis: &AnalysisResult, cell: &str) -> Option<String> {
    namespace_hover_markdown(cell, namespace_facts(analysis, cell))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    /// The cell the cursor at the first occurrence of `needle` names.
    fn cell_at(source: &str, needle: &str) -> Option<String> {
        let analysis = analyse(source);
        let off =
            u32::try_from(source.find(needle).expect("needle present")).expect("offset fits u32");
        namespace_cell_at_offset(source, &analysis, off)
    }

    fn texts(source: &str, spans: &[Span]) -> Vec<String> {
        spans
            .iter()
            .map(|s| source[s.start() as usize..s.end() as usize].to_owned())
            .collect()
    }

    // TP: a namespace *name* argument resolves to the namespace, and its
    // declaring `namespace eval` blocks are the definition set.  Both blocks
    // answer — tclsh 9.0.4 / 8.6.16 agree byte-for-byte that reopening
    // `::mypkg` extends the one namespace (`info vars ::mypkg::*` lists both
    // blocks' variables).
    #[test]
    fn tp_namespace_name_argument_resolves_to_every_declaring_block() {
        let src = "namespace eval mypkg { variable a 1 }\n\
                   namespace eval mypkg { variable b 2 }\n\
                   namespace children ::mypkg\n";
        assert_eq!(cell_at(src, "::mypkg").as_deref(), Some("::mypkg"));
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::mypkg")),
            vec!["mypkg", "mypkg"],
        );
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::mypkg")),
            vec!["::mypkg"],
        );
    }

    // TP: the real mined shape — the name inside a `[…]` command
    // substitution (`set targets [namespace children ::tomato]`), which the
    // main walk treats as an opaque value.
    #[test]
    fn tp_namespace_name_inside_command_substitution_resolves() {
        let src = "namespace eval tomato {}\nset targets [namespace children ::tomato]\n";
        assert_eq!(cell_at(src, "::tomato").as_deref(), Some("::tomato"));
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::tomato")),
            vec!["tomato"],
        );
    }

    // TP: a **relative** name roots against the enclosing namespace, exactly
    // as Tcl does — tclsh 9.0.4 / 8.6.16: inside `namespace eval ::outer`,
    // `namespace exists inner` is 1; the same words at global scope are 0.
    #[test]
    fn tp_relative_name_roots_against_the_enclosing_namespace() {
        let src = "namespace eval ::outer {\n\
                   \x20   namespace eval inner {}\n\
                   \x20   namespace children inner\n\
                   }\n\
                   namespace children inner\n";
        let analysis = analyse(src);
        // The occurrence inside `::outer` means `::outer::inner`…
        let inside =
            u32::try_from(src.find("children inner").expect("found") + 9).expect("offset fits u32");
        assert_eq!(
            namespace_cell_at_offset(src, &analysis, inside).as_deref(),
            Some("::outer::inner"),
        );
        // …and the global one means `::inner`, a different namespace that
        // nothing here declares.
        let outside = u32::try_from(src.rfind("children inner").expect("found") + 9)
            .expect("offset fits u32");
        assert_eq!(
            namespace_cell_at_offset(src, &analysis, outside).as_deref(),
            Some("::inner"),
        );
        assert!(namespace_declaration_spans(&analysis, "::inner").is_empty());
        assert_eq!(
            texts(
                src,
                &namespace_declaration_spans(&analysis, "::outer::inner")
            ),
            vec!["inner"],
        );
    }

    // TP: every subcommand the registry marks carries the role — `exists`,
    // `delete` (variadic), `parent`, `upvar`, `inscope`.
    #[test]
    fn tp_every_marked_subcommand_records_its_namespace_word() {
        let src = "namespace exists ::a\n\
                   namespace delete ::a ::b\n\
                   namespace parent ::a\n\
                   namespace upvar ::a v l\n\
                   namespace inscope ::a {set x 1}\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::a")).len(),
            5,
            "exists / delete / parent / upvar / inscope all reference `::a`: {:?}",
            analysis.namespace_refs,
        );
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::b")),
            vec!["::b"],
        );
    }

    // FP guard: `namespace qualifiers` / `tail` take an arbitrary **string**,
    // not a namespace — `namespace tail a::b` answers `b` on both
    // interpreters whether or not `::a` exists — and `import` / `export` /
    // `forget` take glob patterns.  None of them may claim a namespace
    // reference.
    #[test]
    fn fp_string_and_pattern_arguments_are_not_namespace_references() {
        let src = "namespace eval a {}\n\
                   namespace tail a::b\n\
                   namespace qualifiers a::b\n\
                   namespace import a::*\n\
                   namespace export a\n\
                   namespace forget a::*\n";
        let analysis = analyse(src);
        assert!(
            namespace_reference_spans(&analysis, "::a").is_empty(),
            "only the declaring block may be recorded: {:?}",
            analysis.namespace_refs,
        );
    }

    // FP guard: `namespace which -command ::a` / `namespace origin ::a` name
    // a **command**, and `namespace code` takes a script.  Namespaces are a
    // disjoint symbol space, so none of these is a namespace reference.
    #[test]
    fn fp_command_name_arguments_are_not_namespace_references() {
        let src = "namespace eval a {}\n\
                   namespace origin ::a\n\
                   namespace which -command ::a\n\
                   namespace code {puts hi}\n";
        let analysis = analyse(src);
        assert!(
            namespace_reference_spans(&analysis, "::a").is_empty(),
            "command-name words are not namespace references: {:?}",
            analysis.namespace_refs,
        );
    }

    // TN: a namespace-name-shaped run of text inside a **comment** is inert —
    // Tcl runs nothing there, so it must resolve to nothing (issue #923
    // idx 24's rule, applied to this new symbol kind).
    #[test]
    fn tn_comment_text_is_not_a_namespace_reference() {
        let src = "namespace eval mypkg {}\n# namespace children ::mypkg here\n";
        assert_eq!(cell_at(src, "::mypkg here"), None);
        let analysis = analyse(src);
        assert!(namespace_reference_spans(&analysis, "::mypkg").is_empty());
    }

    // TN: the same inside a **braced data word** — `set d {namespace children
    // ::mypkg}` is a literal string on both interpreters, never a command.
    #[test]
    fn tn_braced_data_word_is_not_a_namespace_reference() {
        let src = "namespace eval mypkg {}\nset d {namespace children ::mypkg}\n";
        assert_eq!(cell_at(src, "::mypkg}"), None);
        let analysis = analyse(src);
        assert!(namespace_reference_spans(&analysis, "::mypkg").is_empty());
    }

    // TN: a **computed** target names no static namespace, so it is recorded
    // nowhere and cannot be mistaken for a literal one.
    #[test]
    fn tn_computed_namespace_target_is_not_recorded() {
        let src = "set ns ::mypkg\nnamespace eval $ns {}\nnamespace children $ns\n";
        let analysis = analyse(src);
        assert!(
            analysis.namespace_refs.is_empty(),
            "a `$ns` target names nothing statically: {:?}",
            analysis.namespace_refs,
        );
    }

    // TN: a namespace that exists only as an **implicit parent** has no
    // declaring site of its own.  `namespace eval ::p::q::r {}` really does
    // create `::p` and `::p::q` (both interpreters), but neither name is
    // written anywhere, so definition abstains rather than jumping into the
    // middle of another namespace's name word.
    #[test]
    fn tn_implicit_parent_namespace_has_no_declaration_site() {
        let src = "namespace eval ::p::q::r {}\nnamespace children ::p::q\n";
        let analysis = analyse(src);
        assert_eq!(cell_at(src, "::p::q\n").as_deref(), Some("::p::q"));
        assert!(namespace_declaration_spans(&analysis, "::p::q").is_empty());
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::p::q::r")),
            vec!["::p::q::r"],
        );
    }

    // FN guard: `namespace inscope` must **not** be treated as a declaration —
    // it requires the namespace to exist already, and the discriminator is
    // the registry's `DECLARES_NAMESPACE`, not the argument layout (which is
    // identical to `eval`'s).
    #[test]
    fn fn_inscope_is_a_reference_not_a_declaration() {
        let src = "namespace inscope ::a {set x 1}\n";
        let analysis = analyse(src);
        assert!(namespace_declaration_spans(&analysis, "::a").is_empty());
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::a")),
            vec!["::a"],
        );
    }

    // FN guard: find-references run *from* the declaration must reach the
    // references, and `include_declaration` decides whether the declaring
    // blocks come back too.
    #[test]
    fn fn_references_from_the_declaration_reach_every_use() {
        let src = "namespace eval mypkg {}\n\
                   namespace children ::mypkg\n\
                   namespace exists mypkg\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_all_spans(&analysis, "::mypkg", true)),
            vec!["mypkg", "::mypkg", "mypkg"],
        );
        assert_eq!(
            texts(src, &namespace_all_spans(&analysis, "::mypkg", false)),
            vec!["::mypkg", "mypkg"],
        );
    }

    // The global namespace's real name is the empty string and `::` is its
    // accepted synonym everywhere (`namespace current` prints `::` at top
    // level on both interpreters), so the two spellings must be one symbol.
    #[test]
    fn tp_global_namespace_spellings_are_one_symbol() {
        let src = "namespace eval :: {}\nnamespace children ::\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_all_spans(&analysis, "::", true)).len(),
            2,
        );
    }

    // TP (issue #1088 review, finding 3): the **empty literal** is an
    // ordinary relative namespace name, so it means `::` at global scope and
    // a namespace that cannot exist inside another one.
    //
    // Oracle — tclsh 9.0.4 and 8.6.16, byte-identical.  At global scope:
    // `namespace exists {}` -> 1; `namespace children {}` == `namespace
    // children ::`; `namespace inscope {} {namespace current}` -> `::`;
    // `namespace eval {} {…}` reopens `::` (`namespace current` inside is
    // `::`, and a `variable` it declares lands in `::`).  Inside `namespace
    // eval ::outer`: `namespace exists {}` -> 0; `namespace children {}`
    // fails `namespace "" not found in "::outer"`; `namespace eval {} {…}`
    // fails `can't create namespace "": only global namespace can have empty
    // name`.
    #[test]
    fn tp_empty_literal_is_the_global_namespace_at_global_scope() {
        let src = "namespace eval :: {}\nnamespace eval {} {}\nnamespace children {}\n";
        let analysis = analyse(src);
        assert_eq!(cell_at(src, "{} {}").as_deref(), Some("::"));
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::")),
            vec!["::", "{}"],
            "both spellings declare the global namespace",
        );
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::")),
            vec!["{}"],
        );
    }

    #[test]
    fn tn_empty_literal_inside_a_namespace_names_a_namespace_that_cannot_exist() {
        let src = "namespace eval ::outer {\n    namespace exists {}\n}\n";
        let analysis = analyse(src);
        // It is *recorded* — the word is a real namespace name — but it names
        // `::outer::`, the empty-named child, not `::outer`.
        assert_eq!(cell_at(src, "{}").as_deref(), Some("::outer::"));
        assert!(namespace_declaration_spans(&analysis, "::outer::").is_empty());
        assert!(
            namespace_reference_spans(&analysis, "::outer").is_empty(),
            "`{{}}` must not be folded into the enclosing namespace: {:?}",
            analysis.namespace_refs,
        );
    }

    // TP: a **braced** namespace name is an ordinary name, so the data-brace
    // inertness proof must not veto it (issue #1088 review, finding 3
    // adjacent).
    //
    // Oracle — both interpreters: `namespace eval {my ns} { variable v 7;
    // proc p {} {return P} }` gives `namespace exists {my ns}` -> 1,
    // `namespace children ::` listing `{::my ns}`, `set {::my ns::v}` -> 7,
    // and `{::my ns::p}` -> P.
    #[test]
    fn tp_a_braced_namespace_name_resolves() {
        let src = "namespace eval {my ns} { variable v 7 }\nnamespace children {my ns}\n";
        let analysis = analyse(src);
        assert_eq!(cell_at(src, "{my ns}").as_deref(), Some("::my ns"));
        // The recorded span is the word token's, which for a braced word
        // covers the opening brace and the content but not the closing one —
        // a pre-existing, product-wide convention shared with `proc {my p}`'s
        // own `name_span`, not a namespace-tier fact. What matters here is
        // that the name resolves at all: the data-brace inertness proof used
        // to veto it.
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::my ns")),
            vec!["{my ns"],
        );
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::my ns")),
            vec!["{my ns"],
        );
    }

    // Hover counts travel with the size of the set they describe, so the
    // in-document provider and the server's workspace tier cannot word the
    // same fact differently.
    #[test]
    fn tp_hover_says_which_set_it_counted() {
        let one = NamespaceFacts {
            declarations: 1,
            references: 2,
            documents: 1,
        };
        let text = namespace_hover_markdown("::mypkg", one).expect("hover");
        assert!(
            text.contains("1 `namespace eval` block in this document"),
            "{text}"
        );
        let many = NamespaceFacts {
            declarations: 3,
            references: 4,
            documents: 2,
        };
        let text = namespace_hover_markdown("::mypkg", many).expect("hover");
        assert!(
            text.contains("3 `namespace eval` blocks across 2 documents"),
            "{text}",
        );
        assert_eq!(
            namespace_hover_markdown("::mypkg", NamespaceFacts::default()),
            None,
            "nothing declares it — abstain, never fall through",
        );
    }

    #[test]
    fn tp_facts_merge_across_documents() {
        let a = analyse("namespace eval mypkg {}\nnamespace children ::mypkg\n");
        let b = analyse("namespace eval mypkg {}\n");
        let merged = namespace_facts(&a, "::mypkg").merged(namespace_facts(&b, "::mypkg"));
        assert_eq!(
            merged,
            NamespaceFacts {
                declarations: 2,
                references: 1,
                documents: 2,
            },
        );
    }

    // Hover abstains for a namespace this document declares nowhere, so the
    // cross-document tier can answer instead of a wrong local guess.
    #[test]
    fn tn_hover_abstains_without_a_local_declaration() {
        let src = "namespace children ::elsewhere\n";
        let analysis = analyse(src);
        assert_eq!(namespace_hover_text(&analysis, "::elsewhere"), None);
    }

    #[test]
    fn tp_hover_counts_declaring_blocks_and_references() {
        let src = "namespace eval mypkg {}\n\
                   namespace eval mypkg {}\n\
                   namespace children ::mypkg\n";
        let analysis = analyse(src);
        let text = namespace_hover_text(&analysis, "::mypkg").expect("hover");
        assert!(text.contains("`::mypkg`"), "{text}");
        assert!(text.contains("2 `namespace eval` blocks"), "{text}");
        assert!(text.contains("1 other reference(s)"), "{text}");
    }
}
