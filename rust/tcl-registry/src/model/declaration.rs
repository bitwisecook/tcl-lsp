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

//! **Document- and workspace-declared commands** — gap ruling R1
//! (`docs/design/registry/dialect-and-package-registry-centralisation.md` §4).
//!
//! A `# tcl-lsp: stub NAME {ARGS}` block in the analysed buffer, and a
//! workspace `<environment>.tcl.stubs` sidecar, both say the same kind of
//! thing the catalogue says: *this name is a command, and these are its
//! argument roles*.
//!
//! Here they ingest as ordinary [`SurfaceDeclaration`]s, in the registry's
//! own vocabulary — one role-word table and one `arg_indices_for_role`,
//! with a tracked provenance — rather than a second, parallel
//! representation every consumer would otherwise have to consult beside
//! the registry:
//!
//! - the **provider** is [`Provider::Document`] — active exactly in the
//!   buffer that declared it, which is why such a declaration never joins
//!   a shared [`crate::model::ContextRegistry`] generation (a generation
//!   is keyed by environment, and this row's scope is one document);
//! - the **applicability** is the whole [document
//!   axis](tcl_dialect::model::VersionAxisId::document) — a buffer has no
//!   release train, so the declaration holds for as long as it is written;
//! - the **provenance** is [`Provenance::Document`] for an inline block
//!   and [`Provenance::WorkspaceUntrusted`] for a sidecar — a label for
//!   explanation, binding selection and invalidation, not a precision
//!   class: the declaration's facts are believed either way;
//! - the **argument roles** are the registry's own [`ArgRole`], resolved
//!   by the registry's own role-word table ([`role_for_word`]), so a stub
//!   argument and a catalogue argument are the same kind of fact;
//! - the **behavioural facts** a stub's flags state land on the fields a
//!   catalogue command states them on — [`Traits`] and [`SideEffect`]s
//!   ([`DeclaredCommand::traits`], [`DeclaredCommand::side_effects`]).
//!
//! [`DeclaredSurface`] is the per-document generation of those rows, and
//! [`DocumentCommandSurface`] is **the** door onto the command surface one
//! document analyses against: catalogue generation plus that document's own
//! declarations, asked once. No consumer consults the catalogue and then a
//! second table.
//!
//! ## Nearest wins
//!
//! A declaration is a workspace-authored fact on the same footing as a
//! shipped spec (`docs/design/compiler/registry-consumer-contracts.md`
//! § *Ruling — a stub sidecar is a workspace-authored fact*), so for a name
//! the document declares, the declaration answers — its roles, its traits,
//! its side effects — and the catalogue answers every other name. The one
//! exception is the security floor (invariant I6): a declaration that
//! redeclares a shipped command keeps that command's security traits and its
//! side effects beneath its own, exactly as a pack override does
//! ([`SecurityFloor`]).
//!
//! ## Why the availability check is context-free
//!
//! A [`Provider::Document`] row is unconditional by construction: its
//! provider is active wherever the row is held, its applicability is the
//! full document axis, and its predicate is
//! [`CapabilityPredicate::None`]. So running the ordinary
//! [`ContextQueries::is_available`](crate::model::ContextQueries::is_available)
//! over it can only answer `true`, and [`DeclaredSurface`] does not thread
//! a [`ResolvedContext`](crate::model::ResolvedContext) it would learn
//! nothing from. `declared_rows_are_available_under_the_ordinary_queries`
//! pins that equivalence against the real context queries rather than
//! asserting it in prose.

use std::borrow::Cow;
use std::collections::BTreeMap;

use tcl_dialect::model::{ItemHistory, Provenance, SurfaceQuery, VersionAxisId, VersionSet};

use crate::arg_role::{AppendedArity, ArgRole};
use crate::extension_default;
use crate::frame_effect::{FrameArgLayout, FrameEffectSpec, FrameLevelWord};
use crate::model::surface::{CapabilityPredicate, Provider, SurfaceDeclaration};
use crate::security_floor::SecurityFloor;
use crate::side_effects::SideEffect;
use crate::spec::CommandSpec;
use crate::traits::Traits;

/// Map a stub directive's role word to the registry's own [`ArgRole`], or
/// `None` when the word names no role.
///
/// **The** stub role vocabulary: the directive parser rejects a declaration
/// whose role word this does not know, and every other consumer
/// canonicalises through it. A second list of accepted words beside this one
/// is how a role gets documented but stays unusable.
#[must_use]
pub fn role_for_word_checked(word: &str) -> Option<ArgRole> {
    Some(match word {
        "body" => ArgRole::Body,
        "expr" => ArgRole::Expr,
        "var" => ArgRole::VarWrite,
        "var_read" => ArgRole::VarRead,
        "name" => ArgRole::Name,
        "pattern" => ArgRole::Pattern,
        "channel" => ArgRole::Channel,
        "command_prefix" => ArgRole::CommandPrefix,
        "value" => ArgRole::Value,
        _ => return None,
    })
}

/// [`role_for_word_checked`] with the "value is the default" fallback an
/// argument written without a `:role` annotation gets.
#[must_use]
pub fn role_for_word(word: &str) -> ArgRole {
    role_for_word_checked(word).unwrap_or(ArgRole::Value)
}

