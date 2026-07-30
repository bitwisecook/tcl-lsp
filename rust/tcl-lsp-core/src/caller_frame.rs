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

//! Navigation for **caller-frame variables** — the ones a callee creates in
//! *this* frame through `upvar`, which the frame's own text never assigns.
//!
//! ```tcl
//! proc gridlayoutHasDataSetObj {dts} { upvar 1 $dts dataset; set dataset … }
//! # …
//! gridlayoutHasDataSetObj dataset      ;# creates `dataset` HERE
//! set _dataset $dataset                ;# and this reads it
//! ```
//!
//! Nothing in the caller writes `dataset`, so the ordinary scope-chain walk
//! finds no [`VarDef`](tcl_compiler::analyser::VarDef) for the `$dataset`
//! read.  Before this module the providers then *fell through* to bareword
//! resolution and answered with a coincidentally same-named `TclOO` **method**
//! — a wrong-kind conflation, since Tcl's variable and command namespaces are
//! disjoint and a `$`-led token can never denote a method (issue #923 audit
//! idx 58).  Two fixes follow from that, and both live here:
//!
//! * **Abstain.** [`substituted_var_read_at`] answers "the cursor is on a
//!   `$name` Tcl really substitutes", so hover / find-references can stop
//!   rather than fall through.  It is deliberately independent of whether a
//!   `VarDef` resolved: the *token kind* is what forbids the fallback.
//! * **Resolve.** [`caller_frame_bindings`] finds the call sites in the
//!   enclosing frame that create the name, so hover, go-to-definition, and
//!   find-references can answer for real.
//!
//! # What the call site has to say
//!
//! The fact this module consumes is the analyser's per-parameter
//! [`ProcArgTrait::VarWrite`] / [`ProcArgTrait::VarRead`] — inferred from the
//! callee's own `upvar ?level? $param local` and shared with the
//! [frame-effect summary](tcl_compiler::cfg_builder::upvar_info)'s
//! `param_targets` bucket.  A parameter carrying either trait means *the
//! caller's word at that position names a variable in the caller's frame*,
//! which is exactly the question navigation asks.  No command name appears
//! here: which words name variables is registry- and analyser-derived.
//!
//! C Tcl, pinned on tclsh 9.0.4 and 8.6.14 (identical):
//!
//! ```tcl
//! proc setdef {d} { upvar 1 $d dst; set dst SET }
//! proc build {} { setdef options; return $options }
//! build            ;# → SET — `options` exists in build's frame, unassigned there
//! ```
//!
//! # What it deliberately does not answer
//!
//! A callee that binds a **literal** caller-side name (`upvar 1 name name`,
//! issue #923 audit idx 22/98) spells that name nowhere at the call site, so
//! there is no argument word to key on and no span to navigate to.  Resolving
//! it needs a per-proc *literal caller-frame target* fact on
//! [`ProcDef`](tcl_compiler::analyser::ProcDef) — the summary's
//! `literal_targets` bucket — which the analyser does not yet record.  Those
//! reads keep the abstaining answer above rather than a wrong one.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::ProcArgTrait;
use tcl_lexer::Span;

/// The name of the `$name` read the cursor sits on, when that occurrence is
/// one Tcl actually substitutes.
///
/// The `VarDef`-resolving twin is
/// [`crate::definition::lookup_var_read_at`]; this answers the *token kind*
/// alone, which is what a provider needs to decide whether falling through to
/// bareword (command / class-member) resolution is legitimate.  It never is
/// for a `$`-led read: Tcl keeps variables and commands in disjoint
/// namespaces, so `$dataset` can only ever be the variable, never a method
/// called `dataset` (issue #923 audit idx 58).
#[must_use]
pub(crate) fn substituted_var_read_at(
    source: &str,
    dialect: &str,
    line: u32,
    character: u32,
    cursor_off: u32,
) -> Option<String> {
    let name = crate::hover::find_var_at_position(source, line, character)?;
    let inert = crate::inert_text::offset_in_comment(source, cursor_off)
        || crate::inert_text::offset_in_data_brace(
            source,
            cursor_off,
            tcl_registry::registry_for_dialect(dialect),
            dialect,
        );
    (!inert).then_some(name)
}

