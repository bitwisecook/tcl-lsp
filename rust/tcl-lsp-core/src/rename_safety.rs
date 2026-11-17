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
//! ## Untracked receivers
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
//! scan ([`crate::references::method_references_for_class`]) correctly does
//! not match the site.  An edit set that touched only the declaration would,
//! applied and run under both interpreters, give
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
//! exported. [`crate::references::member_reference_spans`] locates readonly
//! hazards through original source class and definition-body owners. An edit
//! requires the independent original selected worker and class-allocation
//! receipt; missing references or receipts refuse the rename.
//!
//! ## Ambiguous object commands
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
//! ([`tcl_compiler::dynamic_names::source_name_word_is_dynamic`]) in a
//! registry-declared variable-name argument position
//! ([`tcl_registry::ArgRole::VarWrite`] / [`tcl_registry::ArgRole::VarRead`])
//! may be naming the very cell being renamed, with no word to rewrite — so
//! a rename that would touch that namespace is refused.
//!
//! The refusal is decided **per site**, not per document: a
//! word's written text bounds the names it can produce, because a
//! substitution can evaluate to anything but the literal characters around it
//! cannot change. In a current supplied Logical authored-C domain,
//! `set ::other::$n 1` therefore stops refusing a rename of
//! `::ns::v` — nothing that word can spell is under `::ns` — while `variable
//! $n` inside `::ns` still refuses.  See
//! [`namespace_variable_rename_hazard`] for the bound and what stays outside
//! it.
//!
//! # No command names here
//!
//! Every command-level fact this module uses comes from the registry: the
//! self-dispatch keyword ([`tcl_registry::Traits::TCLOO_SELF_DISPATCH`]),
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
use crate::references::{collect_member_bodies_scoped, strip_var_decoration};

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
    /// ([`renamed_member_would_abort`]).
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
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    if crate::references::dispatch_source_context(source, analysis).is_none() {
        return Some(RenameRefusal::new(
            format!(
                "cannot rename `{}`: the complete original source and metadata are unavailable",
                target.method
            ),
            source,
            line_index,
            None,
        ));
    }
    if original_member_coverage_proved(source, analysis, target) {
        return renamed_member_would_abort(source, analysis, target, line_index);
    }
    for class in target
        .family
        .iter()
        .filter_map(|name| analysis.all_classes.get(name))
    {
        let method = if target.is_classmethod {
            &class.class_methods
        } else {
            &class.methods
        }
        .get(target.method);
        if let Some(method) = method
            && tcl_compiler::analyser::types::original_source_linked_member_reference_spans(
                analysis,
                source,
                class,
                method,
                if target.is_classmethod {
                    tcl_compiler::analyser::MemberSide::ClassObject
                } else {
                    tcl_compiler::analyser::MemberSide::Instance
                },
            )
            .is_none()
        {
            return Some(RenameRefusal::new(
                "cannot prove every original link target and coupled alias edit".to_owned(),
                source,
                line_index,
                Some(method.name_span),
            ));
        }
    }
    unlocatable_member_reference(source, dialect, analysis, target, line_index)
        .or_else(|| renamed_member_would_abort(source, analysis, target, line_index))
        .or_else(|| {
            unproved_definition_member_reference(source, dialect, analysis, target, line_index)
        })
        .or_else(|| {
            unproved_callback_method_selector(source, dialect, analysis, target, line_index)
        })
        .or_else(|| computed_method_selector(source, analysis, target, line_index))
        .or_else(|| dispatch_hazard(source, dialect, analysis, target, line_index))
        .or_else(|| ambiguous_object_command(source, analysis, target, line_index))
}