/// What a declaration states of its command's frame effect — the stub
/// grammar's `-frame` word — in the registry's own vocabulary: the
/// frame-crossing argument grammar a catalogue command carries on
/// [`CommandSpec::frame_effect`](crate::CommandSpec), `None` for a command
/// that crosses no frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DeclaredFrameEffect {
    /// The declaration states none, so nothing bounds what the command does
    /// to the frame that calls it: like code the module cannot see, it may
    /// reach that frame (`upvar 1`, `uplevel 1`).
    #[default]
    Unstated,
    /// The declaration states the command's frame effect: `None` for
    /// `-frame own` and `-frame none`, which cross no frame, and
    /// [`DeclaredFrameEffect::CALLER`] for `-frame caller`.
    Stated(Option<FrameEffectSpec>),
}

impl DeclaredFrameEffect {
    /// The effect `-frame caller` states, `argparse`'s: variables of the
    /// frame that calls the command, reached under names nothing states.
    pub const CALLER: FrameEffectSpec = FrameEffectSpec {
        level_word: FrameLevelWord::None,
        layout: FrameArgLayout::OpaqueCallerVars,
    };

    /// The effect a stub's `-frame` word states, or `None` for a word the
    /// grammar does not know. `own` (the command's code runs in a frame of
    /// its own, as a procedure's does) and `none` (it runs no code that
    /// reaches a frame) state the same value, a command that crosses no
    /// frame; `caller` states [`Self::CALLER`].
    #[must_use]
    pub fn from_stub_word(word: &str) -> Option<Self> {
        match word {
            "own" | "none" => Some(Self::Stated(None)),
            "caller" => Some(Self::Stated(Some(Self::CALLER))),
            _ => None,
        }
    }

    /// The effect a stub's trailing flag run states with `-frame WORD`
    /// ([`Self::from_stub_word`]), the last one written winning; unstated
    /// when the run has none, or names a word the grammar does not know,
    /// which is ignored as an unrecognised flag is.
    #[must_use]
    pub fn from_stub_flags(flags: &str) -> Self {
        let mut stated = Self::Unstated;
        let mut words = flags.split_whitespace();
        while let Some(word) = words.next() {
            if word == "-frame"
                && let Some(effect) = words.next().and_then(Self::from_stub_word)
            {
                stated = effect;
            }
        }
        stated
    }
}

/// The full [document axis](VersionAxisId::document) — a declared command
/// exists for as long as its declaration is written.
fn whole_document_axis() -> VersionSet {
    VersionSet::from_requirements(VersionAxisId::document(), &["0-"])
        .expect("the full-axis requirement is well-formed")
}

/// One argument of a declared command: its written name, its registry
/// [`ArgRole`], and whether the directive wrapped it in `?…?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredArgument {
    /// Argument name as the directive wrote it (optional markers stripped).
    pub name: String,
    /// The registry argument role this position carries.
    pub role: ArgRole,
    /// `true` when the directive wrote the argument as `?name?`.
    pub optional: bool,
}

/// One command a document or its workspace declares for itself, with the
/// [`SurfaceDeclaration`] that says who provides it and under what trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredCommand {
    /// The declared command name.
    pub name: String,
    /// Parameters in declaration order.
    pub arguments: Vec<DeclaredArgument>,
    /// The behavioural traits the declaration states, on the fields a
    /// catalogue command states the same facts on — a stub's `-pure` is
    /// [`Traits::PURE`], its `-loop` [`Traits::HAS_LOOP_BODY`]. Empty for a
    /// declaration that states none.
    pub traits: Traits,
    /// The side effects the declaration states — a stub's `-mutator` is a
    /// read and a write of
    /// [`SideEffectTarget::Variable`](crate::side_effects::SideEffectTarget::Variable).
    pub side_effects: Vec<SideEffect>,
    /// The frame effect the declaration states — a stub's `-frame` word —
    /// or [`DeclaredFrameEffect::Unstated`].
    pub frame_effect: DeclaredFrameEffect,
    /// The §4.1 surface row this declaration ingested as.
    pub declaration: SurfaceDeclaration,
}

impl DeclaredCommand {
    /// Declare `name` with `arguments`, provided by the document itself at
    /// `provenance`, stating no traits and no side effects
    /// ([`Self::with_traits`] and [`Self::with_side_effects`] add them).
    ///
    /// `provenance` is the source's trust class —
    /// [`Provenance::Document`] for an inline `# tcl-lsp: stub` block,
    /// [`Provenance::WorkspaceUntrusted`] for a `.tcl.stubs` sidecar.
    #[must_use]
    pub fn new(name: String, arguments: Vec<DeclaredArgument>, provenance: Provenance) -> Self {
        Self {
            name,
            arguments,
            traits: Traits::empty(),
            side_effects: Vec::new(),
            frame_effect: DeclaredFrameEffect::Unstated,
            declaration: SurfaceDeclaration {
                provider: Provider::Document,
                applicable: whole_document_axis(),
                predicate: CapabilityPredicate::None,
                history: ItemHistory::default(),
                provenance,
            },
        }
    }

    /// The same declaration, stating `traits`.
    #[must_use]
    pub fn with_traits(mut self, traits: Traits) -> Self {
        self.traits = traits;
        self
    }

    /// The same declaration, stating `side_effects`.
    #[must_use]
    pub fn with_side_effects(mut self, side_effects: Vec<SideEffect>) -> Self {
        self.side_effects = side_effects;
        self
    }

