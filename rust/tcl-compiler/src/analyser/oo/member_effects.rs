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

//! What one member row does to its `ClassDef`: declared methods and forwards, properties, relations and slots, visibility and retraction.

use super::super::diagnostics::helpers::has_substitution;
use super::super::types::{
    ClassDef, DefinitionAbort, DefinitionAbortKind, MemberSide, MethodDef, PropertyDef,
    RenamedMember,
};
use super::super::utils::parse_param_list;
use super::{
    CollectedMethodBody, MemberDialect, MemberLanding, MemberStatement, member_side, record_method,
};
use crate::ir::MethodKind;
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{
    DefinitionBodyGrammar, MemberEffect, MemberSpec, MemberVisibility, RelationSlot,
};
#[cfg(test)]
use tcl_syntax::word_rules::WordValueRules;

/// Apply a *retracting* member word (`deletemethod m`, `self renamemethod old
/// new`, `private deletemethod m`) to `side`'s method table: remove every name
/// it retracts, and — for a renaming word — re-record the removed member under
/// the name it **arrives** at.
///
/// Which member words retract, and *which of their arguments* they retract or
/// re-key, is registry data ([`MemberSpec::retraction`] /
/// [`tcl_registry::definer::MemberRetraction::split`]), never a keyword matched
/// here — the sibling
/// [`MemberRefKind::Method`] members (`export` / `unexport` / `filter`) name a
/// method without removing it and must not retract.
///
/// `arg_tokens` is the *same slice* as `names`, word for word: the arrival
/// name's own token is the synthetic declaration site the moved member gets
/// (see below), so it has to be reachable from the index the registry hands
/// back.
///
/// # The rename really moves the member
///
/// `renamemethod old new` is not a deletion — it is a move, and the destination
/// is a fully dispatchable member carrying the *source's* body, parameters and
/// visibility. Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
///
/// ```tcl
/// oo::class create ::R1 { method old {} { return OLDBODY } ; renamemethod old new }
/// info class methods ::R1              ;# -> new
/// [::R1 new] new                       ;# -> OLDBODY            (the old body runs)
/// info class definition ::R1 new       ;# -> {} { return OLDBODY }
/// oo::class create ::R4 { method Priv {} {…} ; renamemethod Priv pub }
/// info class methods ::R4              ;# -> (empty)   `pub` is still UNexported…
/// info class methods ::R4 -private     ;# -> pub       …because `Priv` was
/// oo::class create ::R5 { method low {} {…} ; renamemethod low Up }
/// info class methods ::R5              ;# -> Up        …and `Up` is still exported
/// ```
///
/// so the moved [`MethodDef`] keeps its `visibility` verbatim — the family's
/// name-based default (`[a-z]*` is exported for `TclOO`) is applied when a
/// member is *(re)declared*, never when one is renamed. Its `body_span` also
/// stays pointing at the original body, which is the only real body there is;
/// only `name` and `name_span` move, the latter onto the `renamemethod` call's
/// own destination word — the natural go-to-definition target for a member
/// whose sole textual mention of the new name is right there.
///
/// # Definition-aborting shapes
///
/// Real Tcl aborts the *whole* definition — no class is created at all — when a
/// retracting word names a member that does not exist on its own side, or
/// renames onto a name that side already holds. Each is recorded as a
/// [`DefinitionAbort`] for the class handlers to turn into a `W315`; the
/// partial class is still built, so navigation degrades rather than vanishing.
/// This is the one site that can detect them, because it is the only place that
/// knows the side's table state *at the point the word runs* — which is exactly
/// the granularity real Tcl uses (`method a ; method b ; deletemethod b ;
/// renamemethod a b` is legal, and leaves `b`).
///
/// # Tombstones for what this record cannot see
///
/// A name this record does **not** declare leaves a *tombstone* in
/// [`ClassDef::retracted_members`] instead: that is the cross-document shape —
/// `oo::class create ::C { method m … }` in one file, `oo::define ::C {
/// deletemethod m }` in another — where the second file's `via_define` stub has
/// nothing local to remove, and dropping the retraction on the floor leaves the
/// workspace advertising a method that sourcing the extension deletes.
/// A retraction of a member the same document declares needs no
/// tombstone: it removed the entry outright, the order within one body is known,
/// and exporting it would wrongly cancel a *different* document's redeclaration
/// of the name, which no ordering evidence supports.
///
/// Tombstone and abort are the same fact read under different knowledge, so
/// they are mutually exclusive by construction: whether this record is a
/// cross-file stub is not knowable here, so *both* are recorded and the class
/// handler — which does know ([`ClassDef::via_define`]) — keeps exactly one.
fn retract_named_members(
    member: &MemberSpec,
    names: &[String],
    arg_tokens: &[Token],
    class_def: &mut ClassDef,
    side: MemberSide,
) {
    let Some(retraction) = member.retraction else {
        return;
    };
    let words = retraction.split(names);
    // The name the retracted member re-appears under, with the word whose span
    // becomes its synthetic declaration site. A dynamic destination
    // (`renamemethod old $new`) names nothing statically, so the move abstains
    // and the source is simply retracted — a false negative, the direction this
    // campaign abstains toward.
    let arrival = words
        .arrives_at
        .and_then(|i| Some((names.get(i)?, *arg_tokens.get(i)?)))
        .filter(|(name, tok)| !has_substitution(name, tok));
    for (i, name) in words.retracted.iter().enumerate() {
        let tok = arg_tokens.get(i).copied();
        // A dynamic source name (`deletemethod $m`) resolves to nothing
        // statically: it may or may not name a live member, so neither the
        // removal, the tombstone, nor the abort has evidence behind it.
        if tok.is_some_and(|t| has_substitution(name, &t)) {
            continue;
        }
        let Some(removed) = side.table(class_def).remove(name) else {
            // Nothing local to remove: the cross-file stub shape.  The
            // tombstone carries the *arrival* too when the retracting word was
            // a move — the stub has no `MethodDef` to re-key, so
            // recording where the member goes is the only way the workspace
            // join can put it there.  Its own arrival word is the synthetic
            // declaration site, exactly as it is for a same-file move.
            class_def
                .retracted_members
                .push(crate::analyser::types::MemberRetractionRecord {
                    member: name.clone(),
                    side,
                    arrival: arrival.map(|(new_name, _)| new_name.clone()),
                    arrival_span: arrival.map(|(_, tok)| tok.span),
                });
            if let Some(tok) = tok {
                class_def.definition_aborts.push(DefinitionAbort {
                    kind: DefinitionAbortKind::MissingMember,
                    member: name.clone(),
                    span: tok.span,
                });
            }
            // Nothing was removed, so there is nothing to move and no
            // destination question to ask: real Tcl fails on the source word
            // first (`renamemethod ghost b` reports `method ghost does not
            // exist`, never `method called b already exists`), and one word
            // earns one report.
            continue;
        };
        let Some((new_name, new_tok)) = arrival else {
            continue;
        };
        // The member state this move runs against, captured with the source
        // already removed and the destination not yet inserted — exactly the
        // table the interpreter reads at this point in the body.
        let moved = RenamedMember {
            source: name.clone(),
            destination: new_name.clone(),
            side,
            destination_span: new_tok.span,
            blocked: side.table(class_def).keys().cloned().collect(),
        };
        // One fold decides both readings of that state: whether the destination
        // actually written aborts the definition (`W315`, below) and whether
        // some *other* name would (the rename gate, which asks the recorded
        // `RenamedMember` the same question). Deciding them in one place is what
        // stops the diagnostic and the gate from drifting apart.
        //
        // A rename real Tcl rejects moves nothing — and rather than let the
        // wreckage take a declaration with it, the source stays under its own
        // name and an existing destination keeps its own body. Both members the
        // author wrote are then still navigable, which is the whole point of
        // recording a class that cannot run.
        if let Some(kind) = moved.abort_if_renamed_to(new_name) {
            class_def.definition_aborts.push(DefinitionAbort {
                kind,
                member: new_name.clone(),
                span: new_tok.span,
            });
            side.table(class_def).insert(name.clone(), removed);
            continue;
        }
        let mut md = removed;
        // Only the identity moves: the body span still points at the one real
        // body, and the visibility is the source's (oracle above), not the
        // destination name's family default.
        md.name.clone_from(new_name);
        md.name_span = new_tok.span;
        side.table(class_def).insert(new_name.clone(), md);
        class_def.renamed_members.push(moved);
    }
}

