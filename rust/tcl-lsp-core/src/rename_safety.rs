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

//! Rename **safety gate** — the one place that decides a rename must be
//! *refused* rather than answered with an edit set.
//!
//! # The oracle question
//!
//! A rename is a textual edit, and the only question that matters is what
//! the program does *after* it: a rename is safe exactly when the program's
//! runtime binding structure is preserved.  When static analysis cannot
//! prove that, the answer must be a refusal with a precise reason, never a
//! partial edit set — a refused rename costs the user a keystroke, a wrong
//! one silently breaks running code.
//!
//! tclsh 9.0.4 and 8.6.16 agree byte-for-byte on every shape gated here;
//! the transcripts are pinned as tests in this module and in
//! `rust/tcl-lsp-server/tests/e2e/rename_safety.rs`.
//!
//! # What is gated
//!
//! ## Untracked receivers (issue #923 differential-audit finding idx 79)
//!
//! ```tcl
//! oo::class create Vector3d {
//!     constructor {args} {
//!         set other [lindex $args 0]
//!         if {[info object isa typeof $other ::Vector3d]} { set _x [$other X] }
//!     }
//!     method X {} { return $_x }
//!     export X
//! }
//! ```
//!
//! `$other` holds a live `Vector3d` at run time, but nothing in the source
//! *assigns* a constructor result to it — it is `[lindex $args 0]`, guarded
//! by a runtime `info object isa` test.  The analyser's
//! `instance_classes` therefore has no binding for it, and the reference
//! scan ([`crate::references::find_obj_method_call_sites`]) correctly does
//! not match the site.  Renaming `X` used to emit an edit set touching only
//! the declaration; applying it and running under both interpreters gives
//! `unknown method "X": must be Get, GetX, Y or destroy`.
//!
//! The receiver's class cannot be *proved* to be this class — and it cannot
//! be proved *not* to be, either.  So the rename is refused.
//!
//! ## Computed member names
//!
//! `$obj $m` on a receiver that *is* an instance of the renamed class, and
//! `my $m` inside the class's own bodies, dispatch a member whose name is
//! run-time data.  No edit can keep such a site consistent with a renamed
//! declaration, and no analysis can rule out that it names the member being
//! renamed.
//!
//! ## Export lists that cannot be located
//!
//! `export`/`unexport`/`filter` name a method (the registry's
//! [`tcl_registry::MemberRefKind::Method`] members).  Leaving `export Foo`
//! behind after renaming `Foo` to `Bar` really does break the program —
//! tclsh 9.0.4 / 8.6.16, identically: `$a Bar` then answers `unknown method
//! "Bar": must be destroy`, because the renamed method is no longer
//! exported.  Rename rewrites those words ([`crate::references::
//! member_reference_spans`]); when the analyser recorded such a member for
//! the renamed name but the grammar-driven scan cannot find a rewritable
//! word for it (an `oo::define` block the class record does not span), the
//! rename is refused rather than emitted incomplete.
//!
//! ## Ambiguous object commands (issue #981, object-command half)
//!
//! `CLASS create NAME` binds `NAME` in the *creation site's* namespace, so
//! `::a::Factory create rex` and `::b::Widget create rex` are two different
//! commands.  Attribution is namespace-scoped (see
//! [`crate::references::CommandReceivers`]); when two *different* classes
//! bind the very same qualified object-command name, no static rule can say
//! which one a later `rex make` reaches, and the rename is refused.
//!
//! ## Namespace variables whose occurrences are not enumerable
//!
//! The workspace namespace-variable tier renames `$::ns::v` across every
//! document.  A document that computes a variable name
//! ([`tcl_compiler::dynamic_names::names_a_dynamic_variable`]) in a
//! registry-declared variable-name argument position
//! ([`tcl_registry::ArgRole::VarWrite`] / [`tcl_registry::ArgRole::VarRead`])
//! may be naming the very cell being renamed, with no word to rewrite — so
//! a rename that would touch that namespace is refused.
//!
//! The refusal is decided **per site**, not per document (issue #1093): a
//! word's written text bounds the names it can produce, because a
//! substitution can evaluate to anything but the literal characters around it
//! cannot change.  `set ::other::$n 1` therefore stops refusing a rename of
//! `::ns::v` — nothing that word can spell is under `::ns` — while `variable
//! $n` inside `::ns` still refuses.  See
//! [`namespace_variable_rename_hazard`] for the bound and what stays outside
//! it.
//!
//! # No command names here
//!
//! Every command-level fact this module uses comes from the registry: the
//! self-dispatch keyword ([`crate::definition::method_dispatch_keyword_in`]),
//! the definition-body member grammar (`definition_body` /
//! [`tcl_registry::MemberRefKind`]), the same-frame and frame-shifted body
//! roles ([`crate::references::nested_dispatch_regions`]), and the
//! variable-name argument roles.  There is no command-name string match in
//! this file.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::{AnalysisResult, MemberSide};
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_lexer::{LineIndex, Span, TokenType};

use crate::definition::LspRange;
use crate::definition::span_to_range;
use crate::references::{
    MAX_DISPATCH_SCAN_DEPTH, collect_member_bodies_scoped, dispatch_scan_regions,
    strip_outer_braces, strip_var_decoration,
};

/// A refused rename: why, and (when there is one) the offending site.
///
/// The server turns this into an LSP error response, so the editor shows the
/// reason instead of applying a partial edit set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameRefusal {
    /// Precise, user-facing explanation — names the shape that could not be
    /// proved safe, so the user can decide what to do about it.
    pub reason: String,
    /// The site the refusal is about, when it is a single place in the
    /// document the request was issued against.
    pub range: Option<LspRange>,
}

impl RenameRefusal {
    fn new(reason: String, source: &str, line_index: &LineIndex, span: Option<Span>) -> Self {
        Self {
            reason,
            range: span.map(|s| span_to_range(source, line_index, s)),
        }
    }

