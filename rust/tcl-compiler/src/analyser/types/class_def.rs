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

//! `ClassDef`: one class's declarations, the merge of two records of the same class and the provenance of its metaclass and name.

use super::class_factories::ClassFactory;
use super::class_members::{
    DefinitionAbort, MemberRetractionRecord, MemberSide, MethodDef, PropertyDef, RenamedMember,
};
use super::original_members::OriginalSourceMemberLedger;
use super::original_properties::OriginalSourcePropertyLedger;
use super::original_relations::OriginalSourceClassRelationLedger;
use super::original_special_members::OriginalSourceSpecialMemberLedger;
use std::collections::{HashMap, HashSet};
use tcl_lexer::Span;

/// Where a [`ClassDef::metaclass`] value came from.
///
/// An enum rather than a bare flag because the interesting half is what the
/// *default* means: `"oo::class"` sitting in a record no creation command
/// ever touched is a placeholder, and a join that reads it as evidence
/// invents a disagreement out of nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetaclassProvenance {
    /// Left at [`ClassDef::default()`]'s `"oo::class"` — no walk read it.
    ///
    /// The abstaining direction, and therefore the default: a record that
    /// cannot say where its metaclass came from constrains nothing in a
    /// join, while every consumer that simply reads `metaclass` keeps the
    /// stand-in it always had.
    #[default]
    StandIn,
    /// Read from a creation command's own head word — `oo::class create`
    /// and the other `TclOO` metaclasses, `snit::type` / `snit::widget`,
    /// `itcl::class`. The only spelling that is evidence.
    Observed,
}

/// Whether distinct original source slots have collided in one report key.
/// An unobserved collision is not proof of a unique or live native class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SourceNameAmbiguity {
    /// No collision was retained by the source publication join.
    #[default]
    Unobserved,
    /// A distinct original slot or policy was retained under the report key.
    Observed,
}

impl SourceNameAmbiguity {
    /// Reports only the retained collision observation.
    #[must_use]
    pub const fn is_observed(self) -> bool {
        matches!(self, Self::Observed)
    }

    /// Accumulate an observation without withdrawing an earlier collision.
    pub fn observe_if(&mut self, observed: bool) {
        if observed {
            *self = Self::Observed;
        }
    }
}

/// A method declaration written in a counted class's initial source body.
/// This is syntax inventory, not method installation, availability or dispatch.
/// Consumers must rejoin its enclosing class body to the current source and
/// context; a cloned declaration alone cannot establish that correspondence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalCountedBodyMethod {
    pub(in super::super) name: crate::signature_scan::scope::SignatureSourceNameKey,
    pub(in super::super) worker_word: tcl_lexer::NativeWord,
    pub(in super::super) site: crate::command_binding::CommandAllocationSite,
    pub(in super::super) side: MemberSide,
    pub(in super::super) parameters: Option<tcl_lexer::NativeWord>,
    pub(in super::super) body: Option<tcl_lexer::NativeWord>,
}

impl OriginalCountedBodyMethod {
    /// Exact original static naming units.
    #[must_use]
    pub const fn original_name_input(
        &self,
    ) -> &crate::signature_scan::scope::SignatureSourceNameKey {
        &self.name
    }
    /// Actual original naming operand, never an edit recipe for a derived value.
    #[must_use]
    pub const fn original_word(&self) -> &tcl_lexer::NativeWord {
        &self.worker_word
    }
    /// Whole original declaring command site.
    #[must_use]
    pub const fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        &self.site
    }
    /// Source declaration side selected by the definition grammar.
    #[must_use]
    pub const fn side(&self) -> MemberSide {
        self.side
    }
    /// Original parameter-list role, without parameter binding authority.
    #[must_use]
    pub const fn parameters_word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.parameters.as_ref()
    }
    /// Original script-body role, without entry or execution authority.
    #[must_use]
    pub const fn body_word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.body.as_ref()
    }
}

/// Counted original class metadata with no reporting-name projection.
/// This source-only receipt establishes neither runtime installation nor member coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalCountedClass {
    pub(in super::super) body: std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    pub(in super::super) initial_methods: Option<Vec<OriginalCountedBodyMethod>>,
}