/// Apply a member word's **visibility effect** — `export` / `unexport` — to the
/// members it names, scoped to `side`.
///
/// A no-op for a member carrying no effect: which words set visibility is
/// registry data ([`MemberSpec::visibility_effect`]), never a keyword matched
/// here, exactly as [`MemberSpec::retraction`] decides which words
/// retract. The sibling [`MemberRefKind::Method`] member `filter` names a method
/// and does neither, so it falls through untouched, and a definition grammar for
/// another class system gets the same handling by declaring the effect on its
/// own member word.
///
/// The names also update the class-level visibility sets **of that side** —
/// `exports`/`unexports` for the instance side, `class_exports`/
/// `class_unexports` for the class-object side ([`MemberSide::visibility_sets`]).
/// The pair is what carries the flip across documents: a `via_define` stub
/// declares no member to flip, so without a recorded set a `self unexport m` in
/// one file never reaches another file's class-command dispatch.
/// The two pairs stay strictly apart — `exports`/`unexports` are the
/// *instance*-side record by contract (`workspace_index`'s effective-export
/// union over `instance_method`, `rename_safety`'s "the class declares this
/// name" test), so a class-object-side flip landing there would silently
/// re-state an unrelated instance method's export bit.
///
/// Within a pair the two sets are maintained **mutually exclusive**, because
/// each is a record of the *last* writer for the name and the union consumer
/// reads any `exports` entry as decisive: `method m {} {…} ; export m ;
/// unexport m` must leave `m` in `unexports` alone, or cross-file dispatch would
/// keep advertising a method that `[C new] m` rejects with `unknown method "m"`
/// (tclsh 9.0.4 / 8.6.14).
fn apply_visibility_member(
    member: &MemberSpec,
    names: &[String],
    class_def: &mut ClassDef,
    side: MemberSide,
) {
    let Some(effect) = member.visibility_effect else {
        return;
    };
    let visibility = match effect {
        MemberVisibility::Exported => "public",
        MemberVisibility::Unexported => "unexported",
    };
    let (exports, unexports) = side.visibility_sets(class_def);
    let (set, cleared) = match effect {
        MemberVisibility::Exported => (exports, unexports),
        MemberVisibility::Unexported => (unexports, exports),
    };
    for name in names {
        set.insert(name.clone());
        cleared.remove(name);
    }
    set_member_visibility(class_def, names, visibility, side);
}

