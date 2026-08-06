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

//! The **namespace rename tier** (issue #1114).
//!
//! Renaming a namespace is not one symbol's rename: `::old` is written in
//! every `namespace eval` block that opens it, in every qualified name
//! beneath it (`::old::p`, `$::old::v`, `::old::Cls`), in every
//! `NamespaceName`-role argument that mentions it, and in the import /
//! forget patterns that name it.  Miss one and the program stops running;
//! guess at one and it runs differently.  So this tier either produces the
//! complete edit set or **refuses with a reason**, the #1091 precedent.
//!
//! # What one edit looks like
//!
//! Only the namespace's **final segment** changes, so only that segment is
//! rewritten — never the whole word.  That is what makes every spelling work
//! out of one rule:
//!
//! ```text
//! namespace eval ::app::old { … }   ->  namespace eval ::app::new { … }
//! set ::app::old::v 1               ->  set ::app::new::v 1
//! namespace eval ::app { namespace eval old { … } }
//!                                   ->  … { namespace eval new { … } }
//! namespace eval ::app { proc use {} { old::p } }
//!                                   ->  … { proc use {} { new::p } }
//! ```
//!
//! A spelling that does **not** write the segment needs no edit at all and
//! gets none: inside `namespace eval ::app::old`, a bare `p` still resolves
//! to the same proc after the block's own name changes.
//! [`written_cell_segment`] is the single place that decides which byte range
//! of a written name spells the namespace, so no consumer re-derives it.
//!
//! # The refusal gate
//!
//! Coverage equals edits, literally: the gate is asked about every word the
//! edit collector did **not** consider.  A word naming something beneath the
//! namespace that no analyser table attributes — a callback script
//! (`after 0 {::old::tick}`), a `namespace code` body, a dispatch-table
//! literal — is exactly the occurrence an edit set would leave behind, so it
//! refuses.  So do:
//!
//! * a **computed** namespace word (`namespace eval $ns { … }`) that could
//!   spell this namespace — there is no word to rewrite, and the value may be
//!   this very name;
//! * a `namespace path` word built from a **computed** value (`namespace path
//!   $paths`) — its entries are unknowable, and any of them may be this
//!   namespace.  A *literal* path list is rewritten, not refused: each of its
//!   elements is recorded as a `NamespaceRef` at its own span, so the entry
//!   moves through the ordinary edit path (issue #1261);
//! * a **collision** with a namespace that already exists — the server's
//!   half, since it is a workspace question.
//!
//! # Bounds
//!
//! The unattributed-word scan recognises `::`-rooted spellings.  A *relative*
//! name embedded in a string (`after 0 {old::tick}` written inside `::app`)
//! resolves through the frame the script eventually runs in, which is not a
//! lexical fact of the word — abstaining there would mean refusing on any
//! word containing any of the namespace's segments, which would refuse every
//! rename.  Rooted spellings are what a callback string can portably use, and
//! they are what this scan covers.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Span};

use crate::definition::span_to_range;
use crate::rename::TextEdit;
use crate::rename_safety::RenameRefusal;