    /// The same declaration, stating `frame_effect`.
    #[must_use]
    pub fn with_frame_effect(mut self, frame_effect: DeclaredFrameEffect) -> Self {
        self.frame_effect = frame_effect;
        self
    }

    /// The roles a reader that does not hold this declaration still reads
    /// right: data — a value, a name, a pattern, a channel. A script, an
    /// expression, a command prefix and a variable name are words only a
    /// reader of the declaration knows to follow.
    const DATA_ROLES: [ArgRole; 4] = [
        ArgRole::Value,
        ArgRole::Name,
        ArgRole::Pattern,
        ArgRole::Channel,
    ];

    /// The traits a reader that does not hold this declaration may go
    /// without: `-pure`'s, whose absence only declines what purity allows,
    /// and `-unsafe`'s, which speak of safe interpreters alone.
    const INERT_TRAITS: Traits = Traits::PURE
        .union(Traits::UNSAFE)
        .union(Traits::SAFE_INTERP_HIDDEN);

    /// The frame effect the declaration states, when that is all a reader
    /// holding the catalogue alone has to learn from it to read a call to
    /// the command right: every argument is data ([`Self::DATA_ROLES`]), and
    /// it states no trait but `-pure`'s or `-unsafe`'s and no side effect.
    ///
    /// A flow graph is such a reader — it reads roles and traits off the
    /// catalogue, which answers a name it does not hold as a command of
    /// plain values that states nothing — so it may name the command, and a
    /// call to it brings the effect stated here, as a catalogue command's
    /// call brings [`CommandSpec::frame_effect`]. `None` for a declaration
    /// that states no frame effect, or states a role or a fact that reader
    /// would miss: a call to its command stays a call to code the module
    /// cannot see.
    #[must_use]
    pub fn plain_call_frame_effect(&self) -> Option<Option<FrameEffectSpec>> {
        let DeclaredFrameEffect::Stated(effect) = self.frame_effect else {
            return None;
        };
        (self
            .arguments
            .iter()
            .all(|argument| Self::DATA_ROLES.contains(&argument.role))
            && Self::INERT_TRAITS.contains(self.traits)
            && self.side_effects.is_empty())
        .then_some(effect)
    }

    /// Declare `name` as a command a native extension registers: the
    /// conservative default of [`crate::extension_default`] on every axis,
    /// provided by the document itself at `provenance`.
    ///
    /// Its traits and side effects are those of
    /// [`CommandSpec::extension_default`], so a stub for an extension's command
    /// reads the way the registry's own default does to every consumer that
    /// asks the surface. What the declaration states narrows them, through
    /// [`Self::narrowed_by`].
    #[must_use]
    pub fn extension(
        name: String,
        arguments: Vec<DeclaredArgument>,
        provenance: Provenance,
    ) -> Self {
        let default = CommandSpec::extension_default("");
        Self::new(name, arguments, provenance)
            .with_traits(default.traits)
            .with_side_effects(default.side_effects.to_vec())
    }

    /// Narrow this declaration by facts it states, axis by axis.
    ///
    /// A statement of *effects* — [`Traits::PURE`], or a side effect of its
    /// own — is the declaration saying what the command does to state, so it
    /// replaces the default's effect axes ([`extension_default::EFFECT_AXES`]:
    /// code evaluation and traces, with the unknown read and write) by what it
    /// states: a command that is pure, or that only rewrites its target,
    /// evaluates no code. Every other axis — taint, safety — stays where the
    /// default put it unless the declaration states it. The traits it states
    /// are added whether or not they narrow anything, so a stated barrier
    /// survives a stated effect.
    #[must_use]
    pub fn narrowed_by(mut self, traits: Traits, side_effects: Vec<SideEffect>) -> Self {
        if traits.contains(Traits::PURE) || !side_effects.is_empty() {
            self.traits.remove(extension_default::EFFECT_AXES);
            self.side_effects = side_effects;
        }
        self.traits.insert(traits);
        self
    }

    /// The trust class of the source that declared this command.
    #[must_use]
    pub const fn provenance(&self) -> Provenance {
        self.declaration.provenance
    }

    /// The 0-based indices **into a call's own post-head argument list**
    /// whose declared role is `role`, for a call supplying `supplied` words.
    ///
    /// A declared position is not a call position: a declaration is a shape
    /// with optional slots, so `{?table? row:var}` invoked as `fetch out`
    /// writes `out` at index 0, not at the declared index 1. Optional slots
    /// fill left to right, Tcl's own convention, so the number of them
    /// present is whatever the call carries beyond the required words. A call
    /// with fewer words than the declaration requires cannot be laid out at
    /// all and maps to nothing, rather than to positions it does not have.
    #[must_use]
    pub fn arg_indices_for_role(&self, role: ArgRole, supplied: usize) -> Vec<usize> {
        let required = self
            .arguments
            .iter()
            .filter(|argument| !argument.optional)
            .count();
        let Some(mut optionals_present) = supplied.checked_sub(required) else {
            return Vec::new();
        };
        let mut indices = Vec::new();
        let mut position = 0;
        for argument in &self.arguments {
            if argument.optional {
                if optionals_present == 0 {
                    continue;
                }
                optionals_present -= 1;
            }
            if position >= supplied {
                break;
            }
            if argument.role == role {
                indices.push(position);
            }
            position += 1;
        }
        indices
    }
}

