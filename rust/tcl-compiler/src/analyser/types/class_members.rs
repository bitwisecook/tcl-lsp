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

//! The members of a class: methods, properties and per-object state, the side they sit on, and the retraction and rename records behind definition aborts.

use super::class_def::ClassDef;
use super::logical_object_members::LogicalSourceObjectMemberState;
use crate::signature_scan::types::ParamDef;
use std::collections::{HashMap, HashSet};
use tcl_lexer::Span;

/// Method definition inside a `TclOO` class.
///
/// Populated by the class-body walker; the shape is shared so the
/// class-hierarchy / MRO algorithms have a stable target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodDef {
    /// Method name as written.
    pub name: String,
    /// Parameters parsed from the method's parameter list.
    /// Empty for ``destructor`` (no parameter slot in the syntax).
    ///
    /// Empty *and* [`Self::params_computed`] set means "unknown", not
    /// "none" — see that field.
    pub params: Vec<ParamDef>,
    /// Original formal count owner or explicit authored metadata. Display
    /// parameter labels cannot recreate an original binding/count recipe.
    pub formal_count: crate::signature_scan::formal_count::SourceFormalCount,
    /// Unknown formals from a computed parameter word or reflective loop install.
    /// Literal loop names do not establish each iteration's signature.
    /// Arity consumers must use [`Self::arity`], as for
    /// [`crate::analyser::ProcDef::params_computed`].
    pub params_computed: bool,
    /// Source span of the name token.
    pub name_span: Span,
    /// Source span of the method body (braces excluded).
    pub body_span: Span,
    /// Method kind: the spelling of the [`crate::ir::MethodKind`] its row
    /// lands in (``"method"`` / ``"classmethod"`` / ``"constructor"`` /
    /// ``"destructor"``), or ``"forward"``.
    pub kind: String,
    /// `true` only for a `classmethod`-kind entry declared via `TclOO`'s
    /// `self` wrapper (`self method NAME …`) directly on this class.
    /// Unlike `ooutil`'s `classmethod` keyword —
    /// confirmed against tclsh 9.0.4/8.6 to propagate to a subclass's own
    /// bound command via its `Delegate`-mixin machinery, which walks
    /// `info class superclass` — a plain `self method` is visible ONLY on
    /// the exact class that declared it: `oo::class create Gadget {
    /// superclass Widget }` does NOT gain `Widget`'s `self method make`
    /// (real tclsh: `unknown method "make"`). The class-command MRO walk
    /// in `tcl-lsp-core`'s `method_dispatch_definition` accepts a
    /// `class_methods` entry from an ancestor provider only when this is
    /// `false`; a provider matching the receiver class itself is always
    /// accepted regardless.
    pub is_self_method: bool,
    /// Visibility: ``"public"`` / ``"private"`` /
    /// ``"unexported"``.
    pub visibility: String,
    /// Doc-comment text harvested from preceding lines.
    pub doc: String,
    /// For a ``"forward"`` method (``forward NAME TARGET ?ARG…?``), the
    /// forwarded ``(target command, prepended args)`` — `TclOO`'s
    /// version of `interp alias` partial application (confirmed against
    /// tclsh 9.0.4: a forwarded method call binds the prepended args
    /// first, then the caller's own arguments, against `TARGET`'s own
    /// arity). `None` for every other kind, and for a `forward` whose
    /// target couldn't be parsed.
    pub forward_target: Option<(String, Vec<String>)>,
}

impl MethodDef {
    /// The method's declared argument arity — or the **abstaining**
    /// `0..unlimited` when its parameter list is unmodelled
    /// ([`Self::params_computed`]).
    ///
    /// Exactly [`ProcDef::arity`]'s contract, extended to `MethodDef`: every
    /// arity consumer (`$obj method` call-site checking, `next`/`nextto`
    /// dispatch) asks here rather than calling
    /// [`crate::signature_scan::arity::arity_of`] on [`Self::params`]
    /// directly, so a member whose name is all this walk could honestly
    /// record never has its absent parameter list mistaken for a
    /// zero-argument one.
    #[must_use]
    pub fn arity(&self) -> tcl_registry::Arity {
        self.formal_count_projection().arity()
    }