/// Fold a relation row into the graph its slot feeds, on the side the row
/// resolved to.
///
/// `superclasses` / `mixins` feed the class-hierarchy graph (inherited
/// methods, MRO).  The navigable *references* to those base classes are
/// recorded separately as `command_invocations` by
/// `record_member_command_references`, so no per-name span is kept here.
/// A class object's own ancestry is not the class's (`self mixin M` mixes
/// into the class object, which no instance inherits), so a class-side
/// superclass or mixin row records nothing.
///
/// The two filter slots are separate slots that intercept different
/// dispatches (see [`ClassDef::filters`] and [`ClassDef::class_filters`] for
/// the oracle), so a `filter` call lands in the slot of the side its wrapper
/// put it on, never in one flat list: `self filter f` fills the class
/// object's own filter slot, which intercepts dispatches on the *class
/// command* (`::B cls`, and even `::B new`) while leaving instances unfiltered
/// — `info object filters ::B` -> `f`, `info class filters ::B` -> empty, on
/// tclsh 9.0.4 and 8.6.14 alike — and `private filter s` fills the
/// *instance* slot (`info class filters`, tclsh 9.0.4; `private` does not
/// exist on 8.6).
///
/// What the word does to the slot is registry data ([`MemberSpec::slot`]):
/// `filter a; filter b` leaves both live (`-append` default — tclsh 9.0.4 /
/// 8.6.16: `info class filters` → `a b`), `superclass` / `mixin` replace
/// (`-set` default — tclsh 9.0.4: `superclass A` then `superclass -append B`
/// → `::A ::B`), and the explicit operations fold through
/// [`tcl_registry::definer::SlotSpec::apply`] — the same fold every slot
/// consumer uses, so they cannot diverge.
fn apply_relation_member(
    member: &MemberSpec,
    slot: RelationSlot,
    args: &[String],
    class_def: &mut ClassDef,
    side: MemberSide,
) {
    let list = match (slot, side) {
        (RelationSlot::Superclass, MemberSide::Instance) => &mut class_def.superclasses,
        (RelationSlot::Mixin, MemberSide::Instance) => &mut class_def.mixins,
        (RelationSlot::Filter, side) => side.filter_list(class_def),
        (RelationSlot::Superclass | RelationSlot::Mixin, MemberSide::ClassObject) => return,
    };
    apply_slot_member(Some(member), args, list);
}