/// One document's declared command surface: every [`DeclaredCommand`] the
/// buffer and its workspace sidecar contribute, in name order.
///
/// A later declaration of the same name replaces an earlier one — the
/// "last directive wins" rule, and the rule that makes an inline block
/// override a sidecar of the same name when the caller ingests the sidecar
/// first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclaredSurface {
    commands: BTreeMap<String, DeclaredCommand>,
}

impl DeclaredSurface {
    /// An empty surface — a document that declares nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest one declaration, replacing any earlier one of the same name.
    pub fn declare(&mut self, command: DeclaredCommand) {
        self.commands.insert(command.name.clone(), command);
    }

    /// The declaration for `name`, if this document declares it.
    ///
    /// Crate-internal on purpose (ruling R10's visibility half): the door
    /// onto a document's command surface is
    /// [`DocumentCommandSurface`], which answers the catalogue and the
    /// document together. A consumer that could reach the raw per-document
    /// table would be building the second lookup path ruling R1 rules out.
    #[must_use]
    pub(crate) fn get(&self, name: &str) -> Option<&DeclaredCommand> {
        self.commands.get(name)
    }

    /// Every declaration, in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &DeclaredCommand)> {
        self.commands
            .iter()
            .map(|(name, command)| (name.as_str(), command))
    }

    /// How many commands this document declares.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether this document declares nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// **The** command surface one document analyses against: the catalogue
/// generation it resolved, plus whatever that document declares for itself.
///
/// This is R1's "one query path serves all three of today's spec sources".
/// A consumer holds one of these and asks it once; it never holds a
/// registry and a second table and unions the two answers itself.
#[derive(Clone, Copy)]
pub struct DocumentCommandSurface<'a> {
    commands: &'a crate::registry::CommandRegistry,
    declared: Option<&'a DeclaredSurface>,
}

impl std::fmt::Debug for DocumentCommandSurface<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentCommandSurface")
            .field("declared", &self.declared.map_or(0, DeclaredSurface::len))
            .finish_non_exhaustive()
    }
}

impl<'a> DocumentCommandSurface<'a> {
    /// The surface of `commands` extended by `declared`.
    #[must_use]
    pub const fn new(
        commands: &'a crate::registry::CommandRegistry,
        declared: Option<&'a DeclaredSurface>,
    ) -> Self {
        Self { commands, declared }
    }

