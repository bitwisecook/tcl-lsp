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
//! Two per-parameter facts, and a binding needs **both**:
//!
//! * [`ProcArgTrait::VarWrite`] / [`ProcArgTrait::VarRead`] — the parameter's
//!   *value* is used as a variable name through an `upvar`, and whether the
//!   callee writes through the alias or only reads it.  This is what
//!   distinguishes a creating call site from a referencing one.
//! * [`ProcDef::caller_frame_params`](tcl_compiler::analyser::ProcDef::caller_frame_params)
//!   — the alias lands in the **immediate caller's** frame.  The traits carry
//!   no frame level at all, and only `upvar 1` (or an omitted level) reaches
//!   the caller: `upvar 0` aliases the callee's *own* frame, `upvar #0` the
//!   global one, `upvar 2` the caller's caller.  Trusting the trait alone
//!   navigated a variable the frame never gains (codex review of PR #1085).
//!
//! No command name appears here: which words name variables, and which
//! nested scripts still run in this frame, are registry- and
//! analyser-derived.
//!
//! C Tcl, pinned on tclsh 9.0.4 and 8.6.14 (identical):
//!
//! ```tcl
//! proc setdef {d} { upvar 1 $d dst; set dst SET }
//! proc build {} { setdef options; return $options }
//! build            ;# → SET — `options` exists in build's frame, unassigned there
//!
//! proc p0 {d} { upvar 0 $d dst; set dst SET }
//! proc build0 {} { p0 options; return [info exists options] }
//! build0           ;# → 0 — `upvar 0` aliased p0's OWN local, nothing here
//! ```
//!
//! # Literal caller-frame targets (issue #1139)
//!
//! A callee that binds a **literal** caller-side name (`upvar 1 name name`,
//! issue #923 audit idx 22) spells that name nowhere at the call site, so
//! there is no argument word to key on.  The analyser records those names
//! per proc on
//! [`ProcDef::caller_frame_literals`](tcl_compiler::analyser::ProcDef::caller_frame_literals),
//! and [`caller_frame_bindings`] answers for them with the *call-head word*
//! as the binding span — the point where the variable comes to exist in
//! this frame.  A fully-qualified target (`upvar ::tk::FocusGrab($i) data`,
//! idx 98) is not a caller-frame variable at all: it names one fixed global
//! cell, which the analyser's `handle_upvar_command` now defines and links
//! directly.
//!
//! # What it deliberately does not answer
//!
//! A callee reached through `my` / `next` / object dispatch is a method the
//! call never names statically, so its literal targets cannot be resolved
//! here yet — those reads keep the abstaining answer rather than a wrong
//! one (the compiler-side dispatch widening of issue #1177 keeps the
//! diagnostics honest in the meantime; per-callee method resolution
//! composes with issue #1164's MRO work).

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
    /// The callee parameter whose *value* names the variable — or `None`
    /// when the callee spells the name **literally in its own body**
    /// (`upvar 1 name name`, issue #923 audit idx 22 / issue #1139), so no
    /// call-site word carries it at all.
    pub param: Option<String>,
    /// Span of the call-site word that names it — `dataset` in
    /// `gridlayoutHasDataSetObj dataset`.  This is both the creating write
    /// (go-to-definition's target) and a reference to the variable.  For a
    /// literal-target binding ([`Self::param`] `None`) there is no such
    /// word, so this is the call's command-head word — the point where the
    /// variable comes to exist in this frame.
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
        !proc_def.caller_frame_params.is_empty() || !proc_def.caller_frame_literals.is_empty()
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
    resolution: crate::definition::CallResolution<'_>,
    cursor_off: u32,
    name: &str,
) -> Vec<CallerFrameBinding> {
    let mut out: Vec<CallerFrameBinding> = Vec::new();
    // This runs on every hover / go-to-definition / find-references that fails
    // the ordinary scope-chain lookup, and re-segmenting the enclosing frame
    // is the expensive part.  A document with no call-by-name procedure at all
    // — the overwhelming majority — can answer from the already-computed
    // per-proc facts without touching the source.
    if name.is_empty() || !document_has_call_by_name_proc(analysis) {
        return out;
    }
    let (start, end) = enclosing_frame_region(&analysis.global_scope, cursor_off, source);
    if start >= end {
        return out;
    }
    let ctx = BindingScan {
        analysis,
        source,
        dialect,
        resolution,
        namespace: crate::definition::namespace_context_at(
            &analysis.global_scope,
            cursor_off,
            &analysis.namespace_overrides,
        ),
        name,
    };
    collect_bindings_in_region(&ctx, start, end, 0, &mut out);
    out.sort_by_key(|b| b.arg_span.start());
    out.dedup();
    out
}