    /// Stable descriptive header count, without source bytes or runtime grants.
    #[must_use]
    pub fn formal_count_projection(
        &self,
    ) -> crate::signature_scan::formal_count::SourceFormalCountProjection {
        self.formal_count
            .projection(&self.params, self.params_computed)
    }

    /// Whether this nameless member — a constructor or destructor, recorded
    /// under the synthetic `<keyword>` name its member keyword gives it — was
    /// declared by the keyword `word`.
    ///
    /// A provider meeting the keyword itself (the cursor on `constructor` in a
    /// class body) asks the member it declared, and reads what that member is
    /// off [`Self::kind`], rather than comparing the word with a spelling: a
    /// definer whose constructor member is spelt otherwise resolves the same
    /// way.
    #[must_use]
    pub fn is_declared_by_keyword(&self, word: &str) -> bool {
        self.name
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
            == Some(word)
    }
}

/// Stable composite key naming a class's instance method or class method:
/// `{class}::method::{name}` / `{class}::classmethod::{name}`.  The single
/// canonical form for re-identifying the *same* member across a round-trip —
/// the incremental item-tree diff ([`crate::analyser::item_tree`]) and any
/// LSP-facing consumer that hands a member identity to the client and gets
/// it back later (the code-lens `codeLens/resolve` handshake) — so the two
/// never drift onto different naming schemes for the same member.  A class
/// or proc's own qualified name (`::Bar`, `::helper`) never contains
/// `::method::`/`::classmethod::`, so this key cannot collide with theirs.
#[must_use]
pub fn class_member_key(class_qualified: &str, name: &str, is_classmethod: bool) -> String {
    if is_classmethod {
        format!("{class_qualified}::classmethod::{name}")
    } else {
        format!("{class_qualified}::method::{name}")
    }
}

/// Stable composite key naming a class's `property`: `{class}::property::{name}`.
/// Properties are a third, independent member table — never a `method` or
/// `classmethod` — so this never collides with [`class_member_key`]'s output;
/// same round-trip rationale (item-tree diff, code-lens `codeLens/resolve`).
#[must_use]
pub fn class_property_key(class_qualified: &str, name: &str) -> String {
    format!("{class_qualified}::property::{name}")
}

/// Stable composite key naming a class's own effective `constructor`:
/// `{class}::constructor`.  Unlike a method or property there is exactly one
/// dispatch-reachable constructor per class (`oo::configurable` allows
/// several to be *declared*, but only the last is ever effective — see
/// [`crate::analyser::class_hierarchy::ClassHierarchy::constructor_provider`]),
/// so this carries no `{name}` suffix; same round-trip rationale as
/// [`class_member_key`] / [`class_property_key`].
#[must_use]
pub fn class_constructor_key(class_qualified: &str) -> String {
    format!("{class_qualified}::constructor")
}

/// The `destructor` counterpart of [`class_constructor_key`]:
/// `{class}::destructor`.
#[must_use]
pub fn class_destructor_key(class_qualified: &str) -> String {
    format!("{class_qualified}::destructor")
}

/// One per-object method added by an `oo::objdefine`: the method declaration
/// plus the **objdefine site's**
/// receiver offset, the anchor a consumer resolves to a variable
/// binding so same-named receivers in different scopes never collide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMethodDef {
    /// The declared method (name, spans, kind, visibility).
    pub def: MethodDef,
    /// Byte offset of the `oo::objdefine` receiver word — inside the
    /// scope whose binding of the receiver variable this record
    /// belongs to.
    pub objdefine_offset: u32,
}

