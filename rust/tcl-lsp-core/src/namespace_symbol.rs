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
//!   declaring site of its own.  It is answered separately, by
//!   [`namespace_implicit_parent_spans`], with the *covering prefix* of the
//!   deepest written name — the sub-range spelling exactly that namespace,
//!   never the whole word, which names a different one — and hover words it
//!   as "implicitly created by" rather than "declared by" (issue #1113
//!   item 1).  A prefix that is not written at all (inside `namespace eval
//!   ::p`, the word `q::r` spells no `::p`) still answers nothing.
//! * A **computed** target (`namespace eval $ns { … }`) is recorded only when
//!   its value is constant-dominated — `set ns ::app; namespace eval $ns
//!   { … }` creates `::app` on every run, so the `$ns` word is that
//!   namespace's declaring site (issue #1113 item 3).  A branch-conditional
//!   or parameter-fed target proves nothing and is recorded nowhere, so it
//!   neither answers nor pollutes another namespace's reference set.  A
//!   computed word in *reference* position stays unrecorded either way.
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

/// The sites that create `cell` **implicitly**, as the parent of a deeper
/// declared namespace — `namespace eval ::p::q::r {}` really does create
/// `::p` and `::p::q` on both interpreters (`namespace exists ::p::q` is `1`
/// straight after), but neither name is written as a name of its own.
///
/// The answer is the **covering prefix** of the deepest written name: the
/// leading sub-range of that `namespace eval`'s name word which spells
/// `cell`, so the span is real source text naming exactly this namespace and
/// nothing more — never the whole word, which names a different namespace
/// (issue #1113 item 1).
///
/// Empty when `cell` is declared outright (the declaration is the better
/// answer and [`namespace_declaration_spans`] already gives it), and empty
/// when the prefix is not written either: inside `namespace eval ::p`, the
/// word of `namespace eval q::r` spells only `q::r`, so `::p` has no covering
/// prefix here and the honest answer is nothing at all.
#[must_use]
pub fn namespace_implicit_parent_spans(
    source: &str,
    analysis: &AnalysisResult,
    cell: &str,
) -> Vec<Span> {
    let wanted = normalise_namespace(cell);
    if wanted.is_empty() {
        // The global namespace is created by the interpreter, not by any
        // block, so it has no implicit site either.
        return Vec::new();
    }
    let wanted_depth = wanted.split("::").count();
    analysis
        .namespace_refs
        .iter()
        .filter(|r| r.declares)
        .filter_map(|r| {
            let declared = normalise_namespace(&r.qualified_name);
            // Strict descendant only — an exact match is a real declaration.
            declared
                .strip_prefix(wanted)
                .filter(|rest| rest.starts_with("::"))?;
            covering_prefix_span(source, r.span, declared, wanted_depth)
        })
        .collect()
}