impl OriginalCountedClass {
    /// Authenticated original definition body; no entry is implied.
    #[must_use]
    pub fn body(&self) -> &crate::registry_invocation::OriginalSourceScriptBody {
        &self.body
    }
    /// Declarations in the initial written body only. Neither absence nor current members follow.
    #[must_use]
    pub fn initial_method_declarations(&self) -> Option<&[OriginalCountedBodyMethod]> {
        self.initial_methods.as_deref()
    }
}

/// Class definition record.
///
/// The structural fields (`superclasses`, `mixins`, `methods`,
/// `class_methods`) feed the class-hierarchy / MRO algorithms; the
/// body walker populates them.  The remaining fields (``metaclass``,
/// ``properties``, ``variables``, ``filters``, ``exports``,
/// ``unexports``, ``constructors``, ``destructor``, ``doc``) carry
/// the full record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDef {
    /// Authoritative original counted member declarations and ordered effects.
    /// Reporting-name maps do not supply this ledger's identity or coverage.
    pub original_members: OriginalSourceMemberLedger,
    /// Canonical nameless lifecycle declarations, without dispatch or absence.
    pub original_special_members: OriginalSourceSpecialMemberLedger,
    /// Original property source names, kinds and accessor-body producers.
    /// This ledger supplies declaration advice, not native property admission.
    pub original_properties: OriginalSourcePropertyLedger,
    /// Original relation operands and own receiver effects, without report keys.
    pub original_relations: OriginalSourceClassRelationLedger,
    /// Retained original object publication geometry; no runtime class token.
    pub source_name: Option<crate::signature_scan::scope::SignatureSourceCommand>,
    /// Distinct component slots share this record's rendered map key.
    pub source_name_ambiguous: SourceNameAmbiguity,
    /// Original metaclass-head lookup; supplies no reached provider identity.
    pub metaclass_lookup: Option<crate::signature_scan::scope::SignatureSourceLookup>,
    /// Original caller contexts of relation words; ambiguity remains unavailable.
    pub relation_lookups:
        HashMap<String, Option<crate::signature_scan::scope::SignatureSourceLookup>>,
    /// Class name as written.
    pub name: String,
    /// Fully-qualified class name with leading ``::``.
    pub qualified_name: String,
    /// Source span of the name token.
    pub name_span: Span,
    /// Class body span retained by its declaration producer.
    pub body_span: Span,
    /// Metaclass — one of ``"oo::class"`` / ``"oo::configurable"``
    /// / ``"oo::abstract"`` / ``"oo::singleton"``.  Defaults to
    /// ``"oo::class"``.
    ///
    /// The default is a *stand-in*, not a reading of the source: a record
    /// built by anything other than a creation command — an
    /// ``oo::define`` stub, the ``oo::objdefine`` holder — carries it
    /// having observed nothing. [`Self::metaclass_provenance`] is what
    /// tells the two apart.
    pub metaclass: String,
    /// Where [`Self::metaclass`] came from — the observed-fields mask this
    /// record needs, and the only entry it needs.
    ///
    /// Every other field defaults to a value that *is* its own "nothing
    /// observed" reading — empty tables, empty superclass list, `None`,
    /// `false`, a zero span — so a join can treat the default as a lower
    /// bound and take the other side's. `metaclass` alone defaults to a
    /// fabricated concrete name, which a join comparing it as evidence
    /// reads as a walk having *proved* `oo::class`; a `via_define` stub for
    /// a computed-name class made by `::T::Mother` then looked like two
    /// walks disagreeing about the metaclass, and the record abstained down
    /// to no factory and no superclasses.
    pub metaclass_provenance: MetaclassProvenance,
    /// Direct superclasses in declaration order.  Each entry
    /// is a fully-qualified class name with leading ``::``.
    pub superclasses: Vec<String>,
    /// Class-level mixins in declaration order.  Same naming
    /// convention as `superclasses`.
    pub mixins: Vec<String>,
    /// Instance methods keyed by simple name.
    pub methods: HashMap<String, MethodDef>,
    /// Class methods keyed by simple name.
    pub class_methods: HashMap<String, MethodDef>,
    /// Constructor methods (multiple constructors allowed under
    /// ``oo::configurable``).  Stored in declaration order.
    pub constructors: Vec<MethodDef>,
    /// Destructor method, when one was defined.
    pub destructor: Option<MethodDef>,
    /// Class-level instance variables declared via ``variable``.
    pub variables: Vec<String>,
    /// Configurable properties keyed by name.
    pub properties: HashMap<String, PropertyDef>,
    /// **Instance-side** method filters declared via an unwrapped (or
    /// `private`-wrapped) ``filter`` — the class's `info class filters` slot.
    /// They intercept dispatches on *instances* of the class.
    ///
    /// Sided because `TclOO` keeps two independent filter slots, exactly as it
    /// keeps two method tables. Oracle, byte-identical on tclsh
    /// 9.0.4 and 8.6.14:
    ///
    /// ```tcl
    /// oo::class create ::A { method real {} {…} ; method logit {args} {…} ; filter logit }
    /// info class filters ::A    ;# -> logit
    /// info object filters ::A   ;# -> (empty)      the class object is unfiltered
    /// [::A new] real            ;# -> logit fires, `self target` is `::A real`
    /// oo::class create ::P { private { method s {} {} ; filter s } }
    /// info class filters ::P    ;# -> s            `private` is instance-side too
    /// ```
    pub filters: Vec<String>,
    /// **Class-object-side** method filters declared via ``self filter`` — the
    /// class object's own `info object filters` slot.
    ///
    /// A class-side filter intercepts dispatches on the *class command itself*,
    /// including the constructor path, and never touches instance dispatch.
    /// Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
    ///
    /// ```tcl
    /// oo::class create ::B { method inst {} {…}
    ///                        self { method cls {} {…} ; method logit {args} {…} ; filter logit } }
    /// info object filters ::B   ;# -> logit
    /// info class filters ::B    ;# -> (empty)      instances are unfiltered
    /// ::B cls                   ;# -> logit fires, `self target` is `::B cls`
    /// ::B new                   ;# -> logit fires, `self target` is `::oo::class new`
    /// [::B new] inst            ;# -> logit does NOT fire
    /// ```
    pub class_filters: Vec<String>,
    /// Methods explicitly exported via an **instance-side** ``export``.
    pub exports: HashSet<String>,
    /// Methods explicitly unexported via an **instance-side** ``unexport``.
    ///
    /// Kept **mutually exclusive** with [`Self::exports`] — each set records the
    /// last explicit writer for a name, and the cross-file consumer
    /// (`workspace_index::method_dispatch_chain`) reads any `exports` entry as
    /// decisive, so a name left in both would advertise a method the interpreter
    /// rejects.
    pub unexports: HashSet<String>,
    /// Methods explicitly exported via ``self export`` — the **class-object**
    /// side's counterpart of [`Self::exports`], maintained under the identical
    /// last-writer-exclusive rule ([`MemberSide::visibility_sets`]).
    ///
    /// A separate pair rather than a side tag on the existing one because
    /// `exports`/`unexports` are the instance-side record *by contract*: the
    /// workspace effective-export union and `rename_safety` both read them that
    /// way, so a class-side flip landing there would silently re-state an
    /// unrelated instance method's export bit. Without this pair a
    /// `self unexport m` in one file never reached another file's class-command
    /// dispatch, which went on advertising a member `::C m` rejects with
    /// `unknown method "m"`.
    ///
    /// Rides the same cross-file channel as [`Self::retracted_members`] — a
    /// `via_define` stub carries it — and shares that channel's documented
    /// unordered caveat: cross-file load order is not knowable from the index,
    /// so the workspace takes the union rather than a last writer.
    pub class_exports: HashSet<String>,
    /// Methods explicitly unexported via ``self unexport`` — see
    /// [`Self::class_exports`].
    pub class_unexports: HashSet<String>,
    /// Members this record **retracts** (`deletemethod` / `renamemethod`)
    /// without itself declaring them — a *tombstone* for a cross-document
    /// consumer, keyed by member name and the side the retracting word was
    /// scoped to.
    ///
    /// A retraction of a member the same document declares needs no tombstone:
    /// it already removed the entry from that side's table, and the order within
    /// one body is known. What cannot be modelled locally is
    /// `oo::class create ::C { method m … }` in one file and
    /// `oo::define ::C { deletemethod m }` in another: the second file's
    /// [`Self::via_define`] stub finds nothing to remove, so without this the
    /// workspace goes on advertising and resolving `m` even though sourcing the
    /// extension deletes it. Cross-file *order* is
    /// unprovable, so the tombstone is unordered — exactly as a cross-file
    /// `oo::define ::C { method extra … }` is an unordered addition today.
    pub retracted_members: Vec<MemberRetractionRecord>,
    /// Reasons this record's definition body **cannot run at all** — a
    /// retraction of a member absent from its side's table, or a
    /// `renamemethod` onto a name already taken.
    ///
    /// **Transient.** Recorded by the member walker, which is the only place
    /// that knows a side's table state at the point the offending word runs,
    /// and *drained* by the class handlers the moment the body walk returns —
    /// so the record that reaches `all_classes` (and the workspace index)
    /// always has this empty. It is not part of the class's published shape;
    /// it is the walker's channel to the diagnostic emitter.
    ///
    /// The partial class is still recorded, deliberately: a body that cannot
    /// run has no outline at all in real Tcl, but degrading navigation to
    /// nothing is worse than describing what the author clearly meant —
    /// the same judgement parse errors already get.
    pub definition_aborts: Vec<DefinitionAbort>,
    /// Every `renamemethod` in this record's body that **moved** a member, with
    /// the member state the move ran against.
    ///
    /// The moved member's declaration site is the `renamemethod`'s destination
    /// word, so a rename of it rewrites that word — and some new names turn the
    /// body into one real Tcl refuses to run. Unlike
    /// [`Self::definition_aborts`] this is **not** transient: the rename gate
    /// re-analyses the document and reads it from the recorded class, so it has
    /// to survive into `all_classes`.
    pub renamed_members: Vec<RenamedMember>,
    /// Readonly alias/target display inventory. Navigation and edits use
    /// original link invocations and member declarations instead.
    pub linked_members: HashMap<String, String>,
    /// Doc-comment text harvested from the line(s) above the
    /// ``oo::class create`` / ``oo::define`` statement.
    pub doc: String,
    /// `true` when this record originates from an ``oo::define`` on a class
    /// **not** created in this file — a cross-file extension "stub" (it adds
    /// members / a `superclass` to a class defined elsewhere) rather than the
    /// class's own ``oo::class create`` definition.  Go-to-definition prefers a
    /// real creation site over such a stub; `false` for `oo::class create` and
    /// for an ``oo::define`` that extends a class created earlier in the same
    /// file.
    pub via_define: bool,
    /// `true` when the class was manufactured by a **user-defined metaclass**
    /// whose `create` override the analyser could not read, so the superclass
    /// list it splices into the class is unknown.  The class itself is real
    /// and its own body is fully modelled; only its inheritance is opaque.
    /// Method-existence checks (W308) must abstain on such a class exactly as
    /// they do for one whose superclass lives outside the workspace index —
    /// a method it inherits is not one it is missing.
    pub inheritance_unknown: bool,
    /// `true` when calling **this class's own command** with a bare,
    /// unrecognised first word constructs an instance and yields its name —
    /// Tk's `::tk::IconList .il`.
    ///
    /// Proved where the class is written, from its metaclass's
    /// unrecognised-word fallback member
    /// ([`ClassFactory::unknown_binds_instance`]), so a consumer in another
    /// document reads a settled fact instead of needing the metaclass's own
    /// body. `false` is "not proved", never "proved not to" — the abstaining
    /// direction, which leaves a handle untyped rather than mistyped.
    ///
    /// Distinct from the *family-wide*
    /// [`DefinitionBodyGrammar::bare_word_construction`](tcl_registry::definer::DefinitionBodyGrammar::bare_word_construction)
    /// flag snit carries: that one is true of every snit type by definition,
    /// this one is true only of the classes a particular metaclass makes.
    pub class_command_fallback: ClassCommandFallback,
    /// `true` when the class's definition body installs members the analyser
    /// could not read, so the recorded member tables are a **lower bound**
    /// on what the class really has.
    ///
    /// Two shapes set it, both registry-driven rather than keyword-matched:
    ///
    /// * a recognised member word whose declaration is supplied through an
    ///   unresolvable `{*}` expansion or a computed name —
    ///   `constructor {*}[info class constructor ::Base]`,
    ///   `method $m {*}[info class definition ::Base $m]` (the ticklecharts
    ///   `chart3D` reflection idiom). Tcl expands these at definition time,
    ///   so the members are entirely real; only their spelling is opaque.
    /// * a body command that is **not** a member word and either has no
    ///   registry spec at all (a helper proc that installs members) or takes
    ///   a script argument the member walk does not descend into
    ///   (`foreach m {…} { method $m … }`, `if {…} { method … }`).
    ///
    /// The class itself, and every member that *was* readable, stay fully
    /// modelled — this only says "there may be more". Method-existence
    /// checks (W308) must therefore abstain on such a class, exactly as they
    /// do for [`Self::inheritance_unknown`]: a method installed by
    /// reflection is not a method that is missing. tclsh 9.0.4 and 8.6.16
    /// agree that `oo::class create C3 { constructor {*}[info class
    /// constructor ::C] ; foreach m {options} { method $m {*}[info class
    /// definition ::C $m] } }` yields `info class methods ::C3` → `options`,
    /// and `[C3 new] options` really runs.
    pub member_set_incomplete: bool,
    /// Present when this class is itself a **class factory** — a user-defined
    /// `TclOO` metaclass whose superclass chain reaches `oo::class`.
    ///
    /// It describes how the factory manufactures classes (which creation
    /// argument is the name, which is the body, what the manufacturer splices
    /// into every body), derived once here rather than re-derived at each
    /// `Meta create …` call.  Publishing it on the `ClassDef` is what lets a
    /// *different file*'s walk classify `::tk::Megawidget create IconList …`
    /// at all: without it, that call is indistinguishable from
    /// `interp create` or `image create`.
    pub factory: Option<ClassFactory>,
}