/// Effective **per-object member state** for one receiver binding — the
/// durable home for `oo::objdefine` effects, folded in source
/// order across every block on the same binding (the same binding identity
/// [`ObjectMethodDef::objdefine_offset`] anchors: the innermost proc / method
/// body declaring the receiver variable, or the top level).
///
/// [`AnalysisResult::object_methods`] stays the per-block *declaration*
/// record (navigation to each declared member, reassignment abstention);
/// this is the *folded* record — what the object's own table and visibility
/// look like after the last walked block — which is what dispatch-order
/// consumers (definition, completion) and the `oo::objdefine` W315 need.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectMemberState {
    /// Independently selected Logical source names and visibility masks.
    pub logical_source_members: Option<LogicalSourceObjectMemberState>,
    /// The object's own member table after every walked block: declared
    /// methods, minus retractions, with renames applied — each entry
    /// carrying its effective visibility.
    pub methods: HashMap<String, MethodDef>,
    /// Names this binding's blocks explicitly `export` — including names the
    /// object's *class* provides, which is exactly the flip that had nowhere
    /// to live.  Kept mutually exclusive with
    /// [`Self::unexports`], last writer wins, the same contract as the
    /// class-side pairs.
    pub exports: std::collections::HashSet<String>,
    /// Names explicitly `unexport`ed — see [`Self::exports`].
    pub unexports: std::collections::HashSet<String>,
    /// Byte offset of the binding's first `oo::objdefine` receiver word —
    /// the anchor a consumer resolves to a variable binding, exactly as
    /// [`ObjectMethodDef::objdefine_offset`] is resolved.
    pub anchor_offset: u32,
    /// True when any contributing block ran under a conditional: the
    /// cross-block table is then not order-provable, and the per-object
    /// W315 abstains rather than judge retractions against it.
    pub conditional: bool,
}

/// `TclOO` property definition.
///
/// Recorded by the OO body walker for ``property`` subcommands
/// inside an ``oo::class create`` / ``oo::define`` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDef {
    /// Property name as written.
    pub name: String,
    /// Source span of the property-name token.
    pub name_span: Span,
    /// Property kind: ``"readable"`` / ``"writable"`` /
    /// ``"readwrite"``.  Defaults to ``"readwrite"`` when ``-kind``
    /// is omitted.
    pub kind: String,
    /// True when ``-get BODY`` was supplied.
    pub has_getter: bool,
    /// True when ``-set BODY`` was supplied.
    pub has_setter: bool,
}

/// Which of a class's two method tables a definition-body member word acts on.
///
/// `TclOO` keeps the members a class gives its *instances* ([`ClassDef::methods`])
/// and the members defined on the class *object* itself
/// ([`ClassDef::class_methods`], what `self …` targets) in separate tables, and
/// every member word is scoped to exactly one of them: an unwrapped
/// `deletemethod` / `unexport` in a class body acts on the instance side, the
/// same word under `self` acts on the class-object side, and neither reaches
/// across.  Naming the side explicitly is what stops a class-side `unexport m`
/// from also un-exporting an identically-named instance method,
/// and what lets a cross-document retraction record which table it removes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberSide {
    /// [`ClassDef::methods`] — what instances of the class dispatch.
    Instance,
    /// [`ClassDef::class_methods`] — what the class object itself dispatches
    /// (`self method …`, `self { … }`).
    ClassObject,
}

impl MemberSide {
    /// The [`ClassDef`] method table this side names.
    #[must_use]
    pub fn table(self, class_def: &mut ClassDef) -> &mut HashMap<String, MethodDef> {
        match self {
            Self::Instance => &mut class_def.methods,
            Self::ClassObject => &mut class_def.class_methods,
        }
    }

    /// This side's `(exports, unexports)` visibility sets — the pair
    /// `export` / `unexport` writes when scoped to this side.
    ///
    /// The sided counterpart of [`Self::table`], and the reason
    /// `apply_visibility_member` has no `if side == Instance` branch: both
    /// sides take the identical last-writer-exclusive update, they just land in
    /// a different pair.  [`ClassDef::exports`] / [`ClassDef::unexports`] are
    /// the **instance**-side pair by contract and
    /// [`ClassDef::class_exports`] / [`ClassDef::class_unexports`] the
    /// class-object-side one.
    #[must_use]
    pub fn visibility_sets(
        self,
        class_def: &mut ClassDef,
    ) -> (&mut HashSet<String>, &mut HashSet<String>) {
        match self {
            Self::Instance => (&mut class_def.exports, &mut class_def.unexports),
            Self::ClassObject => (&mut class_def.class_exports, &mut class_def.class_unexports),
        }
    }

    /// This side's declared method-filter list — what `filter` (instance) and
    /// `self filter` (class object) each assign.
    ///
    /// The two are genuinely separate slots in `TclOO`, not one list read
    /// twice, and they intercept different dispatches (see
    /// [`ClassDef::filters`] / [`ClassDef::class_filters`] for the oracle).
    #[must_use]
    pub fn filter_list(self, class_def: &mut ClassDef) -> &mut Vec<String> {
        match self {
            Self::Instance => &mut class_def.filters,
            Self::ClassObject => &mut class_def.class_filters,
        }
    }
}