    /// The catalogue generation's command store.
    #[must_use]
    pub const fn commands(&self) -> &'a crate::registry::CommandRegistry {
        self.commands
    }

    /// The document's own declarations, when it has any.
    #[must_use]
    pub const fn declared(&self) -> Option<&'a DeclaredSurface> {
        self.declared
    }

    /// Whether this document declares `name` for itself.
    #[must_use]
    pub fn declares(&self, name: &str) -> bool {
        self.declared
            .is_some_and(|surface| surface.get(name).is_some())
    }

    /// Every name this document declares for itself, in name order.
    pub fn declared_names(&self) -> impl Iterator<Item = &'a str> {
        self.declared
            .into_iter()
            .flat_map(DeclaredSurface::iter)
            .map(|(name, _)| name)
    }

    /// Every command this document declares that a reader holding the
    /// catalogue alone may call by name, with the frame effect its
    /// declaration states ([`DeclaredCommand::plain_call_frame_effect`]), in
    /// name order.
    pub fn plain_call_frame_effects(
        &self,
    ) -> impl Iterator<Item = (&'a str, Option<FrameEffectSpec>)> {
        self.declared
            .into_iter()
            .flat_map(DeclaredSurface::iter)
            .filter_map(|(name, command)| Some((name, command.plain_call_frame_effect()?)))
    }

    /// The document's own declaration of `name`, when it has one — the row
    /// every nearest-wins answer below reads first.
    fn declaration(&self, name: &str) -> Option<&'a DeclaredCommand> {
        self.declared.and_then(|surface| surface.get(name))
    }

    /// The command-prefix positions of `name` over the whole surface, each
    /// with the arity it appends to the callback — nearest wins, as
    /// [`Self::arg_indices_for_role`].
    ///
    /// A declaration carries a position but no arity, so it answers
    /// [`AppendedArity::Unknown`] — the arity-inert default, which names the
    /// callback for reference and reachability consumers without asserting a
    /// count no declaration stated.
    #[must_use]
    pub fn command_prefixes(&self, name: &str, args: &[&str]) -> Vec<(usize, AppendedArity)> {
        match self.declaration(name) {
            Some(declared) => declared
                .arg_indices_for_role(ArgRole::CommandPrefix, args.len())
                .into_iter()
                .map(|index| (index, AppendedArity::Unknown))
                .collect(),
            None => self.commands.command_prefixes(name, args),
        }
    }

    /// The argument indices of `name` carrying `role`, over the whole
    /// surface — nearest wins.
    ///
    /// For a name the document declares, the declaration alone answers: a
    /// stub is a workspace-authored fact on the same footing as a shipped
    /// spec, so one that redeclares a catalogued command narrows its roles to
    /// what the author wrote rather than unioning with the catalogue's. Every
    /// other name is the catalogue's.
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, args: &[&str], role: ArgRole) -> Vec<usize> {
        match self.declaration(name) {
            Some(declared) => declared.arg_indices_for_role(role, args.len()),
            None => self.commands.arg_indices_for_role(name, args, role),
        }
    }

    /// The behavioural traits of `name` over the whole surface — nearest
    /// wins, under the security floor.
    ///
    /// A declaration answers with the traits it states, plus the security
    /// traits of the shipped command it redeclares, if any (invariant I6:
    /// the floor is a security contract, not a precision cap, so a stub can
    /// no more drop `exec`'s `UNSAFE` than a pack override can). Every other
    /// name answers the catalogue's command-level traits; `None` when
    /// neither the document nor the catalogue knows it.
    #[must_use]
    pub fn traits(&self, name: &str) -> Option<Traits> {
        let shipped = self.commands.get(name).map(|spec| spec.traits);
        match self.declaration(name) {
            Some(declared) => Some(
                declared
                    .traits
                    .union(shipped.map_or_else(Traits::empty, SecurityFloor::security_traits)),
            ),
            None => shipped,
        }
    }

    /// The traits one invocation of `name` carries — [`Self::traits`] for a
    /// declared command, which has no subcommands to refine them, and the
    /// catalogue's
    /// [`invocation_traits`](crate::registry::CommandRegistry::invocation_traits)
    /// (the command's traits unioned with the resolved subcommand's)
    /// otherwise.
    #[must_use]
    pub fn invocation_traits(
        &self,
        name: &str,
        args: &[&str],
        query: Option<SurfaceQuery<'_>>,
    ) -> Traits {
        if self.declaration(name).is_some() {
            return self.traits(name).unwrap_or_default();
        }
        self.commands.invocation_traits(name, args, query)
    }

    /// The side effects of `name` over the whole surface — nearest wins,
    /// under the security floor.
    ///
    /// A declaration answers with the effects it states, and a declaration
    /// that redeclares a shipped command keeps that command's effects too:
    /// the floor unions set-valued facts ([`SecurityFloor::apply`]). Every
    /// other name answers the catalogue's command-level effects; `None` when
    /// neither the document nor the catalogue knows it.
    #[must_use]
    pub fn side_effects(&self, name: &str) -> Option<Cow<'a, [SideEffect]>> {
        let shipped = self.commands.get(name).map(|spec| spec.side_effects);
        match self.declaration(name) {
            Some(declared) => Some(match shipped {
                Some(shipped) if !shipped.is_empty() => {
                    let mut effects = declared.side_effects.clone();
                    for effect in shipped {
                        if !effects.contains(effect) {
                            effects.push(*effect);
                        }
                    }
                    Cow::Owned(effects)
                }
                _ => Cow::Borrowed(declared.side_effects.as_slice()),
            }),
            None => shipped.map(Cow::Borrowed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::context::ContextQueries;
    use crate::model::ingress::static_context_for_profile;

    fn declared(name: &str, args: &[(&str, ArgRole)]) -> DeclaredCommand {
        declared_with_optionals(
            name,
            &args
                .iter()
                .map(|(argument, role)| (*argument, *role, false))
                .collect::<Vec<_>>(),
        )
    }

    fn declared_with_optionals(name: &str, args: &[(&str, ArgRole, bool)]) -> DeclaredCommand {
        DeclaredCommand::new(
            name.to_owned(),
            args.iter()
                .map(|(argument, role, optional)| DeclaredArgument {
                    name: (*argument).to_owned(),
                    role: *role,
                    optional: *optional,
                })
                .collect(),
            Provenance::Document,
        )
    }

    #[test]
    fn role_words_map_onto_the_registry_roles() {
        assert_eq!(role_for_word("body"), ArgRole::Body);
        assert_eq!(role_for_word("expr"), ArgRole::Expr);
        assert_eq!(role_for_word("var"), ArgRole::VarWrite);
        assert_eq!(role_for_word("var_read"), ArgRole::VarRead);
        assert_eq!(role_for_word("name"), ArgRole::Name);
        assert_eq!(role_for_word("pattern"), ArgRole::Pattern);
        assert_eq!(role_for_word("channel"), ArgRole::Channel);
        assert_eq!(role_for_word("command_prefix"), ArgRole::CommandPrefix);
        assert_eq!(role_for_word("value"), ArgRole::Value);
        assert_eq!(role_for_word("totally_made_up"), ArgRole::Value);
    }

    #[test]
    fn a_later_declaration_replaces_an_earlier_one() {
        let mut surface = DeclaredSurface::new();
        surface.declare(declared("redef", &[("a", ArgRole::Body)]));
        surface.declare(declared("redef", &[("a", ArgRole::Expr)]));
        assert_eq!(surface.len(), 1);
        assert_eq!(
            surface.get("redef").expect("declared").arguments[0].role,
            ArgRole::Expr
        );
    }

    #[test]
    fn declarations_iterate_in_name_order() {
        let mut surface = DeclaredSurface::new();
        for name in ["zeta", "alpha", "mu"] {
            surface.declare(declared(name, &[]));
        }
        let names: Vec<&str> = surface.iter().map(|(name, _)| name).collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn role_indices_come_from_the_declaration() {
        let command = declared(
            "with_var",
            &[
                ("varName", ArgRole::VarWrite),
                ("value", ArgRole::Value),
                ("body", ArgRole::Body),
            ],
        );
        assert_eq!(command.arg_indices_for_role(ArgRole::VarWrite, 3), vec![0]);
        assert_eq!(command.arg_indices_for_role(ArgRole::Body, 3), vec![2]);
        assert!(command.arg_indices_for_role(ArgRole::Expr, 3).is_empty());
    }

    /// An optional slot the call omits shifts every later role one position
    /// left: `{?table? row:var}` called as `fetch out` writes index 0.
    #[test]
    fn an_omitted_optional_shifts_the_roles_after_it() {
        let command = declared_with_optionals(
            "fetch",
            &[
                ("table", ArgRole::Value, true),
                ("row", ArgRole::VarWrite, false),
            ],
        );
        assert_eq!(command.arg_indices_for_role(ArgRole::VarWrite, 1), vec![0]);
        assert_eq!(command.arg_indices_for_role(ArgRole::VarWrite, 2), vec![1]);
    }

    /// Optional slots fill left to right, so only the leading ones are
    /// present in a call that supplies some but not all of them.
    #[test]
    fn optional_slots_fill_left_to_right() {
        let command = declared_with_optionals(
            "visit",
            &[
                ("first", ArgRole::Value, true),
                ("second", ArgRole::Value, true),
                ("script", ArgRole::Body, false),
            ],
        );
        assert_eq!(command.arg_indices_for_role(ArgRole::Body, 1), vec![0]);
        assert_eq!(command.arg_indices_for_role(ArgRole::Body, 2), vec![1]);
        assert_eq!(command.arg_indices_for_role(ArgRole::Body, 3), vec![2]);
    }

    /// A call the declaration cannot lay out — fewer words than it requires
    /// — maps to nothing rather than to positions the call does not have.
    #[test]
    fn a_call_shorter_than_the_declaration_maps_to_nothing() {
        let command = declared(
            "with_var",
            &[("varName", ArgRole::VarWrite), ("body", ArgRole::Body)],
        );
        assert!(command.arg_indices_for_role(ArgRole::Body, 1).is_empty());
    }

    /// The one door answers the catalogue for a shipped name and the
    /// document for a declared one, without the caller unioning anything.
    #[test]
    fn one_door_answers_catalogue_and_document() {
        let registry = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        let mut surface = DeclaredSurface::new();
        surface.declare(declared("my_eval", &[("script", ArgRole::Body)]));
        let view = DocumentCommandSurface::new(registry, Some(&surface));

        assert_eq!(
            view.arg_indices_for_role("my_eval", &["{...}"], ArgRole::Body),
            vec![0],
        );
        assert!(view.declares("my_eval"));
        assert!(!view.declares("while"));
        // `while cond body` — the catalogue's own answer, unchanged.
        assert_eq!(
            view.arg_indices_for_role("while", &["1", "{...}"], ArgRole::Body),
            registry.arg_indices_for_role("while", &["1", "{...}"], ArgRole::Body),
        );
    }

    /// A declaration that redeclares a catalogued name answers alone —
    /// nearest wins — so the catalogue's role the declaration omits is not
    /// assigned. `while cond body` puts its body at 1; a stub writing
    /// `{script:body cond}` puts it at 0, and 1 is a plain value.
    #[test]
    fn a_redeclared_name_answers_nearest_wins() {
        let registry = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        let args = ["{...}", "1"];
        assert_eq!(
            registry.arg_indices_for_role("while", &args, ArgRole::Body),
            vec![1]
        );
        let mut shadowing = DeclaredSurface::new();
        shadowing.declare(declared(
            "while",
            &[("script", ArgRole::Body), ("cond", ArgRole::Value)],
        ));
        let view = DocumentCommandSurface::new(registry, Some(&shadowing));
        assert_eq!(
            view.arg_indices_for_role("while", &args, ArgRole::Body),
            vec![0],
            "the declaration's role, and not the catalogue's as well"
        );
        assert!(
            view.arg_indices_for_role("while", &args, ArgRole::Expr)
                .is_empty(),
            "the catalogue's condition the declaration omits is not assigned"
        );
        assert!(view.command_prefixes("while", &args).is_empty());
    }

    /// A declaration's traits and side effects answer for the name it
    /// declares, and a redeclared shipped command keeps its security traits
    /// and effects beneath them (I6); an undeclared name is the catalogue's,
    /// and a name neither knows has no answer.
    #[test]
    fn declared_traits_and_effects_answer_under_the_security_floor() {
        use crate::side_effects::SideEffectTarget;
        let registry = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        let mutation = SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        };
        let mut surface = DeclaredSurface::new();
        surface.declare(
            declared("my_fold", &[("x", ArgRole::Value)])
                .with_traits(Traits::PURE)
                .with_side_effects(vec![mutation]),
        );
        surface.declare(declared("exec", &[("cmd", ArgRole::Value)]).with_traits(Traits::PURE));
        let view = DocumentCommandSurface::new(registry, Some(&surface));

        assert_eq!(view.traits("my_fold"), Some(Traits::PURE));
        assert_eq!(
            view.invocation_traits("my_fold", &["a"], None),
            Traits::PURE
        );
        assert_eq!(
            view.side_effects("my_fold").as_deref(),
            Some(&[mutation][..])
        );

        let shipped = registry.get("exec").expect("exec ships");
        let floor = SecurityFloor::security_traits(shipped.traits);
        assert!(floor.contains(Traits::UNSAFE), "exec's floor holds UNSAFE");
        let exec = view.traits("exec").expect("declared");
        assert!(exec.contains(Traits::PURE) && exec.contains(floor));
        assert!(
            shipped.side_effects.iter().all(|effect| view
                .side_effects("exec")
                .is_some_and(|effects| effects.contains(effect))),
            "the shipped effects stay beneath the declaration"
        );

        assert_eq!(
            view.traits("while"),
            registry.get("while").map(|spec| spec.traits)
        );
        assert_eq!(view.traits("no_such_command"), None);
        assert!(view.side_effects("no_such_command").is_none());
    }

    /// A surface with no declarations is exactly the catalogue.
    #[test]
    fn an_undeclaring_document_is_the_catalogue() {
        let registry = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        let view = DocumentCommandSurface::new(registry, None);
        assert!(view.declared_names().next().is_none());
        assert_eq!(
            view.arg_indices_for_role("while", &["1", "{...}"], ArgRole::Body),
            registry.arg_indices_for_role("while", &["1", "{...}"], ArgRole::Body),
        );
    }

    /// The module docs claim a document row is unconditional under the
    /// ordinary availability queries. Pin it against the real ones rather
    /// than asserting it in prose.
    #[test]
    fn declared_rows_are_available_under_the_ordinary_queries() {
        let generation = static_context_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        let context = generation.context();
        for provenance in [Provenance::Document, Provenance::WorkspaceUntrusted] {
            let command = DeclaredCommand::new("my_eval".to_owned(), Vec::new(), provenance);
            let rows = [command.declaration.clone()];
            assert!(context.provider_active(&rows[0].provider));
            assert!(context.predicate_passes(&rows[0].predicate));
            assert!(
                context.is_available(&rows),
                "a {provenance:?} document row must be unconditionally available",
            );
            assert!(context.admits_for_selection(&rows[0]));
        }
    }

    // ───────────── the extension default and what narrows it ─────────────

    fn extension(name: &str) -> DeclaredCommand {
        DeclaredCommand::extension(name.to_owned(), Vec::new(), Provenance::Document)
    }

    fn unknown_read_write() -> Vec<SideEffect> {
        extension_default::SIDE_EFFECTS.to_vec()
    }

    #[test]
    fn an_extension_declaration_is_the_registry_default_and_a_plain_one_is_empty() {
        let command = extension("pkga_calc");
        let default = CommandSpec::extension_default("pkga_calc");
        assert_eq!(command.traits, default.traits);
        assert_eq!(command.side_effects, default.side_effects);
        assert_eq!(command.provenance(), Provenance::Document);
        assert!(!command.traits.contains(Traits::PURE));

        let plain = declared("pkga_calc", &[]);
        assert!(plain.traits.is_empty());
        assert!(plain.side_effects.is_empty());
    }

    #[test]
    fn a_stated_purity_replaces_the_effect_axes_and_leaves_the_others() {
        let narrowed = extension("pure_ext").narrowed_by(Traits::PURE, Vec::new());
        assert!(narrowed.traits.contains(Traits::PURE));
        assert!(
            !narrowed.traits.intersects(extension_default::EFFECT_AXES),
            "a pure command evaluates no code and establishes no trace: {}",
            narrowed.traits
        );
        assert!(narrowed.side_effects.is_empty());
        for kept in [extension_default::TAINT, extension_default::SAFETY] {
            assert!(
                narrowed.traits.contains(kept),
                "purity says nothing of {kept}, so it stays: {}",
                narrowed.traits
            );
        }
    }

    #[test]
    fn a_stated_effect_replaces_the_unknown_one_and_the_barrier() {
        let variable = SideEffect {
            target: crate::side_effects::SideEffectTarget::Variable,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        };
        let narrowed = extension("mut_ext").narrowed_by(Traits::READS_BEFORE_WRITE, vec![variable]);
        assert_eq!(narrowed.side_effects, vec![variable]);
        assert!(!narrowed.traits.intersects(extension_default::EFFECT_AXES));
        assert!(narrowed.traits.contains(Traits::READS_BEFORE_WRITE));
        assert!(narrowed.traits.contains(extension_default::TAINT));
        assert!(narrowed.traits.contains(extension_default::SAFETY));
    }

    #[test]
    fn a_stated_barrier_survives_a_stated_effect() {
        let narrowed = extension("barrier_ext").narrowed_by(
            Traits::PURE.union(Traits::CREATES_DYNAMIC_BARRIER),
            Vec::new(),
        );
        assert!(narrowed.traits.contains(Traits::CREATES_DYNAMIC_BARRIER));
        assert!(
            !narrowed.traits.contains(Traits::EVALUATES_CODE),
            "only the barrier the declaration stated is added back"
        );
    }

    #[test]
    fn a_statement_that_is_not_about_effects_narrows_nothing() {
        let stated = Traits::HAS_LOOP_BODY
            .union(Traits::CREATES_SCOPE_ALIAS)
            .union(Traits::UNSAFE);
        let narrowed = extension("loop_ext").narrowed_by(stated, Vec::new());
        assert!(narrowed.traits.contains(extension_default::TRAITS));
        assert!(narrowed.traits.contains(stated));
        assert_eq!(narrowed.side_effects, unknown_read_write());
    }

    #[test]
    fn narrowing_a_plain_declaration_by_nothing_changes_nothing() {
        let plain = declared("plain", &[("a", ArgRole::Body)]);
        assert_eq!(
            plain.clone().narrowed_by(Traits::empty(), Vec::new()),
            plain
        );
    }

    #[test]
    fn the_frame_words_are_the_registrys_frame_effects() {
        assert_eq!(
            DeclaredFrameEffect::from_stub_word("own"),
            Some(DeclaredFrameEffect::Stated(None))
        );
        assert_eq!(
            DeclaredFrameEffect::from_stub_word("none"),
            Some(DeclaredFrameEffect::Stated(None))
        );
        assert_eq!(
            DeclaredFrameEffect::from_stub_word("caller"),
            Some(DeclaredFrameEffect::Stated(Some(FrameEffectSpec {
                level_word: FrameLevelWord::None,
                layout: FrameArgLayout::OpaqueCallerVars,
            })))
        );
        // `argparse`'s own effect is the vocabulary `caller` names.
        let registry = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        assert_eq!(
            registry.frame_effect("argparse"),
            Some(DeclaredFrameEffect::CALLER)
        );
        assert_eq!(DeclaredFrameEffect::from_stub_word("Own"), None);
        assert_eq!(DeclaredFrameEffect::from_stub_word(""), None);
        // In a flag run, the last `-frame` word wins and an unknown one, or
        // none, states nothing.
        assert_eq!(
            DeclaredFrameEffect::from_stub_flags("-pure -frame caller -frame own"),
            DeclaredFrameEffect::Stated(None)
        );
        assert_eq!(
            DeclaredFrameEffect::from_stub_flags("-frame caller -frame upvar"),
            DeclaredFrameEffect::Stated(Some(DeclaredFrameEffect::CALLER))
        );
        assert_eq!(
            DeclaredFrameEffect::from_stub_flags("-pure -frame"),
            DeclaredFrameEffect::Unstated
        );
        assert_eq!(
            DeclaredFrameEffect::from_stub_flags(""),
            DeclaredFrameEffect::Unstated
        );
        assert_eq!(
            declared("unstated", &[]).frame_effect,
            DeclaredFrameEffect::Unstated
        );
    }

    #[test]
    fn a_plain_call_states_its_frame_effect_and_data_alone() {
        let own =
            |command: DeclaredCommand| command.with_frame_effect(DeclaredFrameEffect::Stated(None));
        let data = [
            ("sql", ArgRole::Value),
            ("design", ArgRole::Name),
            ("glob", ArgRole::Pattern),
            ("chan", ArgRole::Channel),
        ];
        assert_eq!(
            own(declared("db_query", &data)).plain_call_frame_effect(),
            Some(None)
        );
        assert_eq!(
            declared("db_query", &data)
                .with_frame_effect(DeclaredFrameEffect::Stated(Some(
                    DeclaredFrameEffect::CALLER
                )))
                .plain_call_frame_effect(),
            Some(Some(DeclaredFrameEffect::CALLER))
        );
        assert_eq!(
            own(declared("pure_query", &data))
                .with_traits(Traits::PURE.union(Traits::UNSAFE))
                .plain_call_frame_effect(),
            Some(None)
        );
        // Unstated, it is not one.
        assert_eq!(declared("db_query", &data).plain_call_frame_effect(), None);
        // A word only a reader of the declaration knows to follow.
        for role in [
            ArgRole::Body,
            ArgRole::Expr,
            ArgRole::VarWrite,
            ArgRole::VarRead,
            ArgRole::CommandPrefix,
        ] {
            assert_eq!(
                own(declared("db_eval", &[("sql", ArgRole::Value), ("w", role)]))
                    .plain_call_frame_effect(),
                None,
                "{role:?}"
            );
        }
        // A fact only a reader of the declaration knows.
        for traits in [
            Traits::CREATES_DYNAMIC_BARRIER,
            Traits::HAS_LOOP_BODY,
            Traits::CREATES_SCOPE_ALIAS,
            Traits::READS_BEFORE_WRITE,
        ] {
            assert_eq!(
                own(declared("flagged", &data))
                    .with_traits(traits)
                    .plain_call_frame_effect(),
                None,
                "{traits:?}"
            );
        }
        assert_eq!(own(extension("ext_query")).plain_call_frame_effect(), None);
        assert_eq!(
            own(declared("mutator", &data))
                .with_side_effects(unknown_read_write())
                .plain_call_frame_effect(),
            None
        );
    }

    #[test]
    fn the_surface_lists_the_plain_calls_it_declares() {
        let registry = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        let mut surface = DeclaredSurface::new();
        surface.declare(
            declared("db_query", &[("sql", ArgRole::Value)])
                .with_frame_effect(DeclaredFrameEffect::Stated(None)),
        );
        surface.declare(
            declared("db_bind", &[("sql", ArgRole::Value)]).with_frame_effect(
                DeclaredFrameEffect::Stated(Some(DeclaredFrameEffect::CALLER)),
            ),
        );
        surface.declare(declared("db_close", &[("handle", ArgRole::Value)]));
        surface.declare(
            declared(
                "db_eval",
                &[("sql", ArgRole::Value), ("script", ArgRole::Body)],
            )
            .with_frame_effect(DeclaredFrameEffect::Stated(None)),
        );
        let document = DocumentCommandSurface::new(registry, Some(&surface));
        assert_eq!(
            document.plain_call_frame_effects().collect::<Vec<_>>(),
            vec![
                ("db_bind", Some(DeclaredFrameEffect::CALLER)),
                ("db_query", None),
            ]
        );
        assert_eq!(
            DocumentCommandSurface::new(registry, None)
                .plain_call_frame_effects()
                .count(),
            0
        );
    }
}