/// Fold one slot-member call (`filter` / `superclass` / `mixin` /
/// `variable`) into `list` through the member's registry [`SlotSpec`]
/// — the one place the analyser applies slot semantics, so
/// the instance / class-object filter slots, the superclass list, the mixin
/// list, and the declared-variable slot all take the identical fold.
///
/// A member with no slot spec — a relation that is a plain list of class
/// references, like itcl's `inherit`, which takes no operation words — adds
/// the names it lists.
pub(super) fn apply_slot_member(
    member: Option<&MemberSpec>,
    args: &[String],
    list: &mut Vec<String>,
) {
    match member.and_then(|m| m.slot) {
        Some(slot) => slot.apply(list, args),
        None => list.extend(args.iter().cloned()),
    }
}

/// Set the recorded visibility of each named member on `side` only to
/// `visibility` — the `export` / `unexport` member effect.
///
/// A name with no recorded member **on that side** is skipped: a later `method`
/// definition re-applies the name default anyway (tclsh 9.0.4-pinned), an
/// export-only stub has no declaration to navigate to, and — pinned on 9.0.4 and
/// 8.6.14 alike — naming a method that exists only on the *other* side really is
/// a no-op in Tcl, so skipping it is the faithful answer, not an abstention.
fn set_member_visibility(
    class_def: &mut ClassDef,
    names: &[String],
    visibility: &str,
    side: MemberSide,
) {
    let table = side.table(class_def);
    for name in names {
        if let Some(md) = table.get_mut(name) {
            md.visibility = visibility.to_string();
        }
    }
}

/// `oo::define Cls forward name target ...` — records a forward
/// alias as a method, keeping the target command and any prepended
/// arguments (`forward`'s own version of `interp alias` partial
/// application) so a call through the forward can be arity-checked
/// against `target`'s own signature, shifted by the prepended count —
/// see `Analyser::resolve_indirect_call_target`.  Which words are the name and
/// the target is the row's [`MemberEffect::Forward`] slots.
fn apply_oo_forward(
    statement: &MemberStatement<'_>,
    class_def: &mut ClassDef,
    dialect: Option<SurfaceQuery<'_>>,
) {
    let MemberEffect::Forward {
        name_slot,
        prefix_slot,
    } = statement.row.effect
    else {
        return;
    };
    let (args, tokens) = (statement.args(), statement.arg_tokens());
    let (name_slot, prefix_slot) = (usize::from(name_slot), usize::from(prefix_slot));
    if let Some(name) = args.get(name_slot) {
        let span = tokens
            .get(name_slot)
            .map_or(tcl_lexer::Span::new(0, 0), |token| token.span);
        let forward_target = args.get(prefix_slot).map(|target| {
            (
                target.clone(),
                args.get(prefix_slot + 1..).unwrap_or(&[]).to_vec(),
            )
        });
        let md = MethodDef {
            name: name.clone(),
            params: Vec::new(),
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span: span,
            body_span: span,
            kind: "forward".to_string(),
            is_self_method: false,
            // A forward is dispatched by the same name rule as a method
            // (C computes `isPublic` identically for both).
            visibility: statement.recorded_visibility(name, dialect),
            doc: String::new(),
            forward_target,
        };
        class_def.methods.insert(md.name.clone(), md);
    }
}

#[cfg(test)]
pub(in super::super) fn apply_oo_subcommand(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    class_def: &mut ClassDef,
) {
    apply_oo_subcommand_in(
        grammar,
        texts,
        argv,
        class_def,
        MemberDialect {
            surface: None,
            rules: WordValueRules::TCL,
        },
    );
}