/// Concatenate an **appended** ordered slot (`variable`, `filter`) from a
/// record whose words ran `from_ran_second` relative to `into`'s.
///
/// These slots accumulate rather than replace: on 8.6.16 and 9.0.4,
/// `oo::class create C { filter f; variable x }` then
/// `oo::define C { filter g; variable y }` reports filters `f g` and
/// variables `x y`. A name the slot already carries is not repeated —
/// declaring `variable x` twice leaves one `x` — and the surviving order
/// is run order, which is why the earlier record's entries lead.
///
/// See [`ClassDef::absorb_declarations`] for the slots that replace instead.
fn append_ordered_slot(into: &mut Vec<String>, from: &[String], from_ran_second: bool) {
    let (mut merged, tail) = if from_ran_second {
        (std::mem::take(into), from.to_vec())
    } else {
        (from.to_vec(), std::mem::take(into))
    };
    for name in tail {
        if !merged.contains(&name) {
            merged.push(name);
        }
    }
    *into = merged;
}

/// Add `key`'s declaration to `table`, replacing an existing entry only
/// when the contributing record is the one that ran second — see
/// [`ClassDef::absorb_declarations`].
fn insert_declaration<V: Clone>(
    table: &mut HashMap<String, V>,
    key: &str,
    value: &V,
    replace: bool,
) {
    match table.entry(key.to_owned()) {
        std::collections::hash_map::Entry::Occupied(mut slot) => {
            if replace {
                slot.insert(value.clone());
            }
        }
        std::collections::hash_map::Entry::Vacant(slot) => {
            slot.insert(value.clone());
        }
    }
}