/// One call site in the current frame that creates a caller-frame variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallerFrameBinding {
    /// Qualified name of the callee whose `upvar` creates the variable.
    pub callee: String,
    /// The callee parameter whose *value* names the variable.
    pub param: String,
    /// Span of the call-site word that names it — `dataset` in
    /// `gridlayoutHasDataSetObj dataset`.  This is both the creating write
    /// (go-to-definition's target) and a reference to the variable.
    pub arg_span: Span,
    /// Span of the call's command-head word.
    pub call_span: Span,
    /// True when the callee only *reads* through the alias
    /// ([`ProcArgTrait::VarRead`] without
    /// [`ProcArgTrait::VarWrite`]) — the site references the variable but
    /// does not create it.
    pub read_only: bool,
}

/// Byte region of the innermost scope body containing `off`, or the whole
/// document when the cursor is at top level.
///
/// A caller-frame binding is created by a call in the *same frame* as the
/// read, so the search never crosses a proc/method body boundary.
fn enclosing_frame_region(
    global: &tcl_compiler::analyser::Scope,
    off: u32,
    source: &str,
) -> (usize, usize) {
    fn walk(
        scope: &tcl_compiler::analyser::Scope,
        off: u32,
        best: &mut Option<Span>,
    ) -> Option<()> {
        for child in &scope.children {
            if let Some(span) = child.body_span
                && off >= span.start()
                && off <= span.end()
            {
                let better = best.is_none_or(|b: Span| {
                    span.end().saturating_sub(span.start()) < b.end().saturating_sub(b.start())
                });
                if better {
                    *best = Some(span);
                }
            }
            walk(child, off, best)?;
        }
        Some(())
    }
    let mut best = None;
    walk(global, off, &mut best);
    let source_len = source.len();
    let (mut start, mut end) = match best {
        Some(span) => (
            (span.start() as usize).min(source_len),
            (span.end() as usize).min(source_len),
        ),
        None => (0, source_len),
    };
    // A scope's recorded body span can still carry the body word's own
    // delimiters. Left in, the segmenter reads the whole frame as a single
    // braced *word* and finds no commands in it at all, so every lookup here
    // would silently answer "nothing binds this name".
    let bytes = source.as_bytes();
    if start < end && bytes.get(start) == Some(&b'{') {
        start += 1;
    }
    if end > start && bytes.get(end - 1) == Some(&b'}') {
        end -= 1;
    }
    (start, end)
}

/// Whether a call-site word is a plain variable *name* — the only shape whose
/// caller-frame target is knowable.
///
/// A substituted (`$x`) or computed (`[pick]`) word names a variable this
/// analysis cannot identify, and an empty one names none; both abstain rather
/// than guess, the same direction the frame-effect summary takes for its own
/// unresolvable targets.
fn is_plain_var_name(word: &str) -> bool {
    !word.is_empty()
        && !word.contains(['$', '[', '{', '"', ' '])
        && word
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
}

/// Whether any procedure in the document takes a parameter whose value names
/// a caller-frame variable — the cheap pre-filter for
/// [`caller_frame_bindings`]'s source scan.
fn document_has_call_by_name_proc(analysis: &AnalysisResult) -> bool {
    analysis.all_procs.values().any(|proc_def| {
        proc_def.param_traits.values().any(|traits| {
            traits.contains(&ProcArgTrait::VarWrite) || traits.contains(&ProcArgTrait::VarRead)
        })
    })
}