/// Whether the original member rename planner's complete walk of the current
/// source closes every dispatch, receiver, link and family obligation of this
/// member, for a family equal to the caller's declaring family. The walk is
/// name-independent; destination hazards stay with their own arms.
fn original_member_coverage_proved(
    source: &str,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
) -> bool {
    if analysis.allows_lexical_declaration_advice() {
        return false;
    }
    let side = if target.is_classmethod {
        MemberSide::ClassObject
    } else {
        MemberSide::Instance
    };
    let declared = target
        .family
        .iter()
        .flat_map(|name| {
            analysis
                .original_class_declarations()
                .filter(move |record| record.metadata().qualified_name == *name)
        })
        .flat_map(|record| {
            record
                .metadata()
                .original_members
                .declarations()
                .filter(|row| {
                    row.side() == side
                        && row.original_name_input().bytes() == target.method.as_bytes()
                })
                .map(move |row| (record, row))
        })
        .collect::<Vec<_>>();
    let Some(&(record, row)) = declared.first() else {
        return false;
    };
    let Some(identity) = crate::original_declaration::OriginalDeclarationIdentity::for_method(
        "", source, analysis, record, row,
    ) else {
        return false;
    };
    let document = crate::original_declaration::OriginalDeclarationDocument {
        uri: "",
        source,
        analysis,
    };
    if crate::original_member_rename::original_member_rename_edits(
        &[document],
        &identity,
        target.method,
    )
    .is_err()
    {
        return false;
    }
    let mut planned =
        crate::original_member_rename::original_member_family_spans(document, &identity);
    if planned.is_empty() {
        planned.push(identity.span());
    }
    let mut wanted = declared
        .iter()
        .map(|(_, row)| {
            row.declaration()
                .static_occurrence()
                .map(|occurrence| occurrence.name_input().span())
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();
    planned.sort_by_key(|span| (span.start(), span.end()));
    wanted.sort_by_key(|span| (span.start(), span.end()));
    planned == wanted
}

/// Definition metadata syntax locates a possible hazard, never an edit.
/// The selected private-worker/class-allocation receipt must close this site.
fn unproved_definition_member_reference(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let mut unproved = None;
    for name in target.family {
        let inventory = analysis.all_classes.get(name).and_then(|class| {
            crate::references::member_reference_inventory(
                source,
                analysis,
                dialect,
                class,
                target.method,
            )
        });
        let Some(inventory) = inventory else {
            return Some(RenameRefusal::new(
                format!(
                    "cannot rename `{}`: the original definition metadata reference inventory is unavailable.",
                    target.method
                ),
                source,
                line_index,
                None,
            ));
        };
        if let Some(span) = inventory.into_iter().find(|span| {
            crate::receiver_identity::definition_reference_at_cursor(analysis, source, span.start())
                .is_none()
        }) {
            unproved = Some(span);
            break;
        }
    }
    let span = unproved?;
    Some(RenameRefusal::new(
        format!(
            "cannot rename `{}`: a definition metadata reference lacks its original selected worker and class allocation receipt.",
            target.method
        ),
        source,
        line_index,
        Some(span),
    ))
}

fn unproved_callback_method_selector(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let span = crate::references::unproved_callback_method_selector(
        source,
        dialect,
        analysis,
        target.method,
    )?;
    Some(RenameRefusal::new(
        format!(
            "cannot rename `{}`: a captured callback method lacks its original receiver allocation receipt.",
            target.method
        ),
        source,
        line_index,
        Some(span),
    ))
}

/// A temporal dispatch can be known while its substituted selector cannot be
/// edited. Omitting that operand would leave a stale method name after rename.
fn computed_method_selector(
    source: &str,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let (selector, coverage) =
        crate::receiver_identity::uneditable_method_selector(analysis, source, |selected| {
            selected.method.name == target.method
                && (selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class)
                    == target.is_classmethod
                && target.family.iter().any(|class| {
                    analysis.all_classes.get(class).is_some_and(|class| {
                        let members = if target.is_classmethod {
                            &class.class_methods
                        } else {
                            &class.methods
                        };
                        members
                            .get(target.method)
                            .is_some_and(|method| method.name_span == selected.method.name_span)
                    })
                })
        });
    let Some(selector) = selector else {
        return (coverage != crate::executable_regions::PotentialSourceVisit::Complete
            && !target.family.is_empty())
        .then(|| incomplete_hazard_census(source, line_index, target.method));
    };
    Some(RenameRefusal::new(
        format!(
            "cannot rename `{}`: a selected dispatch uses a computed method name without exact editable contributor proof.",
            target.method,
        ),
        source,
        line_index,
        Some(selector),
    ))
}

/// The requested new name would turn a `renamemethod` in this document into one
/// that **aborts the whole class definition**.
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
        // filter an instance-side `renamemethod old same` would refuse an
        // unrelated class-side `self method same` -> `old`, which real Tcl
        // runs happily. Oracle, byte-identical on 9.0.4 and 8.6.14:
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
    dialect: &'static tcl_dialect::DialectProfile,
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
        // breaks an instance's — so the class-side sets have to
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
        if crate::references::member_reference_spans(
            source,
            analysis,
            dialect,
            class_def,
            target.method,
        )
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
/// Namespace scoping tells `::a::rex` from `::b::rex`, but two
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
///   has no class binding at all (`$args X` / `$other X`).  The
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
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let (config, _) = crate::references::dispatch_source_context(source, analysis)?;
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
        if slice(source, head.span).is_none() {
            return;
        }
        let computed_member = matches!(member.kind, TokenType::Var | TokenType::Cmd);
        if head.kind == TokenType::Var {
            let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                cmd,
            );
            let receiver = original_words
                .word_exprs
                .first()
                .and_then(tcl_compiler::ir::WordExpr::sole_variable_substitution)
                .and_then(|(spelling, _)| strip_var_decoration(spelling, Some(config)));
            // Compound variable heads remain untracked hazards; a variable
            // prefix cannot narrow them with its own source class metadata.
            // Only a positioned current receiver can narrow a Native hazard.
            // Source constructor candidates never establish rename identity.
            let bound_class = if analysis.allows_lexical_declaration_advice() {
                receiver.and_then(|receiver| {
                    analysis.instance_classes.get(receiver).or_else(|| {
                        crate::definition::lattice_singleton_class(
                            analysis,
                            receiver,
                            head.span.start(),
                        )
                    })
                })
            } else {
                crate::receiver_identity::class_at_command_head(analysis, source, cmd)
                    .map(|class| &class.qualified_name)
            };
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

    let coverage = walk_document(source, dialect, analysis, &mut visit);
    let self_coverage = if hazard.is_none() {
        walk_self_dispatch(source, dialect, analysis, target, &mut |span| {
            hazard = Some((span, "computed-member"));
        })
    } else {
        coverage
    };
    if hazard.is_none()
        && (coverage != crate::executable_regions::PotentialSourceVisit::Complete
            || self_coverage != crate::executable_regions::PotentialSourceVisit::Complete)
    {
        return ambiguous_object_command(source, analysis, target, line_index)
            .or_else(|| Some(incomplete_hazard_census(source, line_index, target.method)));
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
/// a hollow guarantee.
pub(crate) fn walk_document(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    visit: &mut impl FnMut(&SegmentedCommand),
) -> crate::executable_regions::PotentialSourceVisit {
    if crate::references::dispatch_source_context(source, analysis).is_none() {
        return crate::executable_regions::PotentialSourceVisit::Unavailable;
    }
    crate::executable_regions::visit_checked_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            visit(command);
            false
        },
    )
}

fn incomplete_hazard_census(source: &str, line_index: &LineIndex, target: &str) -> RenameRefusal {
    RenameRefusal::new(
        format!(
            "cannot rename `{target}`: original potential source coverage is unavailable; an unrepresented region may contain a rename hazard"
        ),
        source,
        line_index,
        None,
    )
}

/// `my $computed` inside a family class's own member bodies — the
/// self-dispatch counterpart of the `$obj $computed` hazard, recognised
/// through the registry's own dispatch-keyword table rather than a literal
/// `my` comparison.
fn walk_self_dispatch(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    target: MethodRenameTarget<'_>,
    hit: &mut impl FnMut(Span),
) -> crate::executable_regions::PotentialSourceVisit {
    let Some((_, context)) = crate::references::dispatch_source_context(source, analysis) else {
        return crate::executable_regions::PotentialSourceVisit::Unavailable;
    };
    let mut coverage = crate::executable_regions::PotentialSourceVisit::Complete;
    let mut found: Option<Span> = None;
    for class_q in target.family {
        let Some(class_def) = analysis.all_classes.get(class_q) else {
            continue;
        };
        for body in collect_member_bodies_scoped(class_def, target.is_classmethod) {
            let body_coverage = walk_body(
                source,
                dialect,
                analysis,
                body,
                &mut |cmd: &SegmentedCommand| {
                    if found.is_some() {
                        return;
                    }
                    let Some(words) =
                        tcl_compiler::registry_invocation::source_structure::source_registry_words(
                            source, analysis, cmd,
                        )
                    else {
                        return;
                    };
                    let member = words
                        .with_source_schema(&context, |schema| {
                            if !schema
                                .semantics
                                .traits
                                .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
                            {
                                return None;
                            }
                            let ordinal = schema.semantics.argument_offset;
                            let tcl_compiler::registry_invocation::InvocationWordOrigin::Written(
                                written,
                            ) = words.origins().get(ordinal.checked_add(1)?)?
                            else {
                                return None;
                            };
                            cmd.argv.get(*written)?;
                            if schema.words.arguments().get(ordinal)?.literal().is_some() {
                                return None;
                            }
                            // A composite selector retains the whole actual
                            // operand; captures and expanded children cannot
                            // borrow a caller word's location.
                            let operand = words.operands().get(ordinal)?.as_ref()?;
                            operand.word()?;
                            Some(operand.span())
                        })
                        .flatten();
                    if let Some(span) = member {
                        found = Some(span);
                    }
                },
            );
            if body_coverage != crate::executable_regions::PotentialSourceVisit::Complete {
                coverage = body_coverage;
            }
        }
    }
    if let Some(span) = found {
        hit(span);
    }
    coverage
}

fn walk_body(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    body_span: Span,
    visit: &mut impl FnMut(&SegmentedCommand),
) -> crate::executable_regions::PotentialSourceVisit {
    if crate::references::dispatch_source_context(source, analysis).is_none() {
        return crate::executable_regions::PotentialSourceVisit::Unavailable;
    }
    crate::executable_regions::visit_checked_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if body_span.start() <= command.span.start() && command.span.end() <= body_span.end() {
                visit(command);
            }
            false
        },
    )
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
/// [`tcl_registry::ArgRole::VarRead`], and [`tcl_registry::ArgRole::VarCell`]
/// for the cell an `upvar` links); whether a word is computed is
/// [`tcl_compiler::dynamic_names::source_name_word_is_dynamic`]'s. No command
/// name is matched here.
///
/// # Per-site provenance
///
/// The refusal is **per site**, not per document: a dynamic word refuses only
/// when it can be proved *unprovable* — when the names it can produce include
/// a spelling of this cell.  A word's written text already bounds that set,
/// because a substitution can evaluate to anything but the literal characters
/// around it cannot change, so
/// [`tcl_compiler::dynamic_names::source_name_pattern_can_spell`] treats the
/// word as a pattern and the cell's spellings as the candidates, only for
/// current supplied Logical authored-C input and selected name roles. `set
/// ::other::$n 1` beside a rename of `::ns::v` does not refuse; `variable
/// $n` inside `::ns` does, which the oracle bears out
/// (`namespace eval ns { variable v 1; proc bump {n} {variable $n; set $n 2} }`
/// really does reach the cell through `ns::bump v`).
///
/// Abstain-toward-refuse is unchanged — the predicate answers "could spell it"
/// for everything it cannot rule out.
///
/// # Per-site value provenance
///
/// A lone `$n` is a bare wildcard, so the text bound alone rules nothing out.
/// The *value* set can: the analyser resolves each computed name word through
/// the constant lattice that **dominates** it and carries the answer out on
/// [`AnalysisResult::dynamic_variable_names`](tcl_compiler::analyser::AnalysisResult::dynamic_variable_names),
/// so `namespace eval ::ns { variable v 1; proc p {} { set n other; set $n 2 } }`
/// does not refuse a rename of `::ns::v` — that `$n` is provably `other`.
///
/// Narrowing only ([`site_resolution_rules_out`]): a site whose resolution is
/// branch-dependent or parameter-fed, and a site the walk never reached, keep
/// refusing.  The residual is everything the dominating
/// -constant lattice does not track — a value from a `[…]` substitution, a
/// caller-supplied parameter, an `upvar`-aliased name.
#[must_use]
pub fn namespace_variable_rename_hazard(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    cell: &str,
    line_index: &LineIndex,
) -> Option<RenameRefusal> {
    let Some((config, context)) = crate::references::dispatch_source_context(source, analysis)
    else {
        return Some(RenameRefusal::new(
            format!(
                "cannot rename `{cell}`: the complete original source and metadata are unavailable"
            ),
            source,
            line_index,
            None,
        ));
    };
    let metadata = analysis.resolved_input.as_ref().and_then(|input| {
        tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
            context.commands(),
            input,
            config,
            analysis.resolved_profile(),
        )
    });
    let purpose = tcl_compiler::dynamic_names::SourceNamePatternPurpose::VariableRoot;
    let mut hazard: Option<(Span, bool)> = None;
    let mut visit = |cmd: &SegmentedCommand| {
        if hazard.is_some() {
            return;
        }
        let words = tcl_compiler::registry_invocation::source_structure::source_registry_words(
            source, analysis, cmd,
        );
        let roles = words.as_ref().and_then(|words| {
            words
                .with_source_schema(&context, |_| {
                    words.roles().map(|_| words.written_argument_roles())
                })
                .flatten()
        });
        if let Some(span) = roles
            .as_deref()
            .and_then(|roles| computed_namespace_beside_tail(cmd, roles, cell))
        {
            hazard = Some((span, false));
            return;
        }
        for (ordinal, word) in cmd.texts.iter().skip(1).enumerate() {
            let Some(tok) = cmd.argv.get(ordinal + 1) else {
                continue;
            };
            // Known literal names, including braced `$` bytes, cannot be
            // mistaken for source interpolation or borrowed capture geometry.
            let static_word = cmd.single_token_word.get(ordinal + 1) == Some(&true)
                && matches!(tok.kind, TokenType::Esc | TokenType::Str);
            if static_word {
                continue;
            }
            let unknown = match &roles {
                Some(roles)
                    if roles.iter().any(|&(index, role)| {
                        index == ordinal
                            && matches!(
                                role,
                                tcl_registry::ArgRole::VarWrite
                                    | tcl_registry::ArgRole::VarRead
                                    | tcl_registry::ArgRole::VarCell,
                            )
                    }) =>
                {
                    false
                }
                Some(_) => continue,
                None => true,
            };
            if !unknown
                && (tcl_compiler::dynamic_names::source_name_word_is_dynamic(
                    word, purpose, metadata,
                ) == Some(false)
                    || tcl_compiler::dynamic_names::source_name_pattern_can_spell(
                        word, cell, purpose, metadata,
                    ) == Some(false))
            {
                continue;
            }
            if !unknown && site_resolution_rules_out(source, analysis, tok.span, cell, dialect) {
                continue;
            }
            hazard = Some((tok.span, unknown));
            return;
        }
    };
    let coverage = walk_document(source, dialect, analysis, &mut visit);
    if hazard.is_none() && coverage != crate::executable_regions::PotentialSourceVisit::Complete {
        return Some(incomplete_hazard_census(source, line_index, cell));
    }
    namespace_variable_hazard_refusal(source, line_index, cell, hazard)
}