impl ClassDef {
    /// Take every declaration `other` makes that this record does not
    /// already carry.
    ///
    /// Two records can describe one class: the computed-name creation
    /// observed inside a procedure body, and whatever the ordinary walk
    /// already built under the proved name — in particular an
    /// [`oo::define` stub](Self::via_define), whose members are a real
    /// additional contribution to the same class rather than a second
    /// reading of the same body. A join that simply picked the more
    /// complete record therefore silently dropped the other's members,
    /// which is how a stub's `method m` disappeared the moment the join
    /// stopped abstaining.
    ///
    /// **This join knows the order.** An `oo::define` extending a class
    /// necessarily runs after the creation it extends, so
    /// [`via_define`](Self::via_define) tells the two records apart *and*
    /// tells us which ran second. Every rule below is that ordering applied
    /// to one kind of field, and each kind takes it differently — so the
    /// rules are per-slot facts read off a real interpreter, not one
    /// pattern assumed to generalise. Measured identically on tclsh 8.6.16
    /// and 9.0.4, `oo::class create C {…}` followed by `oo::define C {…}`:
    ///
    /// | field | second definition supplies | result |
    /// |---|---|---|
    /// | `method m {a}` → `method m {a b}` | a new body | `{a b} …` — replace |
    /// | `export m` → `unexport m` | the opposite bit | unexported; `m` leaves the public table |
    /// | `mixin ::A` → `mixin ::B` | a new list | `::B` — replace (`mixin -append` is the additive form) |
    /// | `superclass ::S0` → `superclass ::S9` | a new list | `::S9` — replace |
    /// | `filter f` → `filter g` | another filter | `f g` — **append** |
    /// | `variable x` → `variable y` | another name | `x y` — **append**, de-duplicated |
    /// | `method m` → `deletemethod m` | a retraction | `m` is gone; dispatch raises `unknown method "m"` |
    ///
    /// So:
    ///
    /// * **Keyed tables** (`methods`, `class_methods`, `properties`,
    ///   `linked_members`) union, and a key both records declare goes to
    ///   whichever ran second. Where neither side is a stub the choice
    ///   cannot matter: a second creation under a name already taken never
    ///   runs (`can't create object "::D::class": command already exists
    ///   with that name`, same tclsh), so two non-stub observations are two
    ///   readings of one body and a colliding key holds the same
    ///   declaration either way. This record then keeps its own, being the
    ///   observation the caller judged more complete.
    /// * **Visibility** (`exports`/`unexports` and the class-side pair) is
    ///   one bit per member, not two independent sets, so a union can leave
    ///   a member in both — and a workspace consumer reads an `exports`
    ///   entry as decisive, which would advertise a method the later
    ///   definition took away. The later record's bit wins and clears its
    ///   opposite, the same last-writer-exclusive update
    ///   `oo::apply_visibility_member` performs while walking one body.
    /// * **Replaced ordered slots** (`mixins`) take the later record's list
    ///   whole when it has one, and the earlier record's only when the
    ///   later never set the slot.
    /// * **Appended ordered slots** (`variables`, `filters`,
    ///   `class_filters`) concatenate in run order, dropping a name the
    ///   list already carries — `variable x` twice leaves one `x`.
    /// * **Whole-body singletons** (`constructors`, `destructor`) follow the
    ///   replace rule, taking the earlier record's only when the later
    ///   declares none.
    /// * **Retractions** are *applied*, not merely carried. A `deletemethod`
    ///   in the later record removes the member from the joined table and a
    ///   `renamemethod` moves it, because this join is the point where the
    ///   ordering is known. The tombstones ride along as well, for the
    ///   cross-file consumer that still needs them, but compiler-side
    ///   consumers of `all_classes` never apply the workspace retraction
    ///   fold and would otherwise treat a deleted member as live.
    ///
    /// Two observations of one identical body — the case this join was
    /// written for, where neither record is a stub — remain a no-op under
    /// every rule above.
    ///
    /// Identity, spans, provenance flags and `definition_aborts` are not
    /// declarations and are left to the caller;
    /// [`definition_aborts`](Self::definition_aborts) in particular is
    /// transient and already consumed by W315 before either record reaches
    /// a join.
    pub(crate) fn absorb_declarations(&mut self, other: &Self) {
        // Destructured exhaustively on purpose: a field added to
        // `ClassDef` fails to compile here until it has been classified,
        // rather than being quietly dropped by every join.
        let Self {
            original_members: _,
            original_special_members: _,
            original_properties: _,
            original_relations: _,
            source_name: _,
            source_name_ambiguous: _,
            metaclass_lookup: _,
            relation_lookups: _,
            name: _,
            qualified_name: _,
            name_span: _,
            body_span: _,
            metaclass: _,
            metaclass_provenance: _,
            superclasses: _,
            mixins,
            methods,
            class_methods,
            constructors,
            destructor,
            variables,
            properties,
            filters,
            class_filters,
            exports,
            unexports,
            class_exports,
            class_unexports,
            retracted_members,
            definition_aborts: _,
            renamed_members,
            linked_members,
            doc,
            via_define,
            inheritance_unknown: _,
            class_command_fallback: _,
            member_set_incomplete: _,
            factory: _,
        } = other;

        // `other` is the `oo::define` that extends what this record created,
        // so it ran second and its declarations replace rather than add.
        // When *this* record is the stub the ordering is simply reversed:
        // `other` ran first and only fills what this one never mentions.
        let other_ran_second = *via_define && !self.via_define;
        self.absorb_original_declarations(other, other_ran_second);
        for (key, def) in methods {
            insert_declaration(&mut self.methods, key, def, other_ran_second);
        }
        for (key, def) in class_methods {
            insert_declaration(&mut self.class_methods, key, def, other_ran_second);
        }
        for (key, def) in properties {
            insert_declaration(&mut self.properties, key, def, other_ran_second);
        }
        for (key, target) in linked_members {
            insert_declaration(&mut self.linked_members, key, target, other_ran_second);
        }

        for side in [MemberSide::Instance, MemberSide::ClassObject] {
            let (other_exports, other_unexports) = match side {
                MemberSide::Instance => (exports, unexports),
                MemberSide::ClassObject => (class_exports, class_unexports),
            };
            self.absorb_visibility(side, other_exports, other_unexports, other_ran_second);
        }

        // `mixin` (and the whole-body singletons) replace the slot;
        // `variable` / `filter` append to it. Both confirmed on 8.6.16 and
        // 9.0.4 — see this function's table.
        if other_ran_second && !mixins.is_empty() || self.mixins.is_empty() {
            self.mixins.clone_from(mixins);
        }
        for (into, from) in [
            (&mut self.variables, variables),
            (&mut self.filters, filters),
            (&mut self.class_filters, class_filters),
        ] {
            append_ordered_slot(into, from, other_ran_second);
        }
        if other_ran_second && !constructors.is_empty() || self.constructors.is_empty() {
            self.constructors.clone_from(constructors);
        }
        if other_ran_second && destructor.is_some() || self.destructor.is_none() {
            self.destructor.clone_from(destructor);
        }

        for record in retracted_members {
            if !self.retracted_members.contains(record) {
                self.retracted_members.push(record.clone());
            }
        }
        for record in renamed_members {
            if !self.renamed_members.contains(record) {
                self.renamed_members.push(record.clone());
            }
        }
        if other_ran_second {
            self.apply_retractions(retracted_members);
        }
        if self.doc.is_empty() {
            self.doc.clone_from(doc);
        }
    }