/// The leading sub-range of the name word at `span` that spells the first
/// `wanted_depth` segments of `declared`.
///
/// The word may be written relatively (`namespace eval q::r` inside `::p`
/// declares `::p::q::r` from two written segments), so the written text
/// covers only the *last* N segments of the qualified name; a prefix shorter
/// than that offset is not written here at all and yields `None`.
fn covering_prefix_span(
    source: &str,
    span: Span,
    declared: &str,
    wanted_depth: usize,
) -> Option<Span> {
    let text = source.get(span.start() as usize..span.end() as usize)?;
    let rooted = text.starts_with("::");
    let written = text.trim_start_matches("::");
    let written_depth = written.split("::").count();
    let declared_depth = declared.split("::").count();
    // How many leading segments of `declared` this word does not spell.
    let unwritten = declared_depth.checked_sub(written_depth)?;
    let take = wanted_depth.checked_sub(unwritten)?;
    if take == 0 || take > written_depth {
        return None;
    }
    // Byte offset of the end of the `take`-th written segment.
    let mut end = 0usize;
    for (i, segment) in written.split("::").take(take).enumerate() {
        if i > 0 {
            end += 2;
        }
        end += segment.len();
    }
    let lead = u32::try_from(usize::from(rooted) * 2).ok()?;
    let end = u32::try_from(end).ok()?;
    Some(Span::new(span.start(), span.start() + lead + end))
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
    /// Deeper `namespace eval` blocks that create this namespace **only as a
    /// parent** — `namespace eval ::p::q::r {}` for `::p::q` (issue #1113
    /// item 1).  Counted separately because it is a weaker statement than a
    /// declaration and hover must not word the two alike.
    pub implicit_declarations: usize,
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
            implicit_declarations: self.implicit_declarations + other.implicit_declarations,
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
    // Deeper blocks that create `cell` only as a parent.  Pure name
    // arithmetic, so the workspace tier can fold in its own rows the same
    // way without needing any document's text.
    let wanted = normalise_namespace(cell);
    let implicit_declarations = analysis
        .namespace_refs
        .iter()
        .filter(|r| r.declares && !wanted.is_empty())
        .filter(|r| {
            normalise_namespace(&r.qualified_name)
                .strip_prefix(wanted)
                .is_some_and(|rest| rest.starts_with("::"))
        })
        .count();
    NamespaceFacts {
        declarations,
        references,
        implicit_declarations,
        documents: usize::from(declarations + references + implicit_declarations > 0),
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
        // Nothing declares it outright — but a deeper block may still create
        // it as a parent, which is a real answer and a differently-worded one
        // (issue #1113 item 1).
        if facts.implicit_declarations == 0 {
            return None;
        }
        let blocks = if facts.implicit_declarations == 1 {
            "1 deeper `namespace eval` block".to_owned()
        } else {
            format!(
                "{} deeper `namespace eval` blocks",
                facts.implicit_declarations,
            )
        };
        return Some(format!(
            "**Namespace** `{cell}`\n\nImplicitly created by {blocks}; {} other reference(s)",
            facts.references,
        ));
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

    // A **computed** target is recorded only when its value is
    // constant-dominated (issue #1113 item 3): `set ns ::mypkg` then
    // `namespace eval $ns {}` creates `::mypkg` on every run, so the `$ns`
    // word of the *declaring* command is that namespace's declaring site.
    // A reference-position `$ns` (`namespace children $ns`) is still
    // recorded nowhere — reading a variable is not writing a name, and the
    // whole-word role scan skips dynamic words.
    #[test]
    fn tp_only_the_constant_computed_declaration_is_recorded() {
        let src = "set ns ::mypkg\nnamespace eval $ns {}\nnamespace children $ns\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::mypkg")),
            vec!["$ns"],
            "{:?}",
            analysis.namespace_refs,
        );
        assert!(
            namespace_reference_spans(&analysis, "::mypkg").is_empty(),
            "a computed *reference* still names nothing statically: {:?}",
            analysis.namespace_refs,
        );
    }

    #[test]
    fn tn_a_computed_target_with_no_constant_value_is_recorded_nowhere() {
        let src = "proc mk {ns} { namespace eval $ns {} }\n";
        let analysis = analyse(src);
        assert!(
            analysis.namespace_refs.is_empty(),
            "a parameter target names nothing statically: {:?}",
            analysis.namespace_refs,
        );
    }

    // A namespace that exists only as an **implicit parent** has no declaring
    // site of its own — `namespace eval ::p::q::r {}` really does create
    // `::p` and `::p::q` (both interpreters), but neither name is written as
    // a name of its own.  The answer is the covering prefix of the deepest
    // written name, never the whole word (issue #1113 item 1).
    #[test]
    fn tp_implicit_parent_answers_with_the_covering_prefix() {
        let src = "namespace eval ::p::q::r {}\nnamespace children ::p::q\n";
        let analysis = analyse(src);
        assert_eq!(cell_at(src, "::p::q\n").as_deref(), Some("::p::q"));
        assert!(
            namespace_declaration_spans(&analysis, "::p::q").is_empty(),
            "an implicit parent is not a declaration",
        );
        assert_eq!(
            texts(src, &namespace_implicit_parent_spans(src, &analysis, "::p::q")),
            vec!["::p::q"],
            "the prefix, not the whole `::p::q::r` word",
        );
        assert_eq!(
            texts(src, &namespace_implicit_parent_spans(src, &analysis, "::p")),
            vec!["::p"],
        );
        // The declared namespace itself keeps its exact declaration and gains
        // no implicit site.
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::p::q::r")),
            vec!["::p::q::r"],
        );
        assert!(
            namespace_implicit_parent_spans(src, &analysis, "::p::q::r").is_empty(),
        );
    }

    #[test]
    fn tp_a_relatively_written_name_still_yields_the_written_prefix() {
        // Only the segments actually written can be answered with: inside
        // `::outer`, the word of `namespace eval a::b` spells `a::b`, so
        // `::outer::a` is covered by its first segment…
        let src = "namespace eval ::outer {\n    namespace eval a::b {}\n}\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(
                src,
                &namespace_implicit_parent_spans(src, &analysis, "::outer::a")
            ),
            vec!["a"],
        );
        // …while `::outer` itself is not spelled in that word at all, so it
        // gets no implicit site from it — its own block declares it anyway.
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::outer")),
            vec!["::outer"],
        );
    }

    #[test]
    fn fp_an_unrelated_namespace_gets_no_implicit_site() {
        // FP guard — prefix matching is segment-wise, so `::pq` must not be
        // read as a parent of `::p::q::r`.
        let src = "namespace eval ::p::q::r {}\n";
        let analysis = analyse(src);
        assert!(namespace_implicit_parent_spans(src, &analysis, "::pq").is_empty());
        assert!(namespace_implicit_parent_spans(src, &analysis, "::q").is_empty());
    }

    #[test]
    fn tn_the_global_namespace_has_no_implicit_site() {
        // TN — `::` is created by the interpreter, not by any block.
        let src = "namespace eval ::p::q {}\n";
        let analysis = analyse(src);
        assert!(namespace_implicit_parent_spans(src, &analysis, "::").is_empty());
    }

    #[test]
    fn tp_hover_labels_an_implicitly_created_namespace_differently() {
        let src = "namespace eval ::p::q::r {}\nnamespace children ::p::q\n";
        let analysis = analyse(src);
        let text = namespace_hover_text(&analysis, "::p::q").expect("hover");
        assert!(text.contains("Implicitly created by"), "{text}");
        assert!(text.contains("1 deeper `namespace eval` block"), "{text}");
        // …and a really-declared namespace keeps the stronger wording.
        let declared = namespace_hover_text(&analysis, "::p::q::r").expect("hover");
        assert!(declared.contains("Declared by"), "{declared}");
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
            implicit_declarations: 0,
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
            implicit_declarations: 0,
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
                implicit_declarations: 0,
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

    // `namespace path {::a ::b}` — issue #1113 item 2.  The argument is a
    // *list* of namespace names inside one word, so no whole-word `ArgRole`
    // can mark it; each element is recorded at its own span by the analyser's
    // `namespace path` handler, using the shared Tcl list grammar.

    #[test]
    fn tp_each_namespace_path_element_is_its_own_reference() {
        let src = "namespace eval ::a {}\n\
                   namespace eval ::b {}\n\
                   namespace path {::a ::b}\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::a")),
            vec!["::a"],
        );
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::b")),
            vec!["::b"],
        );
        // …and a cursor on the second element names that namespace, so
        // go-to-definition reaches its declaring block.
        let off = u32::try_from(src.rfind("::b").expect("found")).expect("offset fits u32");
        assert_eq!(
            namespace_cell_at_offset(src, &analysis, off).as_deref(),
            Some("::b"),
        );
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::b")),
            vec!["::b"],
        );
    }

    #[test]
    fn tp_a_relative_path_element_roots_against_the_declaring_namespace() {
        // The path entry follows the ordinary relative rule, which is what
        // `command_resolution_candidates` already assumes.
        let src = "namespace eval ::outer {\n\
                   \x20   namespace eval helpers {}\n\
                   \x20   namespace path helpers\n\
                   }\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(
                src,
                &namespace_reference_spans(&analysis, "::outer::helpers")
            ),
            vec!["helpers"],
        );
    }

    #[test]
    fn tp_a_braced_path_element_gets_the_element_span_not_the_word() {
        // A braced element (`{my ns}`) is exactly what the old
        // `split_whitespace` split could not express — it made two bogus
        // entries.  The list grammar keeps it one name.
        let src = "namespace path {{my ns} ::b}\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::my ns")),
            vec!["my ns"],
        );
    }

    #[test]
    fn fp_a_path_word_carrying_substitution_records_nothing() {
        // FP guard — the whole word substitutes, so no element has a span
        // whose text is a namespace name.  The handler's existing
        // dynamic-word gate rejects the command before the split, which is
        // the conservative answer and the one this test pins.
        let src = "namespace eval ::a {}\nnamespace path [concat $extra ::a]\n";
        let analysis = analyse(src);
        assert!(
            namespace_reference_spans(&analysis, "::a").is_empty(),
            "a computed path claims no element spans: {:?}",
            analysis.namespace_refs,
        );
    }

    #[test]
    fn tn_a_braced_path_word_that_looks_dynamic_is_still_skipped() {
        // TN / documented limit — braces suppress substitution, so tclsh
        // reads `$ns` here as a *literal* namespace name.  The handler's
        // whole-word dynamic gate predates the element split and skips the
        // command outright; recording nothing is a miss, never a wrong span.
        let src = "namespace path {$ns ::a}\n";
        let analysis = analyse(src);
        assert!(
            analysis.namespace_refs.is_empty(),
            "{:?}",
            analysis.namespace_refs,
        );
    }

    #[test]
    fn fp_a_whole_word_dynamic_path_records_nothing_at_all() {
        let src = "namespace path $entries\n";
        let analysis = analyse(src);
        assert!(
            analysis.namespace_refs.is_empty(),
            "{:?}",
            analysis.namespace_refs,
        );
    }

    // `apply {{x} {…} ::ns}`'s third list element — issue #1113 item 4.  The
    // analyser already models its *semantics* (`namespace_overrides`); this
    // is its reference identity, which sits inside an
    // `ArgRole::LambdaLiteral` word no whole-word role reaches.

    #[test]
    fn tp_an_apply_lambdas_namespace_element_is_a_reference() {
        let src = "namespace eval ::ns {}\napply {{x} { return $x } ::ns} 1\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_reference_spans(&analysis, "::ns")),
            vec!["::ns"],
        );
        let off = u32::try_from(src.rfind("::ns").expect("found")).expect("offset fits u32");
        assert_eq!(
            namespace_cell_at_offset(src, &analysis, off).as_deref(),
            Some("::ns"),
        );
    }

    #[test]
    fn fp_a_two_element_lambda_records_no_namespace_reference() {
        // FP guard — the element is optional; a two-element lambda runs in
        // `::` and names nothing.
        let src = "apply {{x} { return $x }} 1\n";
        let analysis = analyse(src);
        assert!(
            analysis.namespace_refs.is_empty(),
            "{:?}",
            analysis.namespace_refs,
        );
    }

    #[test]
    fn fp_a_dynamic_lambda_namespace_element_records_nothing() {
        let src = "apply [list {x} { return $x } $ns] 1\n";
        let analysis = analyse(src);
        assert!(
            analysis.namespace_refs.is_empty(),
            "{:?}",
            analysis.namespace_refs,
        );
    }

    // A constant-dominated computed target — issue #1113 item 3.  `set ns
    // ::app; namespace eval $ns { … }` creates `::app` on every run, so the
    // `$ns` word is a declaring occurrence and navigation reaches it.

    #[test]
    fn tp_a_constant_computed_target_is_a_declaring_site() {
        let src = "set ns ::app\nnamespace eval $ns { proc go {} { return 1 } }\nnamespace children ::app\n";
        let analysis = analyse(src);
        assert_eq!(
            texts(src, &namespace_declaration_spans(&analysis, "::app")),
            vec!["$ns"],
        );
        // …and the reference in `namespace children ::app` reaches it.
        let off = u32::try_from(src.rfind("::app").expect("found")).expect("offset fits u32");
        assert_eq!(
            namespace_cell_at_offset(src, &analysis, off).as_deref(),
            Some("::app"),
        );
    }

    #[test]
    fn fp_a_branch_conditional_target_declares_nothing() {
        // FP guard — which namespace the block creates is a run-time
        // question, so no declaration may be claimed for either candidate.
        let src = "set ns ::a\nif {$c} { set ns ::b }\nnamespace eval $ns { proc go {} { return 1 } }\n";
        let analysis = analyse(src);
        assert!(namespace_declaration_spans(&analysis, "::a").is_empty());
        assert!(namespace_declaration_spans(&analysis, "::b").is_empty());
    }
}