/// A reason one `TclOO` definition body **cannot run at all** — real Tcl
/// aborts the whole `oo::class create` / `oo::define` and creates no class.
///
/// Recorded by the member walker where the retracting word is applied (the one
/// site that knows the side's table state at that point in the body) and
/// drained by the class handlers, which turn each into a `W315`.  Every arm is
/// oracle-pinned byte-identical on tclsh 9.0.4 and 8.6.14, and each carries the
/// interpreter's own error text so the diagnostic reads as the failure the user
/// will actually hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionAbortKind {
    /// A retracting word (`deletemethod m`, `renamemethod m x`) named a member
    /// that does not exist **on the side the word is scoped to** — either never
    /// declared, or declared only on the other side.
    ///
    /// ```tcl
    /// oo::class create ::E1 { deletemethod ghost ; method ghost {} {} }
    /// ;# -> method ghost does not exist        (and ::E1 is never created)
    /// oo::class create ::E2 { self { method cm {} {} } ; deletemethod cm }
    /// ;# -> method cm does not exist
    /// ```
    MissingMember,
    /// `renamemethod a b` where `b` is already a member of the same side.
    ///
    /// ```tcl
    /// oo::class create ::E3 { method a {} {} ; method b {} {} ; renamemethod a b }
    /// ;# -> method called b already exists
    /// ```
    ///
    /// Side-scoped like everything else: `method a` + `self method b` +
    /// `renamemethod a b` is legal and leaves the instance side with `b`.
    DestinationExists,
    /// `renamemethod a a` — the source and destination are the same name.
    ///
    /// ```tcl
    /// oo::class create ::A4 { method a {} {} ; renamemethod a a }
    /// ;# -> cannot rename method to itself
    /// ```
    RenameToItself,
}

/// One member a record **retracts without declaring** — the cross-document
/// tombstone described on
/// [`ClassDef::retracted_members`](crate::analyser::ClassDef::retracted_members).
///
/// Carries a *destination*: a `renamemethod old new` in a
/// cross-file `oo::define` stub does not merely delete `old`, it moves the
/// member to `new`, and the stub has no [`MethodDef`] of its own to move — the
/// member's params / body / visibility live in the defining file's record.
/// Recording the arrival name here is what lets the workspace join re-key that
/// other record's member, so `new` appears instead of the member vanishing.
///
/// `arrival` is `None` for a plain `deletemethod`, and also for a
/// `renamemethod old $new` whose destination is computed — that names nothing
/// statically, so the retraction stands alone and the move abstains, the
/// direction this campaign abstains toward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRetractionRecord {
    /// The member name retracted.
    pub member: String,
    /// The side the retracting word was scoped to.
    pub side: MemberSide,
    /// The name the member arrives under, when the retracting word was a
    /// **move** rather than a deletion.
    pub arrival: Option<String>,
    /// Span of the arrival word — the synthetic declaration site the moved
    /// member gets, and therefore what go-to-definition on the new name
    /// answers with.  `None` whenever [`Self::arrival`] is.
    pub arrival_span: Option<Span>,
}

impl MemberRetractionRecord {
    /// A plain retraction with no destination.
    #[must_use]
    pub fn deletion(member: String, side: MemberSide) -> Self {
        Self {
            member,
            side,
            arrival: None,
            arrival_span: None,
        }
    }
}