/// Every edit renaming the namespace `cell` — a `::`-rooted qualified name —
/// so that its final segment becomes `new_tail`, within one document.
///
/// `Err` is a refusal: this document holds an occurrence the rename can
/// neither rewrite nor rule out, so the whole rename must be refused rather
/// than half-applied.  See the [module docs](self) for the gate.
///
/// The result is deduplicated by range; the caller merges each document's
/// share into the workspace edit.
///
/// # Errors
///
/// Returns the [`RenameRefusal`] for the first unprovable occurrence found.
pub fn namespace_rename_edits(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    cell: &str,
    new_tail: &str,
) -> Result<Vec<TextEdit>, RenameRefusal> {
    let line_index = LineIndex::new(source);
    let mut considered: Vec<Span> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut record = |word: Span, resolved: &str, considered: &mut Vec<Span>| {
        considered.push(word);
        if let Some(segment) = written_cell_segment(source, word, resolved, cell) {
            spans.push(segment);
        }
    };

    // 1. Every `NamespaceName`-role word naming the namespace or one beneath
    //    it — the `namespace eval` / `children` / `exists` / `delete` /
    //    `upvar` / `inscope` arguments the analyser recorded.  Descendants
    //    count: `namespace eval ::old::sub {}` writes `old` too, and it is
    //    also how an implicitly-created namespace (#1113 item 1) is renamed
    //    at all — it has no declaring word of its own.
    for nref in &analysis.namespace_refs {
        if names_at_or_under(cell, &nref.qualified_name) {
            record(nref.span, &nref.qualified_name, &mut considered);
        }
    }
    // 2. Qualified variable occurrences beneath it (`$::old::v`, `set
    //    app::old::v 1`).
    for vref in &analysis.qualified_var_refs {
        if names_under(cell, &vref.qualified_name) {
            record(vref.span, &vref.qualified_name, &mut considered);
        }
    }
    // 3. Command call sites resolved beneath it.  An indirect site's span is
    //    not the written name (M7), so it is never a rewrite target — and
    //    never a silent miss either: the gate below sees it as an
    //    unattributed word if it writes the namespace.
    for inv in &analysis.command_invocations {
        let Some(resolved) = inv.resolved_qualified_name.as_deref() else {
            continue;
        };
        if inv.indirect || !names_under(cell, resolved) {
            continue;
        }
        record(inv.range, resolved, &mut considered);
    }
    // 4. Declaration name words beneath it — `proc ::old::p`, `oo::class
    //    create ::old::C`.
    for proc_def in analysis.all_procs.values() {
        if names_under(cell, &proc_def.qualified_name) {
            record(
                proc_def.name_span,
                &proc_def.qualified_name,
                &mut considered,
            );
        }
    }
    for class_def in analysis.all_classes.values() {
        if names_under(cell, &class_def.qualified_name) {
            record(
                class_def.name_span,
                &class_def.qualified_name,
                &mut considered,
            );
        }
    }
    // 5. Import / forget patterns naming it.  `namespace export` patterns are
    //    never qualified — a `::`-qualified export is a hard error in real
    //    Tcl (`invalid export pattern "::foo": pattern can't specify a
    //    namespace`) — so they name no other namespace and need no edit.
    for imp in &analysis.namespace_imports {
        if names_under(cell, &imp.pattern) {
            record(imp.range, &imp.pattern, &mut considered);
        }
    }
    for forget in &analysis.namespace_forgets {
        let Some(source_ns) = forget.source_ns.as_deref() else {
            continue;
        };
        if !names_at_or_under(cell, source_ns) {
            continue;
        }
        // The recorded span covers the whole pattern word (`::old::p`), and
        // the pattern's own namespace is what names the renamed one.
        let resolved = format!("{}::{}", source_ns.trim_end_matches("::"), forget.pattern);
        record(forget.range, &resolved, &mut considered);
    }

    if let Some(refusal) =
        namespace_rename_hazard(source, dialect, analysis, cell, &considered, &line_index)
    {
        return Err(refusal);
    }

    let mut edits: Vec<TextEdit> = spans
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|span| TextEdit {
            range: span_to_range(source, &line_index, span),
            new_text: new_tail.to_owned(),
        })
        .collect();
    edits.sort_by_key(|e| {
        (
            e.range.start_line,
            e.range.start_character,
            e.range.end_line,
            e.range.end_character,
        )
    });
    edits.dedup_by(|a, b| a.range == b.range);
    Ok(edits)
}

/// The byte span of `cell`'s final segment inside the namespace-name word the
/// cursor at `cursor` sits in — the range `prepare_rename` offers and the
/// rename rewrites.
///
/// `None` when the word does not spell that segment: inside `namespace eval
/// ::app::old`, the word of a nested `namespace eval sub` writes no `old`, so
/// there is nothing there for a rename of `::app::old` to anchor on.
#[must_use]
pub fn namespace_segment_at(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    cell: &str,
) -> Option<Span> {
    let hit = analysis
        .namespace_refs
        .iter()
        .find(|r| r.span.start() <= cursor && cursor < r.span.end())?;
    written_cell_segment(source, hit.span, &hit.qualified_name, cell)
}

/// Whether `candidate` is the namespace `cell` itself or one beneath it.
fn names_at_or_under(cell: &str, candidate: &str) -> bool {
    let cell = cell.trim_start_matches("::");
    let candidate = candidate.trim_start_matches("::");
    candidate == cell || crate::namespace_symbol::namespace_strictly_contains(cell, candidate)
}