/// A computed namespace word beside a literal cell word spelling the cell's
/// tail may link the cell itself (`namespace upvar $ns version local`).
fn computed_namespace_beside_tail(
    cmd: &SegmentedCommand,
    roles: &[(usize, tcl_registry::ArgRole)],
    cell: &str,
) -> Option<Span> {
    let tail = cell.rsplit("::").next()?;
    let literal = |ordinal: usize| {
        cmd.single_token_word.get(ordinal + 1) == Some(&true)
            && cmd
                .argv
                .get(ordinal + 1)
                .is_some_and(|tok| matches!(tok.kind, TokenType::Esc | TokenType::Str))
    };
    roles
        .iter()
        .any(|&(ordinal, role)| {
            role == tcl_registry::ArgRole::VarCell
                && literal(ordinal)
                && cmd.texts.get(ordinal + 1).map(String::as_str) == Some(tail)
        })
        .then_some(())?;
    roles.iter().find_map(|&(ordinal, role)| {
        (role == tcl_registry::ArgRole::NamespaceName && !literal(ordinal))
            .then(|| cmd.argv.get(ordinal + 1).map(|tok| tok.span))
            .flatten()
    })
}

fn namespace_variable_hazard_refusal(
    source: &str,
    line_index: &LineIndex,
    cell: &str,
    hazard: Option<(Span, bool)>,
) -> Option<RenameRefusal> {
    let (span, unknown) = hazard?;
    if unknown {
        return Some(RenameRefusal::new(
            format!(
                "cannot rename `{cell}`: original variable-name roles are unavailable at a computed operand"
            ),
            source,
            line_index,
            Some(span),
        ));
    }

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

/// Whether current original value provenance excludes `cell` at this site.
/// Missing or unresolved values retain the textual wildcard hazard.
fn site_resolution_rules_out(
    source: &str,
    analysis: &AnalysisResult,
    span: Span,
    cell: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
) -> bool {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    // Source role/value advice does not prove a Native or hosted current cell.
    // Logical compatibility may narrow its own source refusal independently.
    if !analysis.allows_lexical_declaration_advice() {
        return false;
    }
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return false;
    };
    let metadata = analysis.resolved_input.as_ref().and_then(|input| {
        tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
            current.registry(),
            input,
            current.config(),
            Some(current.profile()),
        )
    });
    analysis.dynamic_variable_names.iter().any(|site| {
        site.span == span
            && site.resolved.as_deref().is_some_and(|resolved| {
                tcl_compiler::dynamic_names::source_name_pattern_can_spell(
                    resolved,
                    cell,
                    tcl_compiler::dynamic_names::SourceNamePatternPurpose::VariableRoot,
                    metadata,
                ) == Some(false)
            })
    })
}