/// Every call in the frame enclosing `cursor_off` that binds `name` in that
/// frame through a callee's `upvar`.
///
/// Returned in source order.  Empty when nothing binds the name — the caller
/// then keeps abstaining rather than falling through.
#[must_use]
pub(crate) fn caller_frame_bindings(
    analysis: &AnalysisResult,
    source: &str,
    dialect: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    cursor_off: u32,
    name: &str,
) -> Vec<CallerFrameBinding> {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;

    let mut out: Vec<CallerFrameBinding> = Vec::new();
    // This runs on every hover / go-to-definition / find-references that fails
    // the ordinary scope-chain lookup, and re-segmenting the enclosing frame
    // is the expensive part.  A document with no call-by-name procedure at all
    // — the overwhelming majority — can answer from the already-computed trait
    // maps without touching the source.
    if name.is_empty() || !document_has_call_by_name_proc(analysis) {
        return out;
    }
    let (start, end) = enclosing_frame_region(&analysis.global_scope, cursor_off, source);
    if start >= end {
        return out;
    }
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    let commands = segment_commands_with_offset_and_config(
        &source[start..end],
        u32::try_from(start).unwrap_or(0),
        tcl_lexer::LexerConfig::for_dialect(dialect),
    );
    for cmd in &commands {
        let Some(head) = cmd.argv.first() else {
            continue;
        };
        let Some(head_text) = cmd.texts.first() else {
            continue;
        };
        let Some(proc_def) = crate::definition::resolve_called_proc(
            analysis, source, &namespace, head_text, registry,
        ) else {
            continue;
        };
        if proc_def.param_traits.is_empty() {
            continue;
        }
        for (i, param) in proc_def.params.iter().enumerate() {
            let Some(arg_tok) = cmd.argv.get(i + 1) else {
                break;
            };
            let Some(arg_text) = cmd.texts.get(i + 1) else {
                break;
            };
            if arg_text != name || !is_plain_var_name(arg_text) {
                continue;
            }
            let Some(traits) = proc_def.param_traits.get(&param.name) else {
                continue;
            };
            let writes = traits.contains(&ProcArgTrait::VarWrite);
            let reads = traits.contains(&ProcArgTrait::VarRead);
            if !writes && !reads {
                continue;
            }
            out.push(CallerFrameBinding {
                callee: proc_def.qualified_name.clone(),
                param: param.name.clone(),
                arg_span: arg_tok.span,
                call_span: head.span,
                read_only: !writes,
            });
        }
    }
    out.sort_by_key(|b| b.arg_span.start());
    out
}

/// Every span in the enclosing frame that refers to the caller-frame variable
/// `name`: each call-site word that names it, plus each `$name` read.
///
/// Both halves are the point of the idiom — the caller writes the name once,
/// bare, at the call site and then reads it with a `$`, so a reference set
/// that showed only one of the two would miss what the user is looking for.
/// Returns empty when no call site binds the name, so a provider that gets
/// nothing here keeps abstaining.
#[must_use]
pub(crate) fn caller_frame_reference_spans(
    analysis: &AnalysisResult,
    source: &str,
    dialect: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    cursor_off: u32,
    name: &str,
) -> Vec<Span> {
    let bindings = caller_frame_bindings(analysis, source, dialect, registry, cursor_off, name);
    if bindings.is_empty() {
        return Vec::new();
    }
    let (start, end) = enclosing_frame_region(&analysis.global_scope, cursor_off, source);
    let mut spans: Vec<Span> = bindings.iter().map(|b| b.arg_span).collect();
    spans.extend(substituted_read_spans(source, dialect, start, end, name));
    spans.sort_by_key(|span: &Span| span.start());
    spans.dedup();
    spans
}