    /// [`Self::new`] for the sibling gates that live in their own modules
    /// ([`crate::namespace_rename`]) — one constructor, so every refusal
    /// resolves its span against the document it came from the same way.
    #[must_use]
    pub fn at(reason: String, source: &str, line_index: &LineIndex, span: Option<Span>) -> Self {
        Self::new(reason, source, line_index, span)
    }
}

/// The member rename a [`method_rename_hazard`] check is about.
#[derive(Debug, Clone, Copy)]
pub struct MethodRenameTarget<'a> {
    /// Every class in the rename's override family, fully qualified — the
    /// set whose declarations the edit set rewrites.
    pub family: &'a [String],
    /// The member name being renamed.
    pub method: &'a str,
    /// Which dispatch table the member lives in — a `classmethod` is never
    /// reached through an instance receiver.
    pub is_classmethod: bool,
    /// The name the user asked for.
    ///
    /// Needed because some *destinations* are unsafe even when the member and
    /// its dispatches are: renaming a member whose declaration site is a
    /// `renamemethod`'s destination word rewrites that word, and for some names
    /// the result is a class definition real Tcl refuses to run
    /// ([`renamed_member_would_abort`], issue #1121 review).
    pub new_name: &'a str,
}

/// Whether renaming `target`'s member is unsafe **in this document**.
///
/// Called once per document the rename would touch (the request's own
/// document plus every family / consumer document the cross-file path
/// visits), so a hazard anywhere in the workspace refuses the whole rename.
#[must_use]
pub fn method_rename_hazard(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    unlocatable_member_reference(source, dialect, analysis, target, line_index)
        .or_else(|| renamed_member_would_abort(source, analysis, target, line_index))
        .or_else(|| dispatch_hazard(source, dialect, analysis, target, line_index))
        .or_else(|| ambiguous_object_command(source, analysis, target, line_index))
}

/// The requested new name would turn a `renamemethod` in this document into one
/// that **aborts the whole class definition** (issue #1121 review).
///
/// A moved member's declaration site *is* the `renamemethod`'s destination word
/// (that is what makes the arrived member navigable at all), so renaming it
/// rewrites that word and nothing else — `renamemethod old new` becomes
/// `renamemethod old X`. Two values of `X` are fatal, and both are the shapes
/// `W315` already diagnoses, so the gate must refuse rather than answer with
/// edits that make the class invalid:
///
/// ```tcl
/// oo::class create ::C { method old {} {…} ; renamemethod old old }
/// ;# -> cannot rename method to itself           (no class is created)
/// oo::class create ::C { method old {} {…} ; method sib {} {…}
///                        renamemethod old sib }
/// ;# -> method called sib already exists         (no class is created)
/// ```
///
/// The mirror direction counts too: renaming an ordinary sibling *into* the
/// name a later `renamemethod` needs free produces the identical collision at
/// that `renamemethod`'s site.
///
/// Both readings come from [`RenamedMember`], the record the member walker
/// captured at the move — the *same* fold that decided whether to emit `W315`
/// there. Gate and diagnostic therefore cannot disagree about which bodies run.
/// Because the snapshot is taken at the move's own point in the body, order is
/// honoured exactly as the interpreter honours it: a destination deleted
/// earlier is free, and a name declared *after* the `renamemethod` never
/// collides with it (both pinned on tclsh 9.0.4 and 8.6.14).
fn renamed_member_would_abort(
    source: &str,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    for class_q in target.family {
        let Some(class_def) = analysis.all_classes.get(class_q) else {
            continue;
        };
        // A `renamemethod` acts on exactly one of the class's two method
        // tables, and so does the member being renamed — so a move on the
        // *other* side is not evidence about this rename at all. Without this
        // filter an instance-side `renamemethod old same` refused an unrelated
        // class-side `self method same` -> `old`, which real Tcl runs happily
        // (issue #1178 review). Oracle, byte-identical on 9.0.4 and 8.6.14:
        //
        //   oo::class create ::P2 { method old {} { return inst-old }
        //                           renamemethod old same
        //                           self { method old {} { return cls-old } } }
        //   info class methods ::P2   ;# -> same    [::P2 new] same -> inst-old
        //   info object methods ::P2  ;# -> old     ::P2 old        -> cls-old
        //
        // The `W315` consumer of these same records needs no equivalent filter:
        // the walker builds each record at its own move site, against that
        // side's table only, so it never sees the other side's names.
        let target_side = if target.is_classmethod {
            MemberSide::ClassObject
        } else {
            MemberSide::Instance
        };
        for moved in class_def
            .renamed_members
            .iter()
            .filter(|moved| moved.side == target_side)
        {
            // The cursor is on the moved member itself: renaming it rewrites
            // this `renamemethod`'s destination word.
            if moved.destination == target.method
                && let Some(kind) = moved.abort_if_renamed_to(target.new_name)
            {
                return Some(RenameRefusal::new(
                    abort_refusal_reason(kind, target, &moved.source, class_q),
                    source,
                    line_index,
                    Some(moved.destination_span),
                ));
            }
            // The mirror: an ordinary member renamed into the destination this
            // move needs free.
            if moved.collides_with_renaming(target.method, target.new_name) {
                return Some(RenameRefusal::new(
                    format!(
                        "cannot rename `{method}` to `{new}`: class `{class_q}` renames \
                         `{src}` to `{new}` later in the same definition body, so a member \
                         already called `{new}` at that point makes the whole definition \
                         fail with `method called {new} already exists` — no class is \
                         created at all.",
                        method = target.method,
                        new = target.new_name,
                        src = moved.source,
                    ),
                    source,
                    line_index,
                    Some(moved.destination_span),
                ));
            }
        }
    }
    None
}