/// Apply one registry member using the selected dialect's concrete layout.
///
/// The statement's [`MemberRow`] decides everything: its effect says what the
/// member declares ([`member_landing`], the routing the snit and itcl walkers
/// share) and its resolved receiver says which side it lands on. A wrapper's
/// shift is already in that receiver, so `self` and `private` have no arm of
/// their own: `self method` lands among the class methods because its row's
/// receiver is the type object, `private method` among the instance methods
/// with the visibility its shift declares, and the block forms reach here
/// member by member, prefixed with their wrapper by
/// [`expand_wrapper_block_members`]. A statement the registry cannot read
/// records nothing.
///
/// A member word that *removes* the members it names — `deletemethod m`,
/// `renamemethod old new` — and one that flips their visibility — `export`
/// / `unexport` — act on their row's side and nowhere else: written with no
/// wrapper, the **instance** side; under `self`, the class-object side;
/// under `private`, `private`'s own (instance) side, including a member the
/// same block just recorded. Oracle, byte-identical on tclsh 9.0.4 and
/// 8.6.14:
///
/// ```tcl
/// oo::class create ::I1 { method gone {} {…}; method kept {} {…}
///                         deletemethod gone }
/// info class methods ::I1   ;# -> kept
/// oo::class create ::I3 { method old {} {…}; renamemethod old new }
/// info class methods ::I3   ;# -> new          (`old` really is gone)
/// oo::class create ::I4 { method gone {} {…} }
/// oo::define ::I4 { deletemethod gone }
/// info class methods ::I4   ;# -> (empty)
/// oo::class create C { method m {} {…}
///                      self { method m {} {…}; unexport m } }
/// info object methods ::C   ;# -> (empty)     class-side `m` unexported
/// info class methods ::C    ;# -> m           instance side untouched
/// ```
///
/// and `oo::class create E2 { self { method onlyclass {} {…} } }` then
/// `oo::define E2 { unexport onlyclass }` leaves the class-object side's
/// `onlyclass` exported and dispatchable, while `oo::define E { self
/// unexport onlyinst }` naming a method only the *other* side has is a
/// silent no-op, not the hard error `deletemethod` raises.
pub(in super::super) fn apply_oo_subcommand_in(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    class_def: &mut ClassDef,
    dialect: MemberDialect<'_>,
) {
    let Some(statement) = MemberStatement::read(grammar, texts, argv, dialect.surface) else {
        return;
    };
    let Some(side) = member_side(statement.row.receiver) else {
        return;
    };
    let (member, words, tokens) = (statement.member, statement.args(), statement.arg_tokens());
    match statement.landing() {
        MemberLanding::Method(kind) => record_declared_method(&statement, kind, class_def, dialect),
        // `class_methods` holds the `classmethod`-kind entries the
        // class-command dispatch reads, so a class-object forward
        // (`self forward`) stays unrecorded.
        MemberLanding::Forward if side == MemberSide::Instance => {
            apply_oo_forward(&statement, class_def, dialect.surface);
        }
        // Additive, not a reset (tclsh9.0-verified: `oo::define Cls
        // variable a b; oo::define Cls variable c` leaves all of `a`, `b`, `c`
        // live simultaneously), because the class `variable` word is a slot
        // whose default operation is `-append` (with dedup — tclsh 9.0.4:
        // `variable a; variable a b` → `a b`); the explicit `-set` / `-clear`
        // / `-remove` operations fold through the same registry fold as every
        // other slot. A class-object variable (`self variable`) is no instance
        // variable of the class.
        MemberLanding::State(_) if side == MemberSide::Instance => {
            apply_slot_member(Some(member), words, &mut class_def.variables);
        }
        MemberLanding::Relation(slot) => {
            apply_relation_member(member, slot, words, class_def, side);
        }
        MemberLanding::Retraction => retract_named_members(member, words, tokens, class_def, side),
        MemberLanding::Visibility => apply_visibility_member(member, words, class_def, side),
        MemberLanding::Accessors if side == MemberSide::Instance => {
            extract_property_defs(words, tokens, class_def);
        }
        // A class-level init script's body is collected and walked on its
        // own ([`collect_class_level_bodies`]); a procedure, a configuration
        // word and the wrapped spellings of the sided arms above record
        // nothing here.
        _ => {}
    }
}