/// Whether the qualified *name* `candidate` (a proc / variable / class, not a
/// namespace) lives beneath `cell`.
fn names_under(cell: &str, candidate: &str) -> bool {
    crate::namespace_symbol::namespace_strictly_contains(cell, candidate)
}

/// The byte span, inside the name word at `word`, of the segment that spells
/// the **final segment of `cell`** — the one byte range this rename rewrites.
///
/// `resolved` is the fully-qualified name the word denotes, which is what
/// says *how many* leading segments the word leaves unwritten: `namespace
/// eval sub {}` inside `::old` writes one segment of `::old::sub`, so `old`
/// is not written there and the answer is `None`.  A word that does not spell
/// the namespace needs no edit — after the enclosing block's own name changes
/// it still resolves to the same thing.
///
/// Variable decoration (`$name`, `${name}`) and an array-element suffix are
/// stripped first: they surround the name, they are not part of it.
#[must_use]
fn written_cell_segment(source: &str, word: Span, resolved: &str, cell: &str) -> Option<Span> {
    let start = word.start() as usize;
    let end = word.end() as usize;
    if start > end || end > source.len() {
        return None;
    }
    let raw = &source[start..end];
    // Leading `$` / `${` decoration, and the matching `}` / element suffix.
    let (offset, text) = if let Some(rest) = raw.strip_prefix("${") {
        (2usize, rest.strip_suffix('}').unwrap_or(rest))
    } else if let Some(rest) = raw.strip_prefix('$') {
        (1usize, rest)
    } else {
        (0usize, raw)
    };
    let text = text.split_once('(').map_or(text, |(base, _)| base);
    // A word whose name is computed spells nothing statically; the gate
    // decides what to do about it.
    if text.contains(['$', '[']) {
        return None;
    }
    let cell_body = cell.trim_start_matches("::");
    let cell_depth = cell_body.split("::").count();
    let resolved_depth = resolved.trim_start_matches("::").split("::").count();
    let rooted = text.starts_with("::");
    let written = text.trim_start_matches("::");
    if written.is_empty() {
        return None;
    }
    let written_segments: Vec<&str> = written.split("::").collect();
    // How many leading segments of `resolved` this word does not spell.
    let unwritten = resolved_depth.checked_sub(written_segments.len())?;
    // The namespace's final segment is `cell_depth - 1` segments in.
    let index = (cell_depth - 1).checked_sub(unwritten)?;
    let segment = written_segments.get(index)?;
    // Spelling check: the word must really write this segment.  A word that
    // reaches the name some other way (an alias, an ensemble spelling) is not
    // a namespace occurrence and must not be rewritten.
    if *segment != cell_body.rsplit("::").next().unwrap_or(cell_body) {
        return None;
    }
    let mut at = offset + usize::from(rooted) * 2;
    for prior in written_segments.iter().take(index) {
        at += prior.len() + 2;
    }
    let seg_start = u32::try_from(start + at).ok()?;
    let seg_end = u32::try_from(start + at + segment.len()).ok()?;
    Some(Span::new(seg_start, seg_end))
}

