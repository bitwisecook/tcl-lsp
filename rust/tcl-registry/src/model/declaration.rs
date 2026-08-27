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
//! (`docs/design/dialect-and-package-registry-centralisation.md` §4).
//!
//! A `# tcl-lsp: stub NAME {ARGS}` block in the analysed buffer, and a
//! workspace `<environment>.tcl.stubs` sidecar, both say the same kind of
//! thing the catalogue says: *this name is a command, and these are its
//! argument roles*. Until this module they said it in their own vocabulary
//! — a `StubOverlay` of `StubSig`/`StubArg`/`StubSigFlags` values that
//! every consumer had to consult **beside** the registry, with its own
//! role-word parser and its own `arg_indices_for_role` twin, and with no
//! provenance at all.
//!
//! Here they ingest as ordinary [`SurfaceDeclaration`]s:
//!
//! - the **provider** is [`Provider::Document`] — active exactly in the
//!   buffer that declared it, which is why such a declaration never joins
//!   a shared [`crate::model::ContextRegistry`] generation (a generation
//!   is keyed by environment, and this row's scope is one document);
//! - the **applicability** is the whole [document
//!   axis](tcl_dialect::model::VersionAxisId::document) — a buffer has no
//!   release train, so the declaration holds for as long as it is written;
//! - the **provenance** is [`Provenance::Document`] for an inline block
//!   and [`Provenance::WorkspaceUntrusted`] for a sidecar: the two lowest
//!   trust classes in §6.4's lattice, so a declaration may add assistance
//!   and can never weaken a shipped analysis fact;
//! - the **argument roles** are the registry's own [`ArgRole`], resolved
//!   by the registry's own role-word table ([`role_for_word`]), so a stub
//!   argument and a catalogue argument are the same kind of fact.
//!
//! [`DeclaredSurface`] is the per-document generation of those rows, and
//! [`DocumentCommandSurface`] is **the** door onto the command surface one
//! document analyses against: catalogue generation plus that document's own
//! declarations, asked once. No consumer consults the catalogue and then a
//! second table.
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

use std::collections::BTreeMap;

use tcl_dialect::model::{ItemHistory, Provenance, VersionAxisId, VersionSet};

use crate::arg_role::ArgRole;
use crate::model::surface::{CapabilityPredicate, Provider, SurfaceDeclaration};

/// Map a stub directive's role word (`body`, `expr`, `var`, `var_read`,
/// `name`, `pattern`, `channel`, `command_prefix`) to the registry's own
/// [`ArgRole`].
///
/// An unrecognised word is [`ArgRole::Value`] — the same "value is the
/// default" rule an argument with no `:role` annotation gets.
#[must_use]
pub fn role_for_word(word: &str) -> ArgRole {
    match word {
        "body" => ArgRole::Body,
        "expr" => ArgRole::Expr,
        "var" => ArgRole::VarWrite,
        "var_read" => ArgRole::VarRead,
        "name" => ArgRole::Name,
        "pattern" => ArgRole::Pattern,
        "channel" => ArgRole::Channel,
        "command_prefix" => ArgRole::CommandPrefix,
        _ => ArgRole::Value,
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
    /// The §4.1 surface row this declaration ingested as.
    pub declaration: SurfaceDeclaration,
}

impl DeclaredCommand {
    /// Declare `name` with `arguments`, provided by the document itself at
    /// `provenance`.
    ///
    /// `provenance` is the source's trust class —
    /// [`Provenance::Document`] for an inline `# tcl-lsp: stub` block,
    /// [`Provenance::WorkspaceUntrusted`] for a `.tcl.stubs` sidecar.
    #[must_use]
    pub fn new(name: String, arguments: Vec<DeclaredArgument>, provenance: Provenance) -> Self {
        Self {
            name,
            arguments,
            declaration: SurfaceDeclaration {
                provider: Provider::Document,
                applicable: whole_document_axis(),
                predicate: CapabilityPredicate::None,
                history: ItemHistory::default(),
                provenance,
            },
        }
    }

    /// The trust class of the source that declared this command.
    #[must_use]
    pub const fn provenance(&self) -> Provenance {
        self.declaration.provenance
    }

    /// The 0-based argument indices (against the post-head argument list)
    /// whose declared role is `role`.
    pub fn arg_indices_for_role(&self, role: ArgRole) -> impl Iterator<Item = usize> + '_ {
        self.arguments
            .iter()
            .enumerate()
            .filter(move |(_, argument)| argument.role == role)
            .map(|(index, _)| index)
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
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&DeclaredCommand> {
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

    /// The argument indices of `name` carrying `role`, over the whole
    /// surface.
    ///
    /// The document's declaration **adds to** the catalogue's answer, it
    /// does not replace it. That is §6.4's rule for the lowest trust
    /// classes read literally: a document or untrusted-workspace
    /// declaration "may improve assistance, never weaken shipped analysis
    /// facts", so a stub that happens to shadow a shipped name can add a
    /// role position but can never take one away.
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, args: &[&str], role: ArgRole) -> Vec<usize> {
        let mut indices = self.commands.arg_indices_for_role(name, args, role);
        if let Some(declared) = self.declared.and_then(|surface| surface.get(name)) {
            for index in declared.arg_indices_for_role(role) {
                if !indices.contains(&index) {
                    indices.push(index);
                }
            }
        }
        indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::context::ContextQueries;
    use crate::model::ingress::static_context_for_profile;

    fn declared(name: &str, args: &[(&str, ArgRole)]) -> DeclaredCommand {
        DeclaredCommand::new(
            name.to_owned(),
            args.iter()
                .map(|(argument, role)| DeclaredArgument {
                    name: (*argument).to_owned(),
                    role: *role,
                    optional: false,
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
        assert_eq!(
            command
                .arg_indices_for_role(ArgRole::VarWrite)
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(
            command
                .arg_indices_for_role(ArgRole::Body)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert!(command.arg_indices_for_role(ArgRole::Expr).next().is_none());
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
        // A declaration shadowing a shipped name adds to the catalogue's
        // role answer and never removes one (§6.4's untrusted-tier rule).
        let mut shadowing = DeclaredSurface::new();
        shadowing.declare(declared("while", &[("script", ArgRole::Body)]));
        let shadow_view = DocumentCommandSurface::new(registry, Some(&shadowing));
        let shipped = registry.arg_indices_for_role("while", &["1", "{...}"], ArgRole::Body);
        let widened = shadow_view.arg_indices_for_role("while", &["1", "{...}"], ArgRole::Body);
        assert!(shipped.iter().all(|index| widened.contains(index)));
        assert!(widened.contains(&0));
        assert!(view.declares("my_eval"));
        assert!(!view.declares("while"));
        // `while cond body` — the catalogue's own answer, unchanged.
        assert_eq!(
            view.arg_indices_for_role("while", &["1", "{...}"], ArgRole::Body),
            registry.arg_indices_for_role("while", &["1", "{...}"], ArgRole::Body),
        );
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
}