/// Spans of every `$name` / `${name}` occurrence in `source[start..end]` that
/// Tcl actually substitutes.
///
/// The inertness proofs ([`crate::inert_text`]) are the same ones
/// [`crate::definition::lookup_var_read_at`] applies, so a `$name`-shaped run
/// inside a comment or a brace-quoted data word is not counted here either
/// (issue #923 audit idx 24).
fn substituted_read_spans(
    source: &str,
    dialect: &str,
    start: usize,
    end: usize,
    name: &str,
) -> Vec<Span> {
    let mut out = Vec::new();
    if start >= end || end > source.len() {
        return out;
    }
    let bytes = source.as_bytes();
    for (form, offset_to_name) in [(format!("${name}"), 1), (format!("${{{name}}}"), 2)] {
        let mut from = start;
        while let Some(rel) = source[from..end].find(&form) {
            let at = from + rel;
            from = at + 1;
            let after = at + form.len();
            // A bare `$name` must not be the prefix of a longer name; the
            // braced form is already delimited by its own `}`.
            if offset_to_name == 1
                && bytes
                    .get(after)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b':')
            {
                continue;
            }
            let name_start = u32::try_from(at + offset_to_name).unwrap_or(u32::MAX);
            let inert = crate::inert_text::offset_in_comment(source, name_start)
                || crate::inert_text::offset_in_data_brace(
                    source,
                    name_start,
                    tcl_registry::registry_for_dialect(dialect),
                    dialect,
                );
            if inert {
                continue;
            }
            out.push(Span::new(
                name_start,
                name_start + u32::try_from(name.len()).unwrap_or(0),
            ));
        }
    }
    out
}

/// The caller-frame variable the cursor's **bare** word names, when that word
/// is a call-site argument binding one.
///
/// The bareword half of the idiom — `gridlayoutHasDataSetObj dataset` — is a
/// creating write, so hover, go-to-definition, and find-references must all
/// answer for it exactly as they do for the `$dataset` reads it feeds.
#[must_use]
pub(crate) fn binding_at_offset(
    analysis: &AnalysisResult,
    source: &str,
    dialect: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
    cursor_off: u32,
    word: &str,
) -> Option<CallerFrameBinding> {
    caller_frame_bindings(analysis, source, dialect, registry, cursor_off, word)
        .into_iter()
        .find(|b| cursor_off >= b.arg_span.start() && cursor_off <= b.arg_span.end())
}