/// The immutable inputs one binding scan threads through its recursion.
struct BindingScan<'a> {
    analysis: &'a AnalysisResult,
    source: &'a str,
    dialect: &'a str,
    /// The whole-program context every [`crate::definition::resolve_called_proc`]
    /// in this scan is answered in — the builtin gate and, when the host has a
    /// workspace index, the export oracle (issue #1116 item 1).
    resolution: crate::definition::CallResolution<'a>,
    namespace: String,
    name: &'a str,
}

/// Collect the binding call sites in one script region, then recurse into
/// every **same-frame** script nested in it.
///
/// The frame is not the region's outer command list: `if {$ok} { setdef x }`
/// runs `setdef` in the very frame the `if` is written in, so the variable it
/// creates through `upvar` belongs here and navigation must find it.  Which
/// nested arguments are same-frame is the registry's answer, not a keyword
/// list — [`crate::references::nested_dispatch_regions`] is the existing
/// walker for exactly this question (`ArgRole::Body` gated on a `Plain`
/// [`tcl_registry::BodyKind`], plus `[…]` substitutions, plus `switch`-style
/// clause lists via the registry's own `CaseListSpec`).  Reusing it means a
/// spec change reaches this scan too, and `Structural` bodies — `proc`,
/// `namespace eval`, `uplevel`, `oo::define` — plus `apply`'s
/// `LambdaLiteral` stay excluded, which is what the fresh-frame boundary
/// requires: a `setdef x` inside a nested `proc` body creates *that* proc's
/// variable, never this frame's.
///
/// `depth` uses the same [`crate::references::MAX_DISPATCH_SCAN_DEPTH`] guard
/// the sibling dispatch scans use.
fn collect_bindings_in_region(
    ctx: &BindingScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    out: &mut Vec<CallerFrameBinding>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;

    if start >= end
        || end > ctx.source.len()
        || crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth)
    {
        return;
    }
    let commands = segment_commands_with_offset_and_config(
        &ctx.source[start..end],
        u32::try_from(start).unwrap_or(0),
        tcl_lexer::LexerConfig::for_dialect(ctx.dialect),
    );
    for cmd in &commands {
        bindings_from_call(ctx, cmd, out);
        for (inner_start, inner_end) in
            crate::references::nested_dispatch_regions(ctx.source, ctx.dialect, cmd)
        {
            collect_bindings_in_region(ctx, inner_start, inner_end, depth + 1, out);
        }
    }
}