/// Record a callable member row as the [`MethodDef`] its kind names.
///
/// The name, parameter list and body words come from the member's registry
/// layout ([`extract_method_def_in`]); a nameless member (a constructor or
/// destructor) is recorded under its synthetic `<keyword>` name and anchored on
/// its keyword, where an editor lands for go-to-definition and hover. A method
/// (re)definition takes the visibility its row names — the family's name-based
/// default unless its option word or wrapper says otherwise — discarding any
/// earlier explicit `export`/`unexport` of the name (tclsh 9.0.4-pinned; the
/// rule itself is registry data — `[a-z]*` for `TclOO`).
///
/// A member a receiver-moving wrapper put on the class object (`self method
/// NAME ARGS BODY`, `TclOO`'s own spelling of a class-level method) is tagged
/// `is_self_method`, so the class-command MRO walk in `tcl-lsp-core` knows not
/// to treat it as inherited the way a `classmethod` is (real tclsh: a subclass
/// with no override does not gain a `self method` at all).
fn record_declared_method(
    statement: &MemberStatement<'_>,
    kind: MethodKind,
    class_def: &mut ClassDef,
    dialect: MemberDialect<'_>,
) {
    let synthetic = format!("<{}>", statement.keyword());
    let Some(mut md) = extract_method_def_in(
        statement.member,
        statement.args(),
        statement.arg_tokens(),
        kind.as_str(),
        "public",
        &synthetic,
        dialect,
    ) else {
        return;
    };
    if matches!(
        statement.row.effect,
        MemberEffect::Callable {
            name_slot: None,
            ..
        }
    ) && let Some(keyword) = statement.keyword_token()
    {
        md.name_span = keyword.span;
    }
    md.visibility = statement.recorded_visibility(&md.name, dialect.surface);
    md.is_self_method = kind == MethodKind::ClassMethod && statement.shift().receiver.is_some();
    record_method(class_def, kind, md);
}

/// Extract property definitions from a ``property`` subcommand.
///
/// Walks the args, splitting names from option values
/// (``-get BODY``, ``-set BODY``, ``-kind readable|writable|readwrite``).
/// Each property gets a [`PropertyDef`] entry in
/// ``class_def.properties``.
///
/// All property options take a value (``-get``, ``-set``,
/// ``-kind``); there are no flag-only options.  When ``-kind``
/// is omitted the property defaults to ``"readwrite"``.
///
/// This records only the class-level `PropertyDef` entries; the
/// accessor (`-get` / `-set`) bodies are collected and walked
/// separately by [`collect_property_accessor_bodies`].
fn extract_property_defs(args: &[String], arg_tokens: &[Token], class_def: &mut ClassDef) {
    let zero = Span::new(0, 0);

    // Collect property names + their per-arg index, then the
    // trailing options, in a two-pass shape.
    let mut names: Vec<(String, usize)> = Vec::new();
    let mut kind = "readwrite".to_string();
    let mut getter_defined = false;
    let mut setter_defined = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(stripped) = arg.strip_prefix('-') {
            // All property options take a value.
            if i + 1 >= args.len() {
                i += 1;
                continue;
            }
            let value = &args[i + 1];
            match stripped {
                "kind" => kind.clone_from(value),
                // registry-axis-ok: definition_body — property accessor keywords of the class body; until the definition-body grammar exposes property accessors
                "get" => getter_defined = true,
                // registry-axis-ok: definition_body — property accessor keywords of the class body; until the definition-body grammar exposes property accessors
                "set" => setter_defined = true,
                _ => {}
            }
            i += 2;
            continue;
        }
        names.push((arg.clone(), i));
        i += 1;
    }

    for (name, idx) in names {
        let span = arg_tokens.get(idx).map_or(zero, |t| t.span);
        class_def.properties.insert(
            name.clone(),
            PropertyDef {
                name,
                name_span: span,
                kind: kind.clone(),
                has_getter: getter_defined,
                has_setter: setter_defined,
            },
        );
    }
}