/// The user-facing reason for a refused rename of a moved member, naming the
/// interpreter error the edit would cause.
fn abort_refusal_reason(
    kind: tcl_compiler::analyser::DefinitionAbortKind,
    target: MethodRenameTarget<'_>,
    source_name: &str,
    class_q: &str,
) -> String {
    let new = target.new_name;
    match kind {
        tcl_compiler::analyser::DefinitionAbortKind::RenameToItself => format!(
            "cannot rename `{method}` to `{new}`: `{method}` is declared by a \
             `renamemethod {source_name} {method}` in class `{class_q}`, so the edit would \
             make it `renamemethod {new} {new}` — which fails with `cannot rename method to \
             itself` and creates no class at all.",
            method = target.method,
        ),
        _ => format!(
            "cannot rename `{method}` to `{new}`: `{method}` is declared by a \
             `renamemethod {source_name} {method}` in class `{class_q}`, and `{new}` is \
             already a member on that side at that point — the edit would fail with `method \
             called {new} already exists` and creates no class at all.",
            method = target.method,
        ),
    }
}

/// A `MemberRefKind::Method` word the analyser recorded for the renamed
/// member (`export X`, `unexport X`, `filter X`) that the grammar-driven
/// span scan cannot find a rewritable word for.
///
/// The two halves disagreeing means the edit set would leave a stale
/// method-name word behind, and that word is load-bearing: an `export`
/// naming a method that no longer exists leaves the *renamed* method
/// unexported, so every `$obj NewName` call fails (`unknown method "Bar":
/// must be destroy` on tclsh 9.0.4 and 8.6.16 alike).
fn unlocatable_member_reference(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    for class_q in target.family {
        let Some(class_def) = analysis.all_classes.get(class_q) else {
            continue;
        };
        // Both sides count. The word is load-bearing wherever it was written,
        // and a `self export X` / `self filter X` left behind breaks the class
        // command's dispatch of the renamed member exactly as an unwrapped one
        // breaks an instance's (issue #1119) — so the class-side sets have to
        // be consulted here or the refusal silently stops covering them.
        let recorded = class_def.exports.contains(target.method)
            || class_def.unexports.contains(target.method)
            || class_def.class_exports.contains(target.method)
            || class_def.class_unexports.contains(target.method)
            || class_def
                .filters
                .iter()
                .chain(class_def.class_filters.iter())
                .any(|f| f == target.method);
        if !recorded {
            continue;
        }
        if crate::references::member_reference_spans(source, dialect, class_def, target.method)
            .is_empty()
        {
            return Some(RenameRefusal::new(
                format!(
                    "cannot rename `{method}`: class `{class_q}` declares an export / \
                     unexport / filter naming it, but the declaring word is outside the \
                     class body this document records, so the rename cannot rewrite it. \
                     Leaving it behind would leave `{method}` unexported and break every \
                     call to the renamed member.",
                    method = target.method,
                ),
                source,
                line_index,
                None,
            ));
        }
    }
    None
}

/// Two different classes binding the very same **qualified** object-command
/// name (`::a::Factory create rex` and `::a::Widget create rex`).
///
/// Namespace scoping (issue #981) tells `::a::rex` from `::b::rex`, but two
/// creations of the same qualified name are genuinely indistinguishable: a
/// later `rex make` reaches whichever creation ran last, which is a runtime
/// fact.  Rewriting either class's `make` would rewrite call sites that may
/// belong to the other, so the rename is refused.
fn ambiguous_object_command(
    source: &str,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let family: FxHashSet<&str> = target.family.iter().map(String::as_str).collect();
    for binding in &analysis.instance_command_bindings {
        if !family.contains(binding.class_q.as_str()) {
            continue;
        }
        let clashes = analysis.instance_command_bindings.iter().any(|other| {
            other.qualified_name == binding.qualified_name && other.class_q != binding.class_q
        });
        if clashes {
            return Some(RenameRefusal::new(
                format!(
                    "cannot rename `{method}`: the object command `{cmd}` is created by \
                     more than one class in this workspace, so which class a `{tail} \
                     {method}` call dispatches to is a runtime fact this rename cannot \
                     resolve.",
                    method = target.method,
                    cmd = binding.qualified_name,
                    tail = tail_of(&binding.qualified_name),
                ),
                source,
                line_index,
                None,
            ));
        }
    }
    None
}

fn tail_of(qualified: &str) -> &str {
    std::str::from_utf8(tcl_syntax::naming::written_command_tail(
        qualified.as_bytes(),
    ))
    .unwrap_or(qualified)
}