/// The bindings one call site contributes — at most one per parameter whose
/// actual argument is the bare name being navigated.
fn bindings_from_call(
    ctx: &BindingScan<'_>,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
    out: &mut Vec<CallerFrameBinding>,
) {
    let (Some(head), Some(head_text)) = (cmd.argv.first(), cmd.texts.first()) else {
        return;
    };
    let Some(proc_def) = crate::definition::resolve_called_proc(
        ctx.analysis,
        ctx.source,
        &ctx.namespace,
        head_text,
        head.span.start(),
        ctx.resolution,
    ) else {
        return;
    };
    if proc_def.caller_frame_params.is_empty() && proc_def.caller_frame_literals.is_empty() {
        return;
    }
    // A literal caller-frame target (`upvar 1 name name`, issue #1139): the
    // callee spells the name in its own body, so the *call itself* is the
    // binding — no argument word to key on.  The command-head word stands in
    // as the binding span: it is the point where the variable comes to exist
    // in this frame (tclsh 9.0.4 / 8.6.14: `proc np {} {upvar name name;
    // set name W1}` then `np; puts $name` prints `W1`).
    if let Some(written) = proc_def.caller_frame_literals.get(ctx.name) {
        out.push(CallerFrameBinding {
            callee: proc_def.qualified_name.clone(),
            param: None,
            arg_span: head.span,
            call_span: head.span,
            read_only: !written,
        });
    }
    for (i, param) in proc_def.params.iter().enumerate() {
        let (Some(arg_tok), Some(arg_text)) = (cmd.argv.get(i + 1), cmd.texts.get(i + 1)) else {
            break;
        };
        if arg_text != ctx.name || !is_plain_var_name(arg_text) {
            continue;
        }
        // The trait alone is not enough. `VarWrite` / `VarRead` say the
        // parameter's value is used as a variable *name* through an `upvar`,
        // but never which frame the alias lands in — and only `upvar 1` lands
        // in the caller's. tclsh 9.0.4 and 8.6.14 agree exactly: with `proc q
        // {n} {upvar 1 $n a; set a 1}` the caller's variable exists after
        // `q y`; with `upvar 0` / `upvar #0` / `upvar 2` in its place it
        // never does. A binding claimed from the trait alone would navigate a
        // variable this frame does not have.
        // `ProcDef::caller_frame_params` is that missing level fact.
        if !proc_def.caller_frame_params.contains(&param.name) {
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
            param: Some(param.name.clone()),
            arg_span: arg_tok.span,
            call_span: head.span,
            read_only: !writes,
        });
    }
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
    resolution: crate::definition::CallResolution<'_>,
    cursor_off: u32,
    name: &str,
) -> Vec<Span> {
    let bindings = caller_frame_bindings(analysis, source, dialect, resolution, cursor_off, name);
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

/// Whether the `$` at byte `at` is **escaped** by the backslash run
/// immediately before it, and so introduces no substitution.
///
/// Tcl's rule is the run's parity, not merely "is the previous byte a
/// backslash": each `\\` pair is itself an escaped backslash and leaves the
/// `$` free.  Pinned identical on tclsh 9.0.4 and 8.6.14 with `name` set to
/// `SET` through the very `upvar` idiom this module navigates:
///
/// | source | result | `$` substitutes? |
/// |---|---|---|
/// | `"$name"` | `SET` | yes |
/// | `"\$name"` | `$name` | no |
/// | `"\\$name"` | `\SET` | **yes** |
/// | `"\\\$name"` | `\$name` | no |
/// | `"\\\\$name"` | `\\SET` | **yes** |
/// | `"\${name}"` | `${name}` | no |
/// | `"\\${name}"` | `\SET` | **yes** |
///
/// Counting only the immediately-preceding byte would call the even-run rows
/// escaped and silently drop real references.
fn dollar_is_escaped(bytes: &[u8], at: usize) -> bool {
    let run = bytes[..at]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count();
    run % 2 == 1
}

/// Spans of every `$name` / `${name}` occurrence in `source[start..end]` that
/// Tcl actually substitutes.
///
/// The inertness proofs ([`crate::inert_text`]) are the same ones
/// [`crate::definition::lookup_var_read_at`] applies, so a `$name`-shaped run
/// inside a comment or a brace-quoted data word is not counted here either
/// (issue #923 audit idx 24) — and a backslash-escaped `$`
/// ([`dollar_is_escaped`]) is no substitution at all.
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
            if dollar_is_escaped(bytes, at) {
                continue;
            }
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
///
/// A **literal-target** binding is deliberately excluded: its `arg_span` is
/// the call's command-head word, and a bare cursor on the command name must
/// keep resolving as the *command* (definition/references of the proc), not
/// silently become a variable.
#[must_use]
pub(crate) fn binding_at_offset(
    analysis: &AnalysisResult,
    source: &str,
    dialect: &str,
    resolution: crate::definition::CallResolution<'_>,
    cursor_off: u32,
    word: &str,
) -> Option<CallerFrameBinding> {
    caller_frame_bindings(analysis, source, dialect, resolution, cursor_off, word)
        .into_iter()
        .filter(|b| b.param.is_some())
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
        let bindings = caller_frame_bindings(
            &analysis,
            IDX58,
            "tcl9.0",
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "dataset",
        );
        assert_eq!(bindings.len(), 1, "one binding call site: {bindings:?}");
        assert_eq!(bindings[0].param.as_deref(), Some("dts"));
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
            caller_frame_bindings(
                &analysis,
                src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "thing"
            )
            .is_empty()
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
            caller_frame_bindings(
                &analysis,
                src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "thing"
            )
            .is_empty()
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
            caller_frame_bindings(
                &analysis,
                src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "shared"
            )
            .is_empty()
        );
    }

    /// Both halves of the idiom are one variable: the bare call-site word and
    /// every `$name` read it feeds.
    #[test]
    fn reference_spans_cover_the_call_site_word_and_the_reads() {
        let analysis = analyse(IDX58);
        let read = offset_of(IDX58, "$dataset") + 1;
        let spans = caller_frame_reference_spans(
            &analysis,
            IDX58,
            "tcl9.0",
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "dataset",
        );
        assert_eq!(spans.len(), 2, "call-site word + one read: {spans:?}");
        for span in &spans {
            assert_eq!(&IDX58[span.as_range()], "dataset");
        }
        assert!(spans[0].start() < offset_of(IDX58, "$dataset"));
    }

    /// **Finding 1 (codex review of PR #1085).** The `VarRead` / `VarWrite`
    /// trait says a parameter's value is used as a variable *name* through an
    /// `upvar`; it does not say which frame the alias lands in.  Only
    /// `upvar 1` lands in the caller's, so every other level must bind
    /// nothing here.
    ///
    /// tclsh 9.0.4 / 8.6.14, byte-identical — with `proc q {n} {upvar L $n a;
    /// set a 1}` called as `q y` from inside a proc, `info exists y` in the
    /// caller afterwards:
    ///
    /// | `L` | caller's `y` exists |
    /// |---|---|
    /// | `1` | **1** |
    /// | `0` | 0 — the alias is the *callee's own* local |
    /// | `#0` | 0 — the alias is the global `::y` |
    /// | `2` | 0 — the alias is the caller's *caller* |
    #[test]
    fn only_a_caller_frame_upvar_level_binds_here() {
        for level in ["0", "#0", "2", "$lvl", "bogus"] {
            let src = format!(
                "proc setdef {{d}} {{ upvar {level} $d dst; set dst 1 }}\n\
                 proc caller {{}} {{ setdef shared\n puts $shared }}\n"
            );
            let analysis = analyse(&src);
            let read = offset_of(&src, "$shared") + 1;
            let bindings = caller_frame_bindings(
                &analysis,
                &src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "shared",
            );
            assert!(
                bindings.is_empty(),
                "`upvar {level}` does not bind the caller's frame, so navigation \
                 must abstain; got {bindings:?}"
            );
        }
        // TN — the plain `upvar 1 $n a` idiom, and its default-level
        // spelling, keep working.
        for level in ["1 ", ""] {
            let src = format!(
                "proc setdef {{d}} {{ upvar {level}$d dst; set dst 1 }}\n\
                 proc caller {{}} {{ setdef shared\n puts $shared }}\n"
            );
            let analysis = analyse(&src);
            let read = offset_of(&src, "$shared") + 1;
            let bindings = caller_frame_bindings(
                &analysis,
                &src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "shared",
            );
            assert_eq!(
                bindings.len(),
                1,
                "`upvar {level}$d dst` binds the caller's frame: {bindings:?}"
            );
        }
    }

    /// `namespace upvar ns $token local` aliases a **namespace** variable, so
    /// a call site passing the token creates nothing in the calling frame
    /// either — even though the parameter carries the same `VarRead` trait.
    #[test]
    fn namespace_upvar_binds_nothing_in_the_calling_frame() {
        let src = "namespace eval ::cfg { variable shared 1 }\n\
                   proc getcfg {t} { namespace upvar ::cfg $t local; return $local }\n\
                   proc caller {} { getcfg shared\n puts $shared }\n";
        let analysis = analyse(src);
        let read = offset_of(src, "puts $shared") + 6;
        assert!(
            caller_frame_bindings(
                &analysis,
                src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "shared"
            )
            .is_empty()
        );
    }

    /// **Finding 3.** A call in a same-frame script — `if` / `while` /
    /// `catch` / `foreach` body, a `switch` clause — runs in the very frame
    /// the construct is written in, so the variable it creates belongs here.
    ///
    /// tclsh 9.0.4 / 8.6.14, identical: with `proc setdef {d} {upvar 1 $d
    /// dst; set dst SET}`, each of `if {$ok} {setdef x; return $x}`,
    /// `while … {setdef y…}`, `catch {setdef z}`, `switch a {a {setdef s}}`
    /// and `foreach i {1} {setdef f}` returns `SET` from the enclosing proc.
    #[test]
    fn a_binding_inside_a_same_frame_body_is_visible_here() {
        const SETDEF: &str = "proc setdef {d} { upvar 1 $d dst; set dst SET }\n";
        for body in [
            "if {$ok} { setdef shared }\n puts $shared",
            "while {$ok} { setdef shared }\n puts $shared",
            "catch { setdef shared }\n puts $shared",
            "foreach i {1} { setdef shared }\n puts $shared",
            "switch a { a { setdef shared } }\n puts $shared",
            "if {$ok} { if {$ok} { setdef shared } }\n puts $shared",
            "set r [expr {1}]\n if {$ok} { setdef shared }\n puts $shared",
        ] {
            let src = format!("{SETDEF}proc caller {{ok}} {{\n {body}\n}}\n");
            let analysis = analyse(&src);
            let read = offset_of(&src, "puts $shared") + 6;
            let bindings = caller_frame_bindings(
                &analysis,
                &src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "shared",
            );
            assert_eq!(
                bindings.len(),
                1,
                "a same-frame nested body still binds this frame — {body:?}: {bindings:?}"
            );
            assert_eq!(&src[bindings[0].arg_span.as_range()], "shared");
        }
    }

    /// TN for finding 3 — the fresh-frame boundary. A `setdef` written inside
    /// a nested `proc` body, an `apply` lambda, or a `namespace eval` body
    /// creates *that* frame's variable, never the enclosing one.
    ///
    /// tclsh 9.0.4 / 8.6.14, identical: all three report `info exists` = 0 in
    /// the enclosing proc afterwards.
    #[test]
    fn a_binding_inside_a_fresh_frame_body_does_not_leak_out() {
        const SETDEF: &str = "proc setdef {d} { upvar 1 $d dst; set dst SET }\n";
        for body in [
            "proc inner {} { setdef shared }\n inner\n puts $shared",
            "apply {{} { setdef shared }}\n puts $shared",
            "namespace eval ::zz { setdef shared }\n puts $shared",
            "uplevel 1 { setdef shared }\n puts $shared",
        ] {
            let src = format!("{SETDEF}proc caller {{}} {{\n {body}\n}}\n");
            let analysis = analyse(&src);
            let read = offset_of(&src, "puts $shared") + 6;
            let bindings = caller_frame_bindings(
                &analysis,
                &src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "shared",
            );
            assert!(
                bindings.is_empty(),
                "a fresh-frame body's binding must not leak out — {body:?}: {bindings:?}"
            );
        }
    }

    /// **Finding 2.** Tcl's backslash prevents substitution, and the rule is
    /// the backslash *run's parity*, not "is the previous byte a backslash".
    ///
    /// tclsh 9.0.4 / 8.6.14, identical, with `name` upvar-set to `SET`:
    /// `"\$name"` → `$name` (no substitution), `"\\$name"` → `\SET`
    /// (**substitution**), `"\\\$name"` → `\$name`, `"\\\\$name"` → `\\SET`,
    /// `"\${name}"` → `${name}`, `"\\${name}"` → `\SET`.
    #[test]
    fn an_escaped_dollar_is_not_a_read_but_an_escaped_backslash_leaves_one() {
        // Rust `\\` is one source backslash.
        for (fragment, extra_reads) in [
            ("puts \"\\$shared\"", 0),   // \$shared   — escaped
            ("puts \"\\\\$shared\"", 1), // \\$shared  — real substitution
            ("puts \"\\${shared}\"", 0), // \${shared} — escaped, braced form
            ("puts \"\\\\${shared}\"", 1),
            ("puts \"\\\\\\$shared\"", 0), // \\\$shared — escaped
            ("puts \"\\\\\\\\$shared\"", 1),
        ] {
            let src = format!(
                "proc setdef {{d}} {{ upvar 1 $d dst; set dst 1 }}\n\
                 proc caller {{}} {{\n setdef shared\n {fragment}\n}}\n"
            );
            let analysis = analyse(&src);
            let call = offset_of(&src, "setdef shared") + 7;
            let spans = caller_frame_reference_spans(
                &analysis,
                &src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                call,
                "shared",
            );
            assert_eq!(
                spans.len(),
                1 + extra_reads,
                "call-site word + {extra_reads} read for {fragment:?}: {spans:?}"
            );
        }
    }

    /// The literal-target shape of issue #923 audit idx 22 / issue #1139,
    /// proc form: the callee spells the caller-frame name in its **own**
    /// body (`upvar name name`), so no call-site word carries it.
    ///
    /// tclsh 9.0.4: `proc NameProcess {arguments object} {upvar name name;
    /// set name W1}` then `proc build {} {NameProcess x obj; return $name}`
    /// — `build` returns `W1`.
    const IDX22: &str = "\
proc NameProcess {arguments object} {
    upvar name name
    set name W1
}
proc build {} {
    NameProcess x obj
    set out $name
}
";

    #[test]
    fn a_literal_upvar_target_binds_at_the_call_head() {
        let analysis = analyse(IDX22);
        let read = offset_of(IDX22, "$name") + 1;
        let bindings = caller_frame_bindings(
            &analysis,
            IDX22,
            "tcl9.0",
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "name",
        );
        assert_eq!(bindings.len(), 1, "one binding call site: {bindings:?}");
        assert_eq!(bindings[0].param, None, "no call-site word carries it");
        assert!(!bindings[0].read_only, "the alias is written through");
        assert_eq!(
            &IDX22[bindings[0].arg_span.as_range()],
            "NameProcess",
            "the binding span is the call-head word"
        );
    }

    /// TN — the literal fact is level-gated exactly like the param fact:
    /// only `upvar 1` (or the omitted level) binds this frame.
    #[test]
    fn a_literal_target_at_a_non_caller_level_binds_nothing() {
        for level in ["0", "#0", "2", "$lvl"] {
            let src = format!(
                "proc np {{}} {{ upvar {level} name name; set name 1 }}\n\
                 proc build {{}} {{ np\n puts $name }}\n"
            );
            let analysis = analyse(&src);
            let read = offset_of(&src, "$name") + 1;
            let bindings = caller_frame_bindings(
                &analysis,
                &src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "name",
            );
            assert!(
                bindings.is_empty(),
                "`upvar {level}` does not bind the caller's frame: {bindings:?}"
            );
        }
    }

    /// TN — a `::`-qualified target names a fixed global/namespace cell,
    /// not a caller-frame variable; the analyser's `otherVar` link owns it
    /// (issue #923 idx 98).
    #[test]
    fn a_qualified_literal_target_is_not_a_caller_frame_binding() {
        let src = "proc np {} { upvar 1 ::tk::FocusGrab(x) data; set data 1 }\n\
                   proc build {} { np\n puts $FocusGrab }\n";
        let analysis = analyse(src);
        let read = offset_of(src, "$FocusGrab") + 1;
        assert!(
            caller_frame_bindings(
                &analysis,
                src,
                "tcl9.0",
                crate::definition::CallResolution::document_only().with_registry(reg()),
                read,
                "FocusGrab"
            )
            .is_empty()
        );
    }

    /// A callee that only *reads* through the literal alias references the
    /// variable without creating it (tclsh 9.0.4: `proc peekname {} {upvar
    /// name name; return [info exists name]}` creates nothing).
    #[test]
    fn a_read_only_literal_target_is_a_reference_not_a_creation() {
        let src = "proc peekname {} { upvar name name; return [string length $name] }\n\
                   proc build {} { set name N0\n peekname }\n";
        let analysis = analyse(src);
        let call = offset_of(src, "peekname }");
        let bindings = caller_frame_bindings(
            &analysis,
            src,
            "tcl9.0",
            crate::definition::CallResolution::document_only().with_registry(reg()),
            call,
            "name",
        );
        assert_eq!(bindings.len(), 1, "{bindings:?}");
        assert!(bindings[0].read_only, "{bindings:?}");
    }

    /// The parity helper itself, over the pinned table.
    #[test]
    fn dollar_escape_parity_matches_c_tcl() {
        for (text, escaped) in [
            ("$n", false),
            ("\\$n", true),
            ("\\\\$n", false),
            ("\\\\\\$n", true),
            ("\\\\\\\\$n", false),
        ] {
            let at = text.find('$').expect("a dollar");
            assert_eq!(dollar_is_escaped(text.as_bytes(), at), escaped, "{text:?}");
        }
    }

    /// A `$name`-shaped run inside a comment is not a read (issue #923 idx
    /// 24), so it must not enter the reference set.
    #[test]
    fn a_commented_read_is_not_a_reference() {
        let src = "proc setdef {d} { upvar 1 $d dst; set dst 1 }\n\
                   proc caller {} {\n setdef shared\n # mentions $shared but inertly\n }\n";
        let analysis = analyse(src);
        let call = offset_of(src, "setdef shared") + 7;
        let spans = caller_frame_reference_spans(
            &analysis,
            src,
            "tcl9.0",
            crate::definition::CallResolution::document_only().with_registry(reg()),
            call,
            "shared",
        );
        assert_eq!(spans.len(), 1, "only the call-site word: {spans:?}");
    }
}