/// The binding a caller-frame *read* resolves to — the first call site that
/// **creates** the variable, falling back to the first that references it.
///
/// Go-to-definition and hover both want the creating write: it is the nearest
/// thing the frame has to a declaration, and it is the word the user would
/// rename.
#[must_use]
pub(crate) fn primary_binding(bindings: &[CallerFrameBinding]) -> Option<&CallerFrameBinding> {
    bindings
        .iter()
        .find(|b| !b.read_only)
        .or_else(|| bindings.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    /// The idx-58 shape, minimised: a call-by-name helper, a `TclOO` class with
    /// an accessor **method** of the same name as the caller-frame variable,
    /// and a constructor that reads the variable it never assigns.
    ///
    /// tclsh 9.0.4 / 8.6.14 (identical) run the real thing to completion —
    /// `dataset` is a live variable created purely by the callee's `upvar`.
    const IDX58: &str = "\
proc gridlayoutHasDataSetObj {dts} {
    upvar 1 $dts dataset
    set dataset MY-SHARED-DATASET
}
oo::class create chart {
    constructor {} {
        gridlayoutHasDataSetObj dataset
        set _dataset $dataset
    }
    method dataset {} { return 1 }
}
";

    fn analyse(source: &str) -> AnalysisResult {
        Analyser::new().analyse(source, "tcl9.0").clone()
    }

    fn reg() -> &'static tcl_registry::CommandRegistry {
        tcl_registry::registry_for_dialect("tcl9.0")
    }

    fn offset_of(source: &str, needle: &str) -> u32 {
        u32::try_from(source.find(needle).expect("needle present")).expect("offset fits u32")
    }

    #[test]
    fn call_site_argument_binds_the_caller_frame_variable() {
        let analysis = analyse(IDX58);
        let read = offset_of(IDX58, "$dataset") + 1;
        let bindings =
            caller_frame_bindings(&analysis, IDX58, "tcl9.0", Some(reg()), read, "dataset");
        assert_eq!(bindings.len(), 1, "one binding call site: {bindings:?}");
        assert_eq!(bindings[0].param, "dts");
        assert!(!bindings[0].read_only);
        assert_eq!(
            &IDX58[bindings[0].arg_span.as_range()],
            "dataset",
            "the binding span must cover the call-site word"
        );
    }

    /// TN — a call whose callee does *not* alias the argument binds nothing,
    /// so navigation keeps abstaining instead of inventing a variable.
    #[test]
    fn a_plain_value_argument_binds_nothing() {
        let src = "proc plain {v} { return $v }\nproc caller {} { plain thing\n puts $thing }\n";
        let analysis = analyse(src);
        let read = offset_of(src, "$thing") + 1;
        assert!(
            caller_frame_bindings(&analysis, src, "tcl9.0", Some(reg()), read, "thing").is_empty()
        );
    }

    /// TN — a substituted call-site word names a variable this analysis
    /// cannot identify, so it contributes no binding.
    #[test]
    fn a_substituted_call_site_word_binds_nothing() {
        let src = "proc setdef {d} { upvar 1 $d dst; set dst 1 }\n\
                   proc caller {n} { setdef $n\n puts $thing }\n";
        let analysis = analyse(src);
        let read = offset_of(src, "$thing") + 1;
        assert!(
            caller_frame_bindings(&analysis, src, "tcl9.0", Some(reg()), read, "thing").is_empty()
        );
    }

    /// The search never leaves the frame: a binding call in a *different*
    /// proc creates the variable in that proc's frame, not in this one.
    #[test]
    fn a_binding_in_another_frame_is_not_visible_here() {
        let src = "proc setdef {d} { upvar 1 $d dst; set dst 1 }\n\
                   proc other {} { setdef shared }\n\
                   proc caller {} { puts $shared }\n";
        let analysis = analyse(src);
        let read = offset_of(src, "$shared") + 1;
        assert!(
            caller_frame_bindings(&analysis, src, "tcl9.0", Some(reg()), read, "shared").is_empty()
        );
    }

    /// Both halves of the idiom are one variable: the bare call-site word and
    /// every `$name` read it feeds.
    #[test]
    fn reference_spans_cover_the_call_site_word_and_the_reads() {
        let analysis = analyse(IDX58);
        let read = offset_of(IDX58, "$dataset") + 1;
        let spans =
            caller_frame_reference_spans(&analysis, IDX58, "tcl9.0", Some(reg()), read, "dataset");
        assert_eq!(spans.len(), 2, "call-site word + one read: {spans:?}");
        for span in &spans {
            assert_eq!(&IDX58[span.as_range()], "dataset");
        }
        assert!(spans[0].start() < offset_of(IDX58, "$dataset"));
    }

    /// A `$name`-shaped run inside a comment is not a read (issue #923 idx
    /// 24), so it must not enter the reference set.
    #[test]
    fn a_commented_read_is_not_a_reference() {
        let src = "proc setdef {d} { upvar 1 $d dst; set dst 1 }\n\
                   proc caller {} {\n setdef shared\n # mentions $shared but inertly\n }\n";
        let analysis = analyse(src);
        let call = offset_of(src, "setdef shared") + 7;
        let spans =
            caller_frame_reference_spans(&analysis, src, "tcl9.0", Some(reg()), call, "shared");
        assert_eq!(spans.len(), 1, "only the call-site word: {spans:?}");
    }
}

#[cfg(test)]
mod caller_frame_navigation_tests {
    use tcl_compiler::analyser::Analyser;