/// A dispatch site this rename can neither rewrite nor rule out.
///
/// * **untracked receiver, literal member word** — `$v METHOD`, where `v`
///   has no class binding at all (idx 79's `$args X` / `$other X`).  The
///   receiver may be an instance of the renamed class; nothing in the
///   source says it isn't.
/// * **computed member word on a receiver of this family** — `$v $m` /
///   `[$v [pick]]`, where `v` *is* bound to a family class.  The member the
///   site names is run-time data.
/// * **computed member word on `my`** — the same, through the registry's own
///   self-dispatch keyword, inside a family class's own bodies.
///
/// A receiver bound to some *other* class is not a hazard: its dispatch
/// provably reaches that class's table, not this one, so the ordinary
/// reference scan's decision to skip it is sound.
fn dispatch_hazard(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let hierarchy = analysis.class_hierarchy();
    let family: FxHashSet<&str> = target.family.iter().map(String::as_str).collect();
    let in_family = |class: &str| -> bool {
        family.contains(class)
            || hierarchy
                .method_target(class, target.method)
                .is_some_and(|owner| family.contains(owner))
    };

    let mut hazard: Option<(Span, &'static str)> = None;
    let mut visit = |cmd: &SegmentedCommand| {
        if hazard.is_some() {
            return;
        }
        let (Some(head), Some(member)) = (cmd.argv.first(), cmd.argv.get(1)) else {
            return;
        };
        let Some(head_text) = slice(source, head.span) else {
            return;
        };
        let computed_member = matches!(member.kind, TokenType::Var | TokenType::Cmd);
        if head.kind == TokenType::Var {
            let Some(receiver) = strip_var_decoration(head_text) else {
                return;
            };
            // The receiver's class binding: the analyser's `instance_classes`
            // walk first, then the object-type lattice's scope-keyed map — a
            // **singleton** there is the same sound fact the reference scan
            // rewrites through (issue #994 C5b), so a site the scan covers is
            // no hazard and a site provably of a *different* class is not
            // either.  A multi-class or absent lattice binding stays the
            // untracked-receiver refusal: widening an abstention into "not
            // family" would silence the gate that keeps this rename honest.
            let bound_class = analysis.instance_classes.get(receiver).or_else(|| {
                crate::definition::lattice_singleton_class(analysis, receiver, head.span.start())
            });
            match bound_class {
                // Receiver bound to a class: only a computed member word is
                // a hazard, and only when that class's dispatch really can
                // reach the member being renamed.
                Some(class) => {
                    // An instance receiver never reaches a `classmethod` —
                    // TclOO puts a classmethod on the class object's own
                    // class, which no instance's MRO walks — so a computed
                    // member name here cannot be the one being renamed.
                    if !target.is_classmethod && computed_member && in_family(class) {
                        hazard = Some((member.span, "computed-member"));
                    }
                }
                // Receiver of unknown class: a literal word naming the
                // renamed member is the idx-79 shape.
                None => {
                    if !target.is_classmethod
                        && !computed_member
                        && slice(source, member.span) == Some(target.method)
                    {
                        hazard = Some((member.span, "untracked-receiver"));
                    }
                }
            }
        }
    };

    walk_document(source, dialect, analysis, &mut visit);
    if hazard.is_none() {
        walk_self_dispatch(source, dialect, analysis, target, &mut |span| {
            hazard = Some((span, "computed-member"));
        });
    }

    let (span, kind) = hazard?;
    let written = slice(source, span).unwrap_or("");
    let reason = if kind == "untracked-receiver" {
        format!(
            "cannot rename `{method}`: `{site}` is dispatched here on a receiver whose \
             class is not tracked, so the rename can neither prove this call reaches \
             `{method}` nor prove it does not. Rewriting only the declaration would \
             leave this call naming a member that no longer exists.",
            method = target.method,
            site = written,
        )
    } else {
        format!(
            "cannot rename `{method}`: a member name computed at run time is dispatched \
             here on a receiver of this class, so no edit can keep this call consistent \
             with the renamed declaration.",
            method = target.method,
        )
    };
    Some(RenameRefusal::new(reason, source, line_index, Some(span)))
}

/// Run `visit` over every command in every region of the document a dispatch
/// can be written in — the top-level stream, every proc body, and every
/// class member body, descending both same-frame and frame-shifted nested
/// regions exactly as the reference scan does.
///
/// Shared with [`crate::namespace_rename`] so the two rename gates scan the
/// same regions — a gate that visits fewer regions than its edit collector is
/// a hollow guarantee (issue #1092).
pub(crate) fn walk_document(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    visit: &mut impl FnMut(&SegmentedCommand),
) {
    walk_region(source, dialect, 0, source.len(), 0, visit);
    for proc_def in analysis.all_procs.values() {
        walk_body(source, dialect, proc_def.body_span, visit);
    }
    for class_def in analysis.all_classes.values() {
        for m in class_def
            .methods
            .values()
            .chain(class_def.class_methods.values())
            .chain(class_def.constructors.iter())
            .chain(class_def.destructor.iter())
        {
            walk_body(source, dialect, m.body_span, visit);
        }
    }
}

/// `my $computed` inside a family class's own member bodies — the
/// self-dispatch counterpart of the `$obj $computed` hazard, recognised
/// through the registry's own dispatch-keyword table rather than a literal
/// `my` comparison.
fn walk_self_dispatch(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    hit: &mut impl FnMut(Span),
) {
    let mut found: Option<Span> = None;
    for class_q in target.family {
        let Some(class_def) = analysis.all_classes.get(class_q) else {
            continue;
        };
        for body in collect_member_bodies_scoped(class_def, target.is_classmethod) {
            walk_body(source, dialect, body, &mut |cmd: &SegmentedCommand| {
                if found.is_some() {
                    return;
                }
                let (Some(head), Some(member)) = (cmd.argv.first(), cmd.argv.get(1)) else {
                    return;
                };
                if !matches!(member.kind, TokenType::Var | TokenType::Cmd) {
                    return;
                }
                let Some(head_text) = slice(source, head.span) else {
                    return;
                };
                if crate::definition::method_dispatch_keyword_in(dialect, head_text)
                    == Some(tcl_registry::registry::MethodDispatchKind::SelfDispatch)
                {
                    found = Some(member.span);
                }
            });
        }
    }
    if let Some(span) = found {
        hit(span);
    }
}

fn walk_body(
    source: &str,
    dialect: &str,
    body_span: Span,
    visit: &mut impl FnMut(&SegmentedCommand),
) {
    if body_span.is_empty() {
        return;
    }
    let (start, end) = strip_outer_braces(source, body_span);
    if start >= end {
        return;
    }
    walk_region(source, dialect, start, end, 0, visit);
}

fn walk_region(
    source: &str,
    dialect: &str,
    start: usize,
    end: usize,
    depth: u32,
    visit: &mut impl FnMut(&SegmentedCommand),
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    if start >= end || end > source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    let commands = segment_commands_with_offset_and_config(
        &source[start..end],
        u32::try_from(start).unwrap_or(0),
        tcl_lexer::LexerConfig::for_dialect(dialect),
    );
    for cmd in &commands {
        visit(cmd);
        for (inner_start, inner_end) in dispatch_scan_regions(source, dialect, cmd) {
            walk_region(source, dialect, inner_start, inner_end, depth + 1, visit);
        }
    }
}

fn slice(source: &str, span: Span) -> Option<&str> {
    let start = span.start() as usize;
    let end = span.end() as usize;
    (start <= end && end <= source.len()).then(|| &source[start..end])
}

/// Whether a **namespace-variable** rename must be refused because this
/// document computes a variable name that could be the cell being renamed.
///
/// The cross-document namespace-variable rename enumerates occurrences by
/// name: the declaration, every alias Tcl treats as the same cell
/// (`variable` / `global` / `namespace upvar`), and every `::`-qualified
/// occurrence.  A **computed** name in a variable-name argument position
/// (`set ::ns::$n 1`, `variable $n`, `namespace upvar ::ns $n local`) names
/// a cell no enumeration can predict, so a rename that touches that
/// namespace cannot be completed from source edits.
///
/// Which argument of which command names a variable is the registry's
/// answer ([`tcl_registry::ArgRole::VarWrite`] /
/// [`tcl_registry::ArgRole::VarRead`]); whether a word is computed is
/// [`tcl_compiler::dynamic_names::names_a_dynamic_variable`]'s.  No command
/// name is matched here.
///
/// # Per-site provenance (issue #1093)
///
/// The refusal is **per site**, not per document: a dynamic word refuses only
/// when it can be proved *unprovable* — when the names it can produce include
/// a spelling of this cell.  A word's written text already bounds that set,
/// because a substitution can evaluate to anything but the literal characters
/// around it cannot change, so
/// [`tcl_compiler::dynamic_names::dynamic_variable_word_can_spell`] treats the
/// word as a pattern and the cell's spellings as the candidates.  `set
/// ::other::$n 1` beside a rename of `::ns::v` no longer refuses; `variable
/// $n` inside `::ns` still does, which is the issue's own oracle
/// (`namespace eval ns { variable v 1; proc bump {n} {variable $n; set $n 2} }`
/// really does reach the cell through `ns::bump v`).
///
/// Abstain-toward-refuse is unchanged — the predicate answers "could spell it"
/// for everything it cannot rule out, and a lone `$n` is a bare wildcard that
/// rules nothing out.  The residual is a *value*-set fact this text-level
/// bound cannot see: a `$n` whose dominating constant provably differs from
/// the cell's tail still refuses, because no per-site constant-value record
/// survives analysis for a post-analysis consumer to read.
#[must_use]
pub fn namespace_variable_rename_hazard(
    source: &str,
    dialect: &str,
    analysis: &AnalysisResult,
    cell: &str,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let registry = tcl_registry::registry_for_dialect(dialect);
    let mut hazard: Option<Span> = None;
    let mut visit = |cmd: &SegmentedCommand| {
        if hazard.is_some() {
            return;
        }
        let Some(cmd_name) = cmd.texts.first() else {
            return;
        };
        let args: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
        for role in [
            tcl_registry::ArgRole::VarWrite,
            tcl_registry::ArgRole::VarRead,
        ] {
            for idx in registry.arg_indices_for_role(cmd_name, &args, role) {
                let Some(word) = args.get(idx) else {
                    continue;
                };
                if !tcl_compiler::dynamic_names::names_a_dynamic_variable(word) {
                    continue;
                }
                // Per-site provenance: a word that provably cannot spell this
                // cell is not a hazard for *this* rename, however dynamic it
                // is.
                if !tcl_compiler::dynamic_names::dynamic_variable_word_can_spell(word, cell) {
                    continue;
                }
                if let Some(tok) = cmd.argv.get(idx + 1) {
                    hazard = Some(tok.span);
                    return;
                }
            }
        }
    };
    walk_document(source, dialect, analysis, &mut visit);
    let span = hazard?;
    let written = slice(source, span).unwrap_or("");
    Some(RenameRefusal::new(
        format!(
            "cannot rename `{cell}`: this document names a variable through a value \
             computed at run time (`{written}`), which may be this very cell — there is \
             no word to rewrite, so the rename cannot be completed from source edits.",
        ),
        source,
        line_index,
        Some(span),
    ))
}

/// Refuse a **variable** rename whose target's name can only be written
/// quoted — `{$n}` / `${$n}`, `{a b}` / `${a b}`, `{[gen]}`.
///
/// Such a variable is perfectly legal and, since #1078, correctly modelled as
/// its own cell (tclsh 9.0.4 / 8.6.14: `set {$n} v; info exists {$n}` → 1
/// while `info exists n` → 0).  What it is *not* is renameable by span
/// substitution, which is all the variable-rename edit builder does:
///
/// * the recorded spans cover a word's **content**, not its delimiters — the
///   `{$n}` definition span starts at the `{` and stops before the `}`, so
///   writing `q` over it yields `set q} 1`;
/// * whether the *new* name needs delimiters at all is a property of the new
///   name, so every occurrence would need its quoting recomputed — including
///   read sites spelt `${$n}`, whose `${…}` may or may not survive.
///
/// Guessing produces a script that no longer parses, so the rename is refused
/// with the reason, following the #1091 typed-refusal precedent.  A rename
/// **to** such a name never gets this far: `is_safe_symbol_name` already
/// rejects new names carrying `$` / whitespace / brackets.
#[must_use]
pub fn literal_name_variable_rename_refusal(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let off = crate::definition::byte_offset_at(line_index, source, line, character);
    let def = crate::definition::var_def_at_declaration_offset(&analysis.global_scope, off)?;
    if !tcl_syntax::naming::var_name_requires_quoting(&def.name) {
        return None;
    }
    Some(RenameRefusal::new(
        format!(
            "cannot rename `{name}`: the variable's name can only be written quoted \
             (`{{{name}}}` / `${{{name}}}`), so each occurrence carries delimiters that \
             are not part of the recorded name, and the delimiters a new name needs \
             depend on that new name. Rewriting the recorded spans would produce a \
             script that no longer parses, so the rename is refused rather than guessed.",
            name = def.name,
        ),
        source,
        line_index,
        Some(def.definition_span),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        analyse_as(source, "tcl8.6")
    }

    fn analyse_as(source: &str, dialect: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, dialect).clone()
    }

    fn hazard(source: &str, family: &[&str], method: &str, is_classmethod: bool) -> Option<String> {
        hazard_to(source, family, method, is_classmethod, "Renamed", "tcl8.6")
    }

    /// [`hazard`] with the requested new name (and dialect) spelled out — the
    /// destination-sensitive arms need it.  `hazard` passes a name no fixture
    /// declares, so the destination arms stay silent for the older cases.
    fn hazard_to(
        source: &str,
        family: &[&str],
        method: &str,
        is_classmethod: bool,
        new_name: &str,
        dialect: &str,
    ) -> Option<String> {
        let analysis = analyse_as(source, dialect);
        let owned: Vec<String> = family.iter().map(|s| (*s).to_string()).collect();
        method_rename_hazard(
            source,
            dialect,
            &analysis,
            MethodRenameTarget {
                family: &owned,
                method,
                is_classmethod,
                new_name,
            },
            &LineIndex::new(source),
        )
        .map(|r| r.reason)
    }

    /// Issue #923 differential-audit finding idx 79, in its own shape.
    ///
    /// nico-robert/tomato's `Vector3d.tcl` copy-constructor: `$other` is
    /// `[lindex $args 0]` guarded by a runtime `info object isa` test, so it
    /// really is a `Vector3d` at run time (tclsh 9.0.4 / 8.6.16 both print
    /// `v2 = 7 9`) but the analyser has no binding for it.  The rename used to
    /// emit an edit set touching only the declaration; applying exactly that
    /// and re-running gives, on both interpreters:
    ///
    /// ```text
    /// unknown method "X": must be Get, GetX, Y or destroy
    ///     while executing
    /// "$other X"
    /// ```
    ///
    /// FP guard: the gate must refuse.
    #[test]
    fn fp_refuses_a_method_dispatched_on_an_untracked_receiver() {
        let src = "oo::class create Vector3d {\n\
                   \x20   variable _x\n\
                   \x20   constructor {args} {\n\
                   \x20       set other [lindex $args 0]\n\
                   \x20       if {[info object isa typeof $other ::Vector3d]} {\n\
                   \x20           set _x [$other X]\n\
                   \x20       }\n\
                   \x20   }\n\
                   \x20   method X {} { return $_x }\n\
                   }\n";
        let reason = hazard(src, &["::Vector3d"], "X", false)
            .expect("an untracked `$other X` dispatch must refuse the rename");
        assert!(
            reason.contains("not tracked"),
            "refusal must name the untracked receiver: {reason}"
        );
    }

    /// FN guard: the ordinary, provable shape must still rename.  `$v` is
    /// bound by `set v [Dog new]`, so its dispatch is found and rewritten by
    /// the reference scan — nothing to refuse.
    #[test]
    fn fn_guard_allows_a_method_whose_receivers_are_all_tracked() {
        let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   }\n\
                   set v [Dog new]\n\
                   puts [$v speak]\n";
        assert_eq!(hazard(src, &["::Dog"], "speak", false), None);
    }

    /// FN guard: an untracked receiver dispatching some *other* member is not
    /// a hazard for this rename — refusing on it would block almost every
    /// real rename, and it cannot affect the member being renamed.
    #[test]
    fn fn_guard_allows_when_the_untracked_receiver_names_another_member() {
        let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   \x20   method walk {} { return ok }\n\
                   \x20   constructor {other} { $other walk }\n\
                   }\n";
        assert_eq!(hazard(src, &["::Dog"], "speak", false), None);
    }

    /// TN: a receiver tracked to an *unrelated* class provably dispatches
    /// that class's table, so a same-named member elsewhere is untouched and
    /// unblocked.  tclsh 9.0.4 / 8.6.16 confirm the two `speak`s are
    /// independent methods on independent objects.
    #[test]
    fn tn_allows_when_the_receiver_is_tracked_to_an_unrelated_class() {
        let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   }\n\
                   oo::class create Cat {\n\
                   \x20   method speak {} { return meow }\n\
                   }\n\
                   set c [Cat new]\n\
                   puts [$c speak]\n";
        assert_eq!(hazard(src, &["::Dog"], "speak", false), None);
    }

    /// FP guard: a member name computed at run time, dispatched on a receiver
    /// of this very class.  No edit can keep `$v $m` consistent with a
    /// renamed declaration, and nothing proves `$m` is not the renamed
    /// member.
    #[test]
    fn fp_refuses_a_computed_member_name_on_a_tracked_receiver() {
        let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   }\n\
                   set v [Dog new]\n\
                   set m speak\n\
                   puts [$v $m]\n";
        let reason = hazard(src, &["::Dog"], "speak", false)
            .expect("a computed member name on an instance of the class must refuse");
        assert!(
            reason.contains("computed at run time"),
            "refusal must name the computed dispatch: {reason}"
        );
    }

    /// FP guard: the same, through the registry's own self-dispatch keyword
    /// inside the class's own bodies (`my $m`) — recognised via
    /// [`crate::definition::method_dispatch_keyword_in`], never a literal
    /// `== "my"`.
    #[test]
    fn fp_refuses_a_computed_member_name_dispatched_through_my() {
        let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   \x20   method run {m} { my $m }\n\
                   }\n";
        let reason = hazard(src, &["::Dog"], "speak", false)
            .expect("a `my $m` dispatch inside the class must refuse");
        assert!(reason.contains("computed at run time"), "{reason}");
    }

    /// FN guard: a plain `my speak` is exactly what the reference scan finds
    /// and rewrites — never a refusal.
    #[test]
    fn fn_guard_allows_a_literal_my_dispatch() {
        let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   \x20   method run {} { my speak }\n\
                   }\n";
        assert_eq!(hazard(src, &["::Dog"], "speak", false), None);
    }

    /// FP guard, issue #981's object-command half: two classes binding the
    /// *same qualified* object command.  `::a::Factory create rex` twice over
    /// (here with two different classes in one namespace) leaves which class
    /// a later `rex make` reaches a runtime fact — the second `create`
    /// replaces the first command.
    #[test]
    fn fp_refuses_when_two_classes_bind_the_same_qualified_object_command() {
        let src = "namespace eval ::a {\n\
                   \x20   oo::class create Factory { method make {} { return f } }\n\
                   \x20   oo::class create Widget { method make {} { return w } }\n\
                   \x20   Factory create rex\n\
                   \x20   Widget create rex\n\
                   \x20   puts [rex make]\n\
                   }\n";
        let reason = hazard(src, &["::a::Factory"], "make", false)
            .expect("an object command bound by two classes must refuse");
        assert!(
            reason.contains("more than one class"),
            "refusal must name the ambiguity: {reason}"
        );
    }

    /// FN guard: the same bare name bound in *different* namespaces is not
    /// ambiguous at all — `::a::rex` and `::b::rex` are two distinct commands
    /// (tclsh 9.0.4 / 8.6.16: `rex make` inside `::a` prints `a-made`, inside
    /// `::b` prints `b-made`, and both object commands coexist).
    #[test]
    fn fn_guard_allows_same_bare_object_command_in_sibling_namespaces() {
        let src = "namespace eval ::a {\n\
                   \x20   oo::class create Factory { method make {} { return a } }\n\
                   \x20   Factory create rex\n\
                   }\n\
                   namespace eval ::b {\n\
                   \x20   oo::class create Widget { method make {} { return b } }\n\
                   \x20   Widget create rex\n\
                   }\n";
        assert_eq!(hazard(src, &["::a::Factory"], "make", false), None);
    }

    fn var_hazard(source: &str, cell: &str) -> Option<String> {
        let analysis = analyse(source);
        namespace_variable_rename_hazard(source, "tcl8.6", &analysis, cell, &LineIndex::new(source))
            .map(|r| r.reason)
    }

    /// FP guard: a computed variable name in a registry-declared variable-name
    /// argument position may be naming the very cell being renamed, and there
    /// is no word to rewrite.
    #[test]
    fn fp_refuses_a_namespace_variable_rename_beside_a_computed_name() {
        let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {n} { set $n 2 }\n\
                   }\n";
        let reason = var_hazard(src, "::ns::v")
            .expect("a `set $n` in the same document must refuse the cell rename");
        assert!(reason.contains("computed at run time"), "{reason}");
    }

    /// FN guard: the ordinary namespace-variable document has no dynamic
    /// name and must rename freely.
    #[test]
    fn fn_guard_allows_a_namespace_variable_rename_with_static_names() {
        let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} { variable v; return $v }\n\
                   }\n\
                   puts $::ns::v\n";
        assert_eq!(var_hazard(src, "::ns::v"), None);
    }

    // Issue #1093 — per-site provenance.  The refusal now asks whether *this*
    // word can spell *this* cell, instead of refusing on any dynamic word
    // anywhere in the document.

    /// TN: a dynamic word under a different, statically-written namespace
    /// cannot spell a cell in `::ns`, so the rename proceeds.
    ///
    /// tclsh-proof (8.6.14): `namespace eval ::ns {variable v 1}; namespace
    /// eval ::other {}; set n {::ns::v}; set ::other::$n 99` fails with
    /// `can't set "::other::::ns::v": parent namespace doesn't exist` — the
    /// value lands *under* the written prefix, so no value of `$n` reaches
    /// `::ns::v`.
    #[test]
    fn tn_a_dynamic_name_under_another_namespace_no_longer_refuses() {
        let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   }\n\
                   namespace eval ::other {\n\
                   \x20   proc p {n} { set ::other::$n 2 }\n\
                   }\n";
        assert_eq!(
            var_hazard(src, "::ns::v"),
            None,
            "`::other::$n` cannot name `::ns::v`",
        );
    }

    /// TP control for the pair above: the same word *under the renamed
    /// namespace* still refuses — the narrowing is a narrowing, not a
    /// disabling.
    #[test]
    fn tp_a_dynamic_name_under_the_renamed_namespace_still_refuses() {
        let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {n} { set ::ns::$n 2 }\n\
                   }\n";
        let reason = var_hazard(src, "::ns::v").expect("`::ns::$n` can name the cell");
        assert!(reason.contains("computed at run time"), "{reason}");
    }

    /// TN: a written affix bounds the name too — `${p}_count` can never spell
    /// `v` (tclsh-proof, 8.6.14: `set j 1; set v$j 2` leaves `::ns::total`
    /// untouched and creates only `::v1`).
    #[test]
    fn tn_a_dynamic_name_with_an_unsatisfiable_affix_no_longer_refuses() {
        let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {pfx} { set ${pfx}_count 2 }\n\
                   }\n";
        assert_eq!(var_hazard(src, "::ns::v"), None);
    }

    /// TP (CRITICAL): the issue's own oracle shape must stay refused — a bare
    /// `$n` is a lone wildcard and really does reach the cell.
    ///
    /// tclsh-proof (8.6.14): `namespace eval ns {variable v 1; proc bump {n}
    /// {variable $n; set $n 2}}; ns::bump v` leaves `set ::ns::v` -> 2.
    #[test]
    fn tp_the_bare_dynamic_name_oracle_still_refuses() {
        let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc bump {n} { variable $n; set $n 2 }\n\
                   }\n";
        let reason = var_hazard(src, "::ns::v").expect("`variable $n` really does reach the cell");
        assert!(reason.contains("computed at run time"), "{reason}");
    }

    // Issue #1121 review — a moved member's declaration site is the
    // `renamemethod`'s destination word, so renaming it rewrites that word and
    // some new names produce a body real Tcl refuses to run.  The gate reads
    // the same `RenamedMember` fold the W315 diagnostic does.

    /// TP: `renamemethod old new` + renaming `new` back to `old` would emit
    /// `renamemethod old old`, which fails `cannot rename method to itself`
    /// and creates no class (tclsh 9.0.4 / 8.6.14).
    #[test]
    fn tp_refuses_renaming_a_moved_member_to_its_own_source() {
        let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   renamemethod old new\n\
                   }\n";
        let reason = hazard_to(src, &["::C"], "new", false, "old", "tcl9.0")
            .expect("renaming the moved member to its source must refuse");
        assert!(
            reason.contains("cannot rename method to itself"),
            "{reason}"
        );
    }

    /// TP: renaming it onto a **live same-side sibling** would emit
    /// `renamemethod old sib` -> `method called sib already exists`.
    #[test]
    fn tp_refuses_renaming_a_moved_member_onto_a_live_sibling() {
        let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   \x20   renamemethod old new\n\
                   }\n";
        let reason = hazard_to(src, &["::C"], "new", false, "sib", "tcl9.0")
            .expect("renaming the moved member onto a live sibling must refuse");
        assert!(
            reason.contains("method called sib already exists"),
            "{reason}"
        );
    }

    /// TP, the mirror: renaming the ordinary sibling *into* the destination a
    /// later `renamemethod` needs free collides at that same site.
    #[test]
    fn tp_refuses_renaming_a_sibling_onto_a_later_rename_destination() {
        let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   \x20   renamemethod old new\n\
                   }\n";
        let reason = hazard_to(src, &["::C"], "sib", false, "new", "tcl9.0")
            .expect("renaming a sibling onto a later rename destination must refuse");
        assert!(
            reason.contains("method called new already exists"),
            "{reason}"
        );
    }

    /// TN: a fresh destination is a legal rename and must not be refused.
    #[test]
    fn tn_allows_renaming_a_moved_member_to_a_fresh_name() {
        let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   \x20   renamemethod old new\n\
                   }\n";
        assert_eq!(
            hazard_to(src, &["::C"], "new", false, "fresh", "tcl9.0"),
            None
        );
    }

    /// TN (CRITICAL): a name live only on the **other** side is not a
    /// collision — `method old {} {…}` + `self { method sib {} {…} }` +
    /// `renamemethod old sib` runs fine on 9.0.4 and 8.6.14.
    #[test]
    fn tn_allows_renaming_a_moved_member_onto_a_cross_side_name() {
        let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   self { method sib {} { return 2 } }\n\
                   \x20   renamemethod old new\n\
                   }\n";
        assert_eq!(
            hazard_to(src, &["::C"], "new", false, "sib", "tcl9.0"),
            None
        );
    }

    /// TN: a destination deleted *before* the move is free — the snapshot is
    /// taken at the move's own point in the body, exactly as Tcl reads it.
    #[test]
    fn tn_allows_renaming_a_moved_member_onto_an_earlier_deleted_name() {
        let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   \x20   deletemethod sib\n\
                   \x20   renamemethod old new\n\
                   }\n";
        assert_eq!(
            hazard_to(src, &["::C"], "new", false, "sib", "tcl9.0"),
            None
        );
    }

    /// TN (CRITICAL, issue #1178 review): the hazard is **side-local**. An
    /// instance-side `renamemethod old same` says nothing about a
    /// class-object-side `same`, so renaming that class-side member to `old` is
    /// legal and must not be refused. Oracle, byte-identical on tclsh 9.0.4 and
    /// 8.6.14 — the post-edit body creates the class and both members dispatch:
    ///
    /// ```tcl
    /// oo::class create ::P2 { method old {} { return inst-old }
    ///                         renamemethod old same
    ///                         self { method old {} { return cls-old } } }
    /// info class methods ::P2   ;# -> same        [::P2 new] same -> inst-old
    /// info object methods ::P2  ;# -> old         ::P2 old        -> cls-old
    /// ```
    #[test]
    fn tn_an_instance_side_move_does_not_gate_a_class_side_rename() {
        let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   renamemethod old same\n\
                   \x20   self { method same {} { return 2 } }\n\
                   }\n";
        assert_eq!(
            hazard_to(src, &["::C"], "same", true, "old", "tcl9.0"),
            None,
            "an instance-side move must not refuse a class-side rename",
        );
    }

    /// TN, the mirror: a class-object-side move says nothing about an
    /// instance-side member of the same name.
    #[test]
    fn tn_a_class_side_move_does_not_gate_an_instance_side_rename() {
        let src = "oo::class create C {\n\
                   \x20   self { method old {} { return 1 }\n\
                   \x20          renamemethod old same }\n\
                   \x20   method same {} { return 2 }\n\
                   }\n";
        assert_eq!(
            hazard_to(src, &["::C"], "same", false, "old", "tcl9.0"),
            None,
            "a class-side move must not refuse an instance-side rename",
        );
    }

    /// TP control for the pair above: on the **same** side the refusal still
    /// fires, so the side filter narrowed the arm rather than disabling it.
    #[test]
    fn tp_the_side_filter_keeps_the_same_side_refusal() {
        let src = "oo::class create C {\n\
                   \x20   self { method old {} { return 1 }\n\
                   \x20          renamemethod old same }\n\
                   }\n";
        let reason = hazard_to(src, &["::C"], "same", true, "old", "tcl9.0")
            .expect("a class-side move must still gate a class-side rename");
        assert!(
            reason.contains("cannot rename method to itself"),
            "{reason}"
        );
    }

    /// TN: the mirror arm is side-local too — an instance-side sibling renamed
    /// into a *class-side* move's destination collides with nothing.
    #[test]
    fn tn_the_mirror_arm_is_side_local() {
        let src = "oo::class create C {\n\
                   \x20   self { method old {} { return 1 }\n\
                   \x20          method sib {} { return 2 }\n\
                   \x20          renamemethod old new }\n\
                   \x20   method sib {} { return 3 }\n\
                   }\n";
        assert_eq!(
            hazard_to(src, &["::C"], "sib", false, "new", "tcl9.0"),
            None,
            "an instance-side `sib` is not the class-side move's blocker",
        );
    }

    /// TN: a class with no `renamemethod` at all is untouched by the new arm.
    #[test]
    fn tn_allows_an_ordinary_member_rename_with_no_renamemethod_in_the_body() {
        let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   }\n";
        assert_eq!(
            hazard_to(src, &["::C"], "old", false, "sib", "tcl9.0"),
            None
        );
    }
}