    fn absorb_original_declarations(&mut self, other: &Self, other_ran_second: bool) {
        self.original_members
            .absorb(&other.original_members, other_ran_second);
        self.original_special_members
            .absorb(&other.original_special_members, other_ran_second);
        self.original_relations
            .absorb(&other.original_relations, other_ran_second);
        self.original_properties
            .absorb(&other.original_properties, other_ran_second);
        if self.source_name.is_none() {
            self.source_name.clone_from(&other.source_name);
        }
        self.source_name_ambiguous
            .observe_if(other.source_name_ambiguous.is_observed());
        if self.metaclass_lookup.is_none() {
            self.metaclass_lookup.clone_from(&other.metaclass_lookup);
        }
        for (written, lookup) in &other.relation_lookups {
            match self.relation_lookups.entry(written.clone()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(lookup.clone());
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry.get() != lookup {
                        entry.insert(None);
                    }
                }
            }
        }
    }

    /// Merge one side's `export` / `unexport` pair from a record whose
    /// words ran `other_ran_second` relative to this one's.
    ///
    /// `TclOO` keeps one visibility *bit* per member, not two independent
    /// sets: `export m` sets it and `unexport m` clears it, so the later
    /// word wins outright and the earlier one leaves no trace. Confirmed on
    /// 8.6.16 and 9.0.4 — after `oo::class create E1 { method m {} {};
    /// export m }` then `oo::define E1 { unexport m }`,
    /// `info class methods E1` is empty while `-private` still lists `m`,
    /// and `[E1 new] m` raises `unknown method "m"`; the reverse order
    /// re-exports it.
    ///
    /// The same last-writer-exclusive update
    /// [`oo::apply_visibility_member`](crate::analyser::oo) performs while
    /// walking a single body, applied across the two records this join
    /// holds. A plain union would leave a member in both sets, and the
    /// workspace reads an `exports` entry as decisive — so it would go on
    /// advertising a method the later definition took away.
    fn absorb_visibility(
        &mut self,
        side: MemberSide,
        other_exports: &HashSet<String>,
        other_unexports: &HashSet<String>,
        other_ran_second: bool,
    ) {
        for (from, exported) in [(other_exports, true), (other_unexports, false)] {
            for name in from {
                let (exports, unexports) = side.visibility_sets(self);
                let (mine, opposite) = if exported {
                    (exports, unexports)
                } else {
                    (unexports, exports)
                };
                // Skip only a name this record already decided *and*
                // decided later; otherwise `other`'s bit is the live one.
                if !other_ran_second && (mine.contains(name) || opposite.contains(name)) {
                    continue;
                }
                mine.insert(name.clone());
                opposite.remove(name);
            }
        }
    }

    /// Apply retractions recorded by a definition that ran **after** this
    /// record's members were declared.
    ///
    /// A `via_define` stub for a class created elsewhere finds nothing to
    /// remove while it is walked, so `deletemethod m` / `renamemethod m n`
    /// can only be written down as a tombstone. Across files that is all
    /// anyone can say — load order is unknowable, which is why
    /// [`Self::retracted_members`] is documented as unordered and the
    /// workspace applies it as a fold. Here the order *is* known, and the
    /// tombstone alone is not enough: compiler-side consumers of
    /// `AnalysisResult::all_classes` never run that fold, so a deleted
    /// member would stay dispatchable, keep answering method-existence
    /// checks, and feed derived factory facts.
    ///
    /// tclsh 8.6.16 and 9.0.4 agree on both shapes: after
    /// `oo::class create D1 { method m …; method keep … }` then
    /// `oo::define D1 { deletemethod m }`, `info class methods D1` is
    /// `keep` and `[D1 new] m` raises `unknown method "m"`; after
    /// `renamemethod m n` the table holds `n` alone.
    fn apply_retractions(&mut self, retractions: &[MemberRetractionRecord]) {
        for record in retractions {
            let table = record.side.table(self);
            let Some(mut moved) = table.remove(&record.member) else {
                continue;
            };
            // A move re-homes the member under the arrival name, whose
            // declaration site is the `renamemethod` destination word — the
            // token go-to-definition must answer with, and the only one a
            // rename of the arrived member may rewrite.
            let (Some(arrival), Some(span)) = (&record.arrival, record.arrival_span) else {
                continue;
            };
            arrival.clone_into(&mut moved.name);
            moved.name_span = span;
            table.insert(arrival.clone(), moved);
        }
    }
}