#[cfg(test)]
mod caller_frame_navigation_tests {
    use tcl_compiler::analyser::Analyser;

    /// Issue #923 audit idx 98, re-measured for issue #1139: `upvar
    /// ::tk::FocusGrab($index) data` names a fixed, fully-qualified global
    /// cell (level-independent — tclsh-verified in the audit), so the array
    /// must hover, define, and cross-reference from both the `upvar`
    /// `otherVar` word and a sibling proc's `$::tk::FocusGrab($index)`
    /// read.  Before the cell gained a `VarDef` at the `otherVar` word,
    /// every one of these anchors answered nothing (a silent miss).
    ///
    /// The four references are: the `otherVar` word, the `info exists`
    /// argument, the `$` read, and the `unset` argument.  A *bareword*
    /// cursor on the `info exists` argument still abstains — barewords in
    /// variable-role argument positions are a separate anchor kind — but
    /// the occurrence itself is in the reference set.
    #[test]
    fn a_fully_qualified_upvar_target_navigates_from_word_and_read() {
        let src = "\
proc SetFocusGrab {grab focus} {
    set index \"$grab,$focus\"
    upvar ::tk::FocusGrab($index) data
    lappend data one
}
proc RestoreFocusGrab {grab focus} {
    set index \"$grab,$focus\"
    if {[info exists ::tk::FocusGrab($index)]} {
        set data2 $::tk::FocusGrab($index)
        unset ::tk::FocusGrab($index)
    }
}
";
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        for (label, needle, extra) in [
            ("upvar-othervar", "upvar ::tk::FocusGrab", 8usize),
            ("read", "$::tk::FocusGrab($index)", 3usize),
        ] {
            let off = src.find(needle).unwrap() + extra;
            let line = u32::try_from(src[..off].matches('\n').count()).unwrap();
            let line_start = src[..off].rfind('\n').map_or(0, |i| i + 1);
            let col = u32::try_from(off - line_start).unwrap();
            let hover = crate::hover::hover(
                src,
                line,
                col,
                &analysis,
                Some(tcl_registry::registry_for_dialect("tcl9.0")),
            )
            .unwrap_or_else(|| panic!("{label}: expected a hover"));
            assert!(
                hover.value.contains("::tk::FocusGrab"),
                "{label}: the card must name the cell: {}",
                hover.value
            );
            let defs = crate::definition::definition(src, line, col, &analysis);
            assert_eq!(defs.len(), 1, "{label}: one definition: {defs:?}");
            let refs = crate::references::references(src, "tcl9.0", line, col, &analysis, true);
            assert_eq!(
                refs.len(),
                4,
                "{label}: otherVar word + info exists + read + unset: {refs:?}"
            );
        }
    }

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

    /// **Finding 1 at provider level.** With the callee's `upvar` aimed
    /// anywhere but the caller's frame, hover / go-to-definition /
    /// find-references must all abstain — not navigate a variable this frame
    /// never gains.  (`upvar 0` aliases the callee's own frame, `#0` the
    /// global one, `2` the caller's caller; pinned identical on tclsh 9.0.4
    /// and 8.6.14.)
    #[test]
    fn a_non_caller_frame_upvar_level_draws_no_navigation() {
        for level in ["0", "\\#0", "2"] {
            let src = format!(
                "proc setdef {{d}} {{ upvar {level} $d dst; set dst 1 }}\n\
                 proc caller {{}} {{\n    setdef shared\n    puts $shared\n}}\n"
            );
            let analysis = Analyser::new().analyse(&src, "tcl9.0").clone();
            let line = u32::try_from(
                src.lines()
                    .position(|l| l.contains("puts $shared"))
                    .unwrap(),
            )
            .unwrap();
            let col = u32::try_from(
                src.lines()
                    .nth(line as usize)
                    .unwrap()
                    .find("$shared")
                    .unwrap()
                    + 2,
            )
            .unwrap();
            assert!(
                crate::hover::hover(
                    &src,
                    line,
                    col,
                    &analysis,
                    Some(tcl_registry::registry_for_dialect("tcl9.0"))
                )
                .is_none(),
                "`upvar {level}` must draw no caller-frame hover"
            );
            assert!(
                crate::references::references(&src, "tcl9.0", line, col, &analysis, true)
                    .is_empty(),
                "`upvar {level}` must report no references"
            );
            assert!(
                crate::definition::definition(&src, line, col, &analysis).is_empty(),
                "`upvar {level}` must report no definition"
            );
        }
    }

    /// **Finding 4.** `include_declaration = false` drops the *creating* call
    /// site only.  A call whose callee upvar-**reads** through the alias
    /// (`peek x`) creates nothing — tclsh 9.0.4 / 8.6.14 both report `info
    /// exists` = 0 in the caller after `peek w` alone — so that site is an
    /// ordinary reference and must survive.
    #[test]
    fn read_only_call_sites_survive_without_declarations() {
        let src = "\
proc make {d} { upvar 1 $d dst; set dst SET }
proc peek {d} { upvar 1 $d src; return $src }
proc caller {} {
    make shared
    peek shared
    puts $shared
}
";
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        let line = u32::try_from(
            src.lines()
                .position(|l| l.contains("puts $shared"))
                .unwrap(),
        )
        .unwrap();
        let col = u32::try_from(
            src.lines()
                .nth(line as usize)
                .unwrap()
                .find("$shared")
                .unwrap()
                + 2,
        )
        .unwrap();
        let with_decls = crate::references::references(src, "tcl9.0", line, col, &analysis, true);
        assert_eq!(
            with_decls.len(),
            3,
            "make + peek + the read: {with_decls:?}"
        );
        let without = crate::references::references(src, "tcl9.0", line, col, &analysis, false);
        assert_eq!(
            without.len(),
            2,
            "the writer `make shared` is the declaration and drops out; the \
             read-only `peek shared` is a reference and stays: {without:?}"
        );
        let make_line =
            u32::try_from(src.lines().position(|l| l.contains("make shared")).unwrap()).unwrap();
        assert!(
            !without.iter().any(|r| r.start_line == make_line),
            "the creating call site must be excluded: {without:?}"
        );
        let peek_line =
            u32::try_from(src.lines().position(|l| l.contains("peek shared")).unwrap()).unwrap();
        assert!(
            without.iter().any(|r| r.start_line == peek_line),
            "the read-only call site must be retained: {without:?}"
        );
    }

    /// Issue #1139 (idx 22's literal-target residual), provider level: a
    /// callee that binds `upvar name name` creates `name` in this frame,
    /// with no call-site word to point at — hover / go-to-definition /
    /// find-references must answer from the call itself instead of the
    /// former silent miss.
    #[test]
    fn navigation_resolves_a_literal_upvar_target_to_the_creating_call() {
        let src = "\
proc NameProcess {arguments object} {
    upvar name name
    set name W1
}
proc build {} {
    NameProcess x obj
    set out $name
}
";
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        let line = u32::try_from(
            src.lines()
                .position(|l| l.contains("set out $name"))
                .unwrap(),
        )
        .unwrap();
        let col = u32::try_from(
            src.lines()
                .nth(line as usize)
                .unwrap()
                .find("$name")
                .unwrap()
                + 2,
        )
        .unwrap();
        let hover = crate::hover::hover(
            src,
            line,
            col,
            &analysis,
            Some(tcl_registry::registry_for_dialect("tcl9.0")),
        )
        .expect("caller-frame hover for a literal target");
        assert!(
            hover.value.contains("Caller-frame variable"),
            "expected a caller-frame card: {}",
            hover.value
        );
        assert!(
            hover.value.contains("NameProcess"),
            "the card must name the creating callee: {}",
            hover.value
        );
        let locs = crate::definition::definition(src, line, col, &analysis);
        assert_eq!(locs.len(), 1, "one definition: {locs:?}");
        let call_line = u32::try_from(
            src.lines()
                .position(|l| l.contains("NameProcess x obj"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            locs[0].start_line, call_line,
            "definition must reach the creating call: {locs:?}"
        );
        let refs = crate::references::references(src, "tcl9.0", line, col, &analysis, true);
        assert!(
            refs.iter().any(|r| r.start_line == call_line),
            "the creating call is part of the reference set: {refs:?}"
        );
        assert!(
            refs.iter().any(|r| r.start_line == line),
            "the read is part of the reference set: {refs:?}"
        );
    }

    /// A bare cursor on the callee's command word keeps resolving as the
    /// **command**, never as the variable its literal `upvar` creates.
    #[test]
    fn the_call_head_word_still_navigates_as_a_command() {
        let src = "\
proc NameProcess {arguments object} {
    upvar name name
    set name W1
}
proc build {} {
    NameProcess x obj
    set out $name
}
";
        let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
        let call_line = u32::try_from(
            src.lines()
                .position(|l| l.contains("NameProcess x obj"))
                .unwrap(),
        )
        .unwrap();
        let col = u32::try_from(
            src.lines()
                .nth(call_line as usize)
                .unwrap()
                .find("NameProcess")
                .unwrap()
                + 2,
        )
        .unwrap();
        let locs = crate::definition::definition(src, call_line, col, &analysis);
        assert_eq!(locs.len(), 1, "one definition: {locs:?}");
        let def_line = u32::try_from(
            src.lines()
                .position(|l| l.contains("proc NameProcess"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            locs[0].start_line, def_line,
            "the command word must reach the proc definition: {locs:?}"
        );
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