/// Why this document makes the namespace rename unprovable, if it does.
///
/// `considered` is the exact set of word spans the edit collector looked at,
/// so "everything else" is what the gate examines — the coverage-equals-edits
/// discipline #1092 established for the member tier, applied here.
fn namespace_rename_hazard(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    cell: &str,
    considered: &[Span],
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    // A `namespace path` word the analyser could not read as a literal list —
    // `namespace path $paths`, `namespace path "$ns ::a"`.  Its entries are
    // whatever the value turns out to be, which may include this namespace,
    // and there is no element word to rewrite.  A *literal* list needs no
    // arm here: each of its elements is recorded as a `NamespaceRef` at its
    // own span, so the edit collector rewrites the ones naming the renamed
    // cell like any other spelling (issue #1261).
    if let Some(&span) = analysis.namespace_path_computed.first() {
        let written = source
            .get(span.start() as usize..span.end() as usize)
            .unwrap_or("");
        return Some(RenameRefusal::at(
            format!(
                "cannot rename `{cell}`: a `namespace path` is built from a value computed \
                 at run time (`{written}`), whose entries may name this very namespace — \
                 there is no element word to rewrite, so the path could keep naming a \
                 namespace that no longer exists."
            ),
            source,
            line_index,
            Some(span),
        ));
    }
    let registry = tcl_registry::registry_for_dialect(dialect);
    let mut hazard: Option<(Span, HazardKind)> = None;
    let mut visit = |cmd: &tcl_compiler::segmenter::SegmentedCommand| {
        if hazard.is_some() {
            return;
        }
        // A word carrying a script is walked as code in its own right, so its
        // text must not be read as a name here — a body word contains every
        // name written inside it.
        let nested = crate::references::dispatch_scan_regions(source, dialect, cmd);
        let Some(cmd_name) = cmd.texts.first() else {
            return;
        };
        let args: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
        // A computed namespace word: `namespace eval $ns { … }`.  The
        // analyser records such a word only when its value is
        // constant-dominated, so anything left here names a namespace nothing
        // proves is not this one.
        for idx in
            registry.arg_indices_for_role(cmd_name, &args, tcl_registry::ArgRole::NamespaceName)
        {
            let Some(word) = args.get(idx) else { continue };
            if !word.contains(['$', '[']) {
                continue;
            }
            if !tcl_compiler::dynamic_names::dynamic_variable_word_can_spell(word, cell) {
                continue;
            }
            if let Some(tok) = cmd.argv.get(idx + 1) {
                hazard = Some((tok.span, HazardKind::Computed));
                return;
            }
        }
        for tok in &cmd.argv {
            let span = tok.span;
            if considered.iter().any(|c| spans_overlap(*c, span)) {
                continue;
            }
            if nested
                .iter()
                .any(|(s, e)| (span.start() as usize) < *e && *s < span.end() as usize)
            {
                continue;
            }
            let Some(text) = source.get(span.start() as usize..span.end() as usize) else {
                continue;
            };
            if embedded_rooted_name_under(text, cell) {
                hazard = Some((span, HazardKind::Unattributed));
                return;
            }
        }
    };
    crate::rename_safety::walk_document(source, dialect, analysis, &mut visit);
    let (span, kind) = hazard?;
    let written = source
        .get(span.start() as usize..span.end() as usize)
        .unwrap_or("");
    let reason = match kind {
        HazardKind::Computed => format!(
            "cannot rename `{cell}`: this document names a namespace through a value \
             computed at run time (`{written}`), which may be this very namespace — there \
             is no word to rewrite, so the rename cannot be completed from source edits."
        ),
        HazardKind::Unattributed => format!(
            "cannot rename `{cell}`: `{written}` names something beneath it in a position \
             this rename cannot attribute — a callback script, a dispatch-table literal, or \
             another embedded name. Rewriting the declarations would leave it naming a \
             namespace that no longer exists."
        ),
    };
    Some(RenameRefusal::at(reason, source, line_index, Some(span)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HazardKind {
    Computed,
    Unattributed,
}

fn spans_overlap(a: Span, b: Span) -> bool {
    a.start() < b.end() && b.start() < a.end()
}

/// Whether `text` embeds a `::`-rooted name beneath `cell` — the shape a
/// callback string or dispatch-table literal carries.
///
/// The word is split on the characters that separate words inside a script or
/// list literal, so `{::old::tick $arg}` is examined token by token rather
/// than as one string.  Rooted spellings only; see the module docs' bounds.
fn embedded_rooted_name_under(text: &str, cell: &str) -> bool {
    text.split(|c: char| c.is_whitespace() || "{}[];\"".contains(c))
        .map(|token| token.trim_start_matches(['$', '{']).trim_end_matches('}'))
        .filter(|token| token.starts_with("::"))
        .any(|token| {
            let token = token.split_once('(').map_or(token, |(base, _)| base);
            names_at_or_under(cell, token) || names_under(cell, token)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    /// The document rewritten by applying the tier's edits, or the refusal
    /// reason.
    fn renamed(source: &str, cell: &str, new_tail: &str) -> Result<String, String> {
        let analysis = analyse(source);
        let edits = namespace_rename_edits(source, "tcl8.6", &analysis, cell, new_tail)
            .map_err(|r| r.reason)?;
        // Apply back-to-front so earlier spans keep their offsets.
        let line_index = LineIndex::new(source);
        let mut offsets: Vec<(usize, usize, String)> = edits
            .iter()
            .map(|e| {
                let start = crate::definition::byte_offset_at(
                    &line_index,
                    source,
                    e.range.start_line,
                    e.range.start_character,
                ) as usize;
                let end = crate::definition::byte_offset_at(
                    &line_index,
                    source,
                    e.range.end_line,
                    e.range.end_character,
                ) as usize;
                (start, end, e.new_text.clone())
            })
            .collect();
        offsets.sort_by_key(|(start, _, _)| *start);
        let mut out = source.to_owned();
        for (start, end, text) in offsets.into_iter().rev() {
            out.replace_range(start..end, &text);
        }
        Ok(out)
    }

    /// TP — the tier's core claim: every written spelling of the namespace
    /// moves, and nothing else does.
    ///
    /// tclsh-proof (8.6.14) that the before/after programs agree: the source
    /// below prints `1 P`, and the same script with `old` spelled `new`
    /// throughout prints `1 P` too.
    #[test]
    fn tp_rewrites_every_written_spelling_of_the_namespace() {
        let src = "namespace eval ::old {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} { return P }\n\
                   }\n\
                   namespace eval ::old { proc q {} { return Q } }\n\
                   puts $::old::v\n\
                   puts [::old::p]\n\
                   namespace children ::old\n";
        let out = renamed(src, "::old", "new").expect("a clean document must rename");
        assert!(!out.contains("::old"), "no spelling may be left: {out}");
        assert_eq!(out.matches("::new").count(), 5, "{out}");
        // The unqualified names inside the block are untouched — they still
        // resolve to the same cells after the block's own name changes.
        assert!(
            out.contains("variable v 1") && out.contains("proc p {}"),
            "{out}"
        );
    }

    /// TP — a **relative** spelling written inside an enclosing block moves
    /// too, and only its own segment does.
    #[test]
    fn tp_rewrites_a_relatively_written_segment_only() {
        let src = "namespace eval ::app {\n\
                   \x20   namespace eval old { proc p {} { return P } }\n\
                   \x20   proc use {} { return [old::p] }\n\
                   }\n";
        let out = renamed(src, "::app::old", "new").expect("rename");
        assert!(out.contains("namespace eval new {"), "{out}");
        assert!(out.contains("[new::p]"), "{out}");
        assert!(
            out.contains("namespace eval ::app {"),
            "the parent is untouched: {out}"
        );
    }

    /// TP — a deeper block is how an **implicitly created** namespace is
    /// renamed at all: it has no declaring word of its own, so the edit is the
    /// covering segment of the deeper name (issue #1113 item 1).
    ///
    /// tclsh-proof (8.6.14): `namespace eval ::p::q::r {}` leaves `namespace
    /// exists ::p::q` -> 1, so `::p::q` is a real namespace with no block.
    #[test]
    fn tp_renames_an_implicitly_created_namespace_through_its_deeper_block() {
        let src = "namespace eval ::p::q::r { proc f {} {} }\nputs [::p::q::r::f]\n";
        let out = renamed(src, "::p::q", "z").expect("rename");
        assert!(out.contains("namespace eval ::p::z::r"), "{out}");
        assert!(out.contains("[::p::z::r::f]"), "{out}");
    }

    /// TN — a name that merely *shares the tail* is a different namespace and
    /// must not move.
    #[test]
    fn tn_a_same_tailed_sibling_namespace_is_untouched() {
        let src = "namespace eval ::a::old { proc p {} {} }\n\
                   namespace eval ::b::old { proc p {} {} }\n\
                   puts [::b::old::p]\n";
        let out = renamed(src, "::a::old", "new").expect("rename");
        assert!(out.contains("namespace eval ::a::new"), "{out}");
        assert!(out.contains("namespace eval ::b::old"), "{out}");
        assert!(out.contains("[::b::old::p]"), "{out}");
    }

    /// TP (refusal) — a computed namespace word may be this very namespace and
    /// has no word to rewrite.
    #[test]
    fn tp_refuses_a_computed_namespace_target() {
        let src = "namespace eval ::old { variable v 1 }\n\
                   proc mk {ns} { namespace eval $ns { variable w 2 } }\n";
        let reason = renamed(src, "::old", "new").expect_err("a computed target must refuse");
        assert!(reason.contains("computed at run time"), "{reason}");
    }

    /// TN — a computed namespace word that provably cannot spell this
    /// namespace does not refuse (the #1093 provenance rule, applied to
    /// namespace words).
    #[test]
    fn tn_a_computed_namespace_word_elsewhere_does_not_refuse() {
        let src = "namespace eval ::old { variable v 1 }\n\
                   proc mk {ns} { namespace eval ::other::$ns { variable w 2 } }\n";
        assert!(
            renamed(src, "::old", "new").is_ok(),
            "`::other::$ns` cannot name `::old`",
        );
    }

    /// TP — a callback script the registry marks as a *body* is walked as
    /// code, so the name inside it is an attributed call site and moves with
    /// the namespace rather than refusing.
    #[test]
    fn tp_rewrites_a_name_inside_a_walked_callback_body() {
        let src = "namespace eval ::old { proc tick {} {} }\n\
                   after 0 {::old::tick}\n";
        let out = renamed(src, "::old", "new").expect("a walked body is attributed");
        assert!(out.contains("after 0 {::new::tick}"), "{out}");
    }

    /// TP (refusal) — a rooted name beneath the namespace sitting in a plain
    /// data word is exactly what an edit set would leave behind: nothing
    /// attributes it, and it may well be `eval`'d or dispatched later.
    #[test]
    fn tp_refuses_an_unattributed_embedded_name() {
        let src = "namespace eval ::old { proc tick {} {} }\n\
                   set handlers [list ::old::tick]\n";
        let reason = renamed(src, "::old", "new").expect_err("an embedded name must refuse");
        assert!(reason.contains("cannot attribute"), "{reason}");
    }

    /// TN control for the pair above: a data word naming something in a
    /// *different* namespace is not this rename's problem.
    #[test]
    fn tn_an_unattributed_name_elsewhere_does_not_refuse() {
        let src = "namespace eval ::old { proc tick {} {} }\n\
                   set handlers [list ::other::tick]\n";
        assert!(renamed(src, "::old", "new").is_ok());
    }

    /// TP, issue #1261 — a literal `namespace path` entry naming the
    /// namespace is *rewritten*, not refused: each element carries its own
    /// span, so the tier edits the element's segment like any other spelling.
    ///
    /// tclsh-proof (8.6.14) that the before/after programs agree: the source
    /// below prints `P`, and the same script with `old` spelled `new`
    /// throughout prints `P` too — while leaving the entry behind fails with
    /// `namespace "::old" not found in "::"`.
    #[test]
    fn tp_rewrites_a_namespace_path_entry() {
        let src = "namespace eval ::old { proc p {} { return P } ; namespace export p }\n\
                   namespace eval ::user { namespace path {::old}\n\
                   \x20   proc use {} { return [p] } }\n\
                   puts [::user::use]\n";
        let out = renamed(src, "::old", "new").expect("a literal path entry is rewritable");
        assert!(out.contains("namespace path {::new}"), "{out}");
        assert!(out.contains("namespace eval ::new {"), "{out}");
    }

    /// TP — only the entries naming the renamed namespace move; siblings in
    /// the same list word are left exactly as written.
    #[test]
    fn tp_rewrites_only_the_matching_namespace_path_entries() {
        let src = "namespace eval ::old { proc p {} {} }\n\
                   namespace eval ::keep { proc q {} {} }\n\
                   namespace eval ::user { namespace path {::keep ::old::deep ::other} }\n";
        let out = renamed(src, "::old", "new").expect("rename");
        assert!(
            out.contains("namespace path {::keep ::new::deep ::other}"),
            "{out}"
        );
    }

    /// TP — a **relative** entry roots against the namespace the `namespace
    /// path` runs in, and only the segment that spells the renamed namespace
    /// moves.
    ///
    /// tclsh-proof (8.6.14): inside `namespace eval ::app`, `namespace path
    /// {old}` echoes `::app::old` — the entry roots against the namespace the
    /// command runs in, current-namespace-only (the same block written one
    /// level deeper, in `::app::user`, fails `namespace "old" not found in
    /// "::app::user"`).  So a rename of `::app::old` must rewrite the bare
    /// `old` word.
    #[test]
    fn tp_rewrites_a_relatively_written_namespace_path_entry() {
        let src = "namespace eval ::app {\n\
                   \x20   namespace eval old { proc p {} {} }\n\
                   \x20   namespace path {old}\n\
                   }\n";
        let out = renamed(src, "::app::old", "new").expect("rename");
        assert!(out.contains("namespace path {new}"), "{out}");
    }

    /// TN — a path list that names nothing beneath the renamed namespace is
    /// untouched and does not refuse.
    #[test]
    fn tn_an_unrelated_namespace_path_neither_moves_nor_refuses() {
        let src = "namespace eval ::old { proc p {} {} }\n\
                   namespace eval ::user { namespace path {::a ::b} }\n";
        let out = renamed(src, "::old", "new").expect("an unrelated path must not refuse");
        assert!(out.contains("namespace path {::a ::b}"), "{out}");
    }

    /// TP (refusal), issue #1261 — a `namespace path` built from a value
    /// computed at run time still refuses: its entries are unknowable, any of
    /// them may be this namespace, and there is no element word to rewrite.
    #[test]
    fn tp_refuses_a_computed_namespace_path() {
        for src in [
            "namespace eval ::old { proc p {} {} }\n\
             namespace eval ::user { namespace path $paths }\n",
            "namespace eval ::old { proc p {} {} }\n\
             namespace eval ::user { namespace path \"$ns ::a\" }\n",
        ] {
            let reason = renamed(src, "::old", "new").expect_err("a computed path must refuse");
            assert!(reason.contains("namespace path"), "{reason}");
            assert!(reason.contains("computed at run time"), "{reason}");
        }
    }

    /// TN — a **braced** `$`-spelled entry is not computed: braces suppress
    /// substitution, so the word is a literal path list and the tier neither
    /// refuses nor rewrites it.
    ///
    /// tclsh-proof (8.6.14): `namespace eval n { namespace path {::$ns ::a} }`
    /// (with a namespace literally named `::$ns`) echoes `{::$ns} ::a` — the
    /// entry is a name, never a variable read.
    #[test]
    fn tn_a_braced_dollar_entry_is_a_literal_path() {
        let src = "namespace eval ::old { proc p {} {} }\n\
                   namespace eval ::user { namespace path {::a ::b} }\n";
        let analysis = analyse(src);
        assert!(
            analysis.namespace_path_computed.is_empty(),
            "a braced literal list records no abstention",
        );
        assert!(renamed(src, "::old", "new").is_ok());
    }

    /// TP — import patterns follow the namespace.
    #[test]
    fn tp_rewrites_an_import_pattern() {
        let src = "namespace eval ::old { proc p {} {} ; namespace export p }\n\
                   namespace eval ::user { namespace import ::old::p }\n";
        let out = renamed(src, "::old", "new").expect("rename");
        assert!(out.contains("namespace import ::new::p"), "{out}");
        // The export pattern is unqualified and names no namespace, so it is
        // untouched — a `::`-qualified export is a hard error in real Tcl.
        assert!(out.contains("namespace export p"), "{out}");
    }

    /// The segment finder is the one place that decides which byte range a
    /// written name gives to the rename.
    #[test]
    fn written_segment_is_the_namespace_segment_only() {
        let src = "set ::app::old::v 1\n";
        let at = u32::try_from(src.find("::app").expect("word")).expect("offset fits");
        let len = u32::try_from("::app::old::v".len()).expect("length fits");
        let span =
            written_cell_segment(src, Span::new(at, at + len), "::app::old::v", "::app::old")
                .expect("segment");
        assert_eq!(
            &src[span.start() as usize..span.end() as usize],
            "old",
            "only the namespace's own segment is rewritten",
        );
        // A word that does not spell the segment yields nothing to rewrite —
        // a bare `v` inside the namespace still resolves after the rename.
        assert!(
            written_cell_segment("v\n", Span::new(0, 1), "::app::old::v", "::app::old").is_none()
        );
    }
}