/// One `renamemethod` that successfully **moved** a member, with the member
/// state the move ran against.
///
/// Recording the move is what lets a *rename* of the arrived member be checked:
/// its declaration site is the `renamemethod`'s destination word, so renaming
/// it rewrites that word and nothing else — turning `renamemethod old new` into
/// `renamemethod old X`. For some `X` that is a body real Tcl refuses to run,
/// and the request has to be refused rather than answered with edits that make
/// the class invalid.
///
/// [`Self::abort_if_renamed_to`] is the single fold both consumers use: the
/// member walker asks it about the destination actually written (that is how
/// `W315` is decided), and the rename gate asks it about the name the user
/// typed. Neither re-derives the rule, so the diagnostic and the gate cannot
/// drift apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamedMember {
    /// The name the member was declared under, retracted by this move.
    pub source: String,
    /// The name it arrived under — the key it now occupies in `side`'s table.
    pub destination: String,
    /// The side the move happened on. A collision is side-local: `method a` +
    /// `self method b` + `renamemethod a b` is legal and leaves the instance
    /// side with `b` (tclsh 9.0.4 / 8.6.14).
    pub side: MemberSide,
    /// Span of the destination word — the moved member's synthetic name span,
    /// and therefore the one token a rename of it rewrites.
    pub destination_span: Span,
    /// Names live on `side` **at the moment the move ran**: the source already
    /// removed, the destination not yet inserted.
    ///
    /// A snapshot rather than the final table because the interpreter reads the
    /// table at that point too, and both directions of that matter:
    /// `deletemethod sib ; renamemethod old sib` is legal (`sib` is gone by
    /// then) and `renamemethod old sib ; method sib {} {}` is legal as well
    /// (`sib` does not exist yet) — both pinned identical on 9.0.4 and 8.6.14.
    pub blocked: Vec<String>,
}

impl RenamedMember {
    /// Why naming this moved member `candidate` would abort the whole class
    /// definition, or `None` when that name is legal here.
    ///
    /// The two shapes, oracle-pinned byte-identical on tclsh 9.0.4 and 8.6.14
    /// (no class is created in either case):
    ///
    /// ```tcl
    /// oo::class create ::A1 { method old {} {…} ; renamemethod old old }
    /// ;# -> cannot rename method to itself
    /// oo::class create ::B1 { method old {} {…} ; method sib {} {…}
    ///                         renamemethod old sib }
    /// ;# -> method called sib already exists
    /// ```
    #[must_use]
    pub fn abort_if_renamed_to(&self, candidate: &str) -> Option<DefinitionAbortKind> {
        if candidate == self.source {
            Some(DefinitionAbortKind::RenameToItself)
        } else if self.blocked.iter().any(|n| n == candidate) {
            Some(DefinitionAbortKind::DestinationExists)
        } else {
            None
        }
    }

    /// Whether renaming the **ordinary member** `member` to `candidate` would
    /// collide at *this* move's site — the mirror direction.
    ///
    /// `method old {} {…} ; method sib {} {…} ; renamemethod old new` is legal,
    /// but renaming `sib` to `new` rewrites the sibling's declaration and
    /// produces `method new {} {…} ; renamemethod old new`, which aborts with
    /// `method called new already exists`. The same recorded snapshot answers
    /// it: `member` was live here, and `candidate` is the name this move needs
    /// free.
    #[must_use]
    pub fn collides_with_renaming(&self, member: &str, candidate: &str) -> bool {
        candidate == self.destination && self.blocked.iter().any(|n| n == member)
    }
}

/// One recorded [`DefinitionAbortKind`] with the member name it names and the
/// span of the offending word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionAbort {
    /// Why the definition cannot run.
    pub kind: DefinitionAbortKind,
    /// The member name the offending word named.
    pub member: String,
    /// Span of the offending argument word — the name that does not exist, or
    /// the destination that is already taken.
    pub span: Span,
}

impl DefinitionAbort {
    /// The diagnostic message for this abort, mirroring the interpreter's own
    /// error text after a fixed "cannot run" preamble.
    #[must_use]
    pub fn message(&self) -> String {
        format!("this class definition cannot run: {}", self.reason())
    }

    /// [`Self::message`] for a **per-object** definition (`oo::objdefine`),
    /// whose failing script is an object definition, not a class one — the
    /// interpreter's own reason text is identical on both paths.
    #[must_use]
    pub fn object_message(&self) -> String {
        format!("this object definition cannot run: {}", self.reason())
    }

    /// The interpreter-mirroring reason text shared by both message forms.
    pub(super) fn reason(&self) -> String {
        match self.kind {
            DefinitionAbortKind::MissingMember => {
                format!("method \"{}\" does not exist", self.member)
            }
            DefinitionAbortKind::DestinationExists => {
                format!("method called \"{}\" already exists", self.member)
            }
            DefinitionAbortKind::RenameToItself => "cannot rename method to itself".to_string(),
        }
    }
}