/// Collect the `-get` / `-set` accessor bodies of a `property` subcommand as
/// walkable method bodies (named `<get>` / `<set>`).  Only braced (`Str`)
/// bodies are walkable.
pub(super) fn collect_property_accessor_bodies(
    texts: &[String],
    argv: &[Token],
    out: &mut Vec<CollectedMethodBody>,
) {
    let mut i = 0;
    while i < texts.len() {
        if let Some(opt) = texts[i].strip_prefix('-') {
            // Every property option takes a value; only `-get`/`-set` carry a
            // body to analyse.
            if i + 1 < texts.len() {
                // registry-axis-ok: definition_body — property accessor keywords of the class body; until the definition-body grammar exposes property accessors
                if matches!(opt, "get" | "set")
                    && let Some(tok) = argv.get(i + 1).copied()
                    && tok.kind == TokenType::Str
                {
                    out.push(CollectedMethodBody {
                        name: format!("<{opt}>"),
                        params: Vec::new(),
                        body_text: texts[i + 1].clone(),
                        body_tok: tok,
                        params_tok: None,
                        // Conservative: a `property` accessor body is a
                        // 9.0+ `oo::configurable` surface this analysis has
                        // not pinned `[self class]` behaviour for — abstain
                        // from the defining-class fact rather than guess.
                        class_side: true,
                    });
                }
                i += 2;
                continue;
            }
        }
        i += 1;
    }
}

/// Extract a [`MethodDef`] from a method-shaped member's args.
///
/// The name / parameter-list / body word positions come from `member`'s
/// registry arg-roles (`method NAME PARAMS BODY`, `constructor PARAMS BODY`,
/// `destructor BODY`), never hardcoded indices.  A member with no
/// [`ArgRole::Name`] (constructor / destructor) takes `synthetic_name` as its
/// placeholder.  `args` / `arg_tokens` are the words *after* the member
/// keyword.
///
/// Returns `None` when the argument count is too short (no body word) to match
/// the member's shape.
#[cfg(test)]
pub(super) fn extract_method_def(
    member: &MemberSpec,
    args: &[String],
    arg_tokens: &[Token],
    kind: &str,
    visibility: &str,
    synthetic_name: &str,
) -> Option<MethodDef> {
    extract_method_def_in(
        member,
        args,
        arg_tokens,
        kind,
        visibility,
        synthetic_name,
        MemberDialect {
            surface: None,
            rules: WordValueRules::TCL,
        },
    )
}

pub(in super::super) fn extract_method_def_in(
    member: &MemberSpec,
    args: &[String],
    arg_tokens: &[Token],
    kind: &str,
    visibility: &str,
    synthetic_name: &str,
    dialect: MemberDialect<'_>,
) -> Option<MethodDef> {
    let zero = tcl_lexer::Span::new(0, 0);
    let body_idx = member
        .indices_for_call_in(args, dialect.surface, ArgRole::Body)
        .next()?;
    // A body word must be present for the member to be well-formed.
    args.get(body_idx)?;
    let name_idx = member
        .indices_for_call_in(args, dialect.surface, ArgRole::Name)
        .next();
    let params_idx = member
        .indices_for_call_in(args, dialect.surface, ArgRole::ParamList)
        .next();

    let name = name_idx
        .and_then(|i| args.get(i))
        .map_or_else(|| synthetic_name.to_string(), String::clone);
    let params = params_idx
        .and_then(|i| args.get(i))
        .map_or_else(Vec::new, |p| parse_param_list(p, dialect.rules));
    let name_span = name_idx
        .and_then(|i| arg_tokens.get(i))
        .map_or(zero, |t| t.span);
    let body_span = arg_tokens.get(body_idx).map_or(zero, |t| t.span);
    Some(MethodDef {
        name,
        params,
        params_computed: false,
        formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
            tcl_dialect::ParameterGrammar::Tcl,
        ),
        name_span,
        body_span,
        kind: kind.to_string(),
        // Flipped by the one caller that needs it (`record_declared_method`,
        // for a member a receiver-moving wrapper put on the class object) —
        // every other caller means it literally, so `false` is the
        // correct default here, not just a placeholder.
        is_self_method: false,
        visibility: visibility.to_string(),
        doc: String::new(),
        forward_target: None,
    })
}