/// Refuse an ordinary **variable** rename whose target's name can only be written
/// quoted — `{$n}` / `${$n}`, `{a b}` / `${a b}`, `{[gen]}`.
///
/// Such a variable is perfectly legal and is modelled as its own cell
/// (tclsh 9.0.4 / 8.6.14: `set {$n} v; info exists {$n}` → 1
/// while `info exists n` → 0). Quoted spellings cannot be safely rewritten by
/// an unproved content-span substitution:
///
/// * the recorded spans cover a word's **content**, not its delimiters — the
///   `{$n}` definition span starts at the `{` and stops before the `}`, so
///   writing `q` over it yields `set q} 1`;
/// * whether the *new* name needs delimiters at all is a property of the new
///   name, so every occurrence would need its quoting recomputed — including
///   read sites spelt `${$n}`, whose `${…}` may or may not survive.
///
/// Guessing produces a script that no longer parses, so the rename is
/// refused with a typed reason.  A rename
/// **to** such a name never gets this far: `is_safe_symbol_name` already
/// rejects new names carrying `$` / whitespace / brackets.
/// Separate conditional source alpha proposals authenticate whole containers
/// and generated source; they do not use this ordinary-route refusal.
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
mod tests;

#[cfg(test)]
mod original_receiver_hazard_tests;

#[cfg(test)]
mod original_hazard_context_tests;

#[cfg(test)]
mod configured_dispatch_receiver_tests;

#[cfg(test)]
mod checked_rename_hazard_census_tests;