impl Default for ClassDef {
    /// Default-construct a [`ClassDef`].
    ///
    /// All names default to empty strings, both spans default to
    /// ``Span::new(0, 0)``, and the metaclass defaults to
    /// ``"oo::class"`` — with
    /// [`metaclass_provenance`](ClassDef::metaclass_provenance)
    /// [`StandIn`](MetaclassProvenance::StandIn), so
    /// that stand-in is never mistaken for a reading of the source.
    /// Used by handlers that build a class record incrementally
    /// (most common shape — only `name` / `qualified_name` /
    /// `name_span` / `body_span` need explicit values).
    fn default() -> Self {
        let zero = Span::new(0, 0);
        Self {
            original_members: OriginalSourceMemberLedger::default(),
            original_special_members: OriginalSourceSpecialMemberLedger::default(),
            original_properties: OriginalSourcePropertyLedger::default(),
            original_relations: OriginalSourceClassRelationLedger::default(),
            source_name: None,
            source_name_ambiguous: SourceNameAmbiguity::Unobserved,
            metaclass_lookup: None,
            relation_lookups: HashMap::new(),
            name: String::new(),
            qualified_name: String::new(),
            name_span: zero,
            body_span: zero,
            metaclass: "oo::class".to_string(),
            metaclass_provenance: MetaclassProvenance::StandIn,
            superclasses: Vec::new(),
            mixins: Vec::new(),
            methods: HashMap::new(),
            class_methods: HashMap::new(),
            constructors: Vec::new(),
            destructor: None,
            variables: Vec::new(),
            properties: HashMap::new(),
            filters: Vec::new(),
            class_filters: Vec::new(),
            exports: HashSet::new(),
            unexports: HashSet::new(),
            class_exports: HashSet::new(),
            class_unexports: HashSet::new(),
            retracted_members: Vec::new(),
            definition_aborts: Vec::new(),
            renamed_members: Vec::new(),
            linked_members: HashMap::new(),
            doc: String::new(),
            via_define: false,
            inheritance_unknown: false,
            class_command_fallback: ClassCommandFallback::None,
            member_set_incomplete: false,
            factory: None,
        }
    }
}

/// Statically proved behaviour of an unrecognised word dispatched through a
/// class command. An enum keeps the proof state explicit and leaves room for
/// future fallback effects without adding another boolean to [`ClassDef`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ClassCommandFallback {
    /// No construction-and-return proof is available.
    #[default]
    None,
    /// The fallback constructs an instance named by the word and returns it.
    ConstructsNamedInstance,
}

impl ClassCommandFallback {
    /// Whether the fallback provides the named-instance contract.
    #[must_use]
    pub fn constructs_named_instance(self) -> bool {
        self == Self::ConstructsNamedInstance
    }
}