    /// The idx-58 shape: `$dataset` is created by the callee's `upvar`, and a
    /// sibling `TclOO` **method** happens to share the name.
    const SRC: &str = "\
proc gridlayoutHasDataSetObj {dts} {
    upvar 1 $dts dataset
    set dataset MY-SHARED-DATASET
}
oo::class create chart {
    constructor {} {
        gridlayoutHasDataSetObj dataset
        set _dataset $dataset
    }
    method dataset {} { return 1 }
}
";

    fn analysis() -> tcl_compiler::analyser::AnalysisResult {
        Analyser::new().analyse(SRC, "tcl9.0").clone()
    }

    /// Line/character of the `dataset` inside `set _dataset $dataset`.
    fn read_position() -> (u32, u32) {
        let line = SRC
            .lines()
            .position(|l| l.contains("set _dataset $dataset"))
            .expect("read line present");
        let col = SRC.lines().nth(line).unwrap().find("$dataset").unwrap() + 2;
        (u32::try_from(line).unwrap(), u32::try_from(col).unwrap())
    }

    /// Line/character of the bare `dataset` argument at the call site.
    fn call_site_position() -> (u32, u32) {
        let line = SRC
            .lines()
            .position(|l| l.contains("gridlayoutHasDataSetObj dataset"))
            .expect("call line present");
        let text = SRC.lines().nth(line).unwrap();
        let head = "gridlayoutHasDataSetObj ";
        let col = text.find(head).unwrap() + head.len() + 2;
        (u32::try_from(line).unwrap(), u32::try_from(col).unwrap())
    }

    /// TP + the idx-58 FP fix in one: hover on the caller-frame read names the
    /// variable and its creating callee — and, critically, never renders the
    /// same-named **method**'s card.
    #[test]
    fn hover_on_a_caller_frame_read_names_the_creating_call() {
        let analysis = analysis();
        let (line, character) = read_position();
        let hover = crate::hover::hover(
            SRC,
            line,
            character,
            &analysis,
            Some(tcl_registry::registry_for_dialect("tcl9.0")),
        )
        .expect("caller-frame hover");
        assert!(
            hover.value.contains("Caller-frame variable"),
            "expected a caller-frame card: {}",
            hover.value
        );
        assert!(
            hover.value.contains("gridlayoutHasDataSetObj"),
            "the card must name the creating callee: {}",
            hover.value
        );
        assert!(
            !hover.value.contains("method"),
            "a `$`-led read must never resolve to a same-named method: {}",
            hover.value
        );
    }

    /// Go-to-definition reaches the call-site word that creates the variable —
    /// the nearest thing the frame has to a declaration.
    #[test]
    fn definition_on_a_caller_frame_read_reaches_the_call_site_word() {
        let analysis = analysis();
        let (line, character) = read_position();
        let locs = crate::definition::definition(SRC, line, character, &analysis);
        assert_eq!(locs.len(), 1, "one definition: {locs:?}");
        let (call_line, call_col) = call_site_position();
        assert_eq!(locs[0].start_line, call_line, "{locs:?}");
        assert!(
            locs[0].start_character <= call_col && locs[0].end_character >= call_col,
            "definition must cover the call-site word: {locs:?}"
        );
    }

    /// Find-All-References links both halves of the idiom.
    #[test]
    fn references_link_the_call_site_word_and_the_read() {
        let analysis = analysis();
        let (line, character) = read_position();
        let refs = crate::references::references(SRC, "tcl9.0", line, character, &analysis, true);
        assert_eq!(refs.len(), 2, "call-site word + read: {refs:?}");
        // …and from the bare call-site word too.
        let (cl, cc) = call_site_position();
        let from_call = crate::references::references(SRC, "tcl9.0", cl, cc, &analysis, true);
        assert_eq!(from_call, refs, "both anchors give one reference set");
    }

    /// TN — a `$`-led read that nothing binds abstains rather than falling
    /// through to a same-named method (the idx-58 wrong-kind conflation).
    #[test]
    fn an_unbound_dollar_read_never_resolves_to_a_same_named_method() {
        let src = "\
oo::class create widget {
    constructor {} { puts $thing }
    method thing {} { return 1 }
}
";
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        let line =
            u32::try_from(src.lines().position(|l| l.contains("puts $thing")).unwrap()).unwrap();
        let col = u32::try_from(
            src.lines()
                .nth(line as usize)
                .unwrap()
                .find("$thing")
                .unwrap()
                + 2,
        )
        .unwrap();
        assert!(
            crate::hover::hover(
                src,
                line,
                col,
                &analysis,
                Some(tcl_registry::registry_for_dialect("tcl9.0"))
            )
            .is_none(),
            "an unbound `$`-led read must draw no hover at all"
        );
        assert!(
            crate::references::references(src, "tcl9.0", line, col, &analysis, true).is_empty(),
            "an unbound `$`-led read must report no references"
        );
        assert!(
            crate::definition::definition(src, line, col, &analysis).is_empty(),
            "an unbound `$`-led read must report no definition"
        );
    }
}
