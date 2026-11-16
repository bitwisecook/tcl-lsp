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

//! Class factories: the manufacturer layout a metaclass publishes, deferred creations and the workspace indices built from them.

use std::collections::BTreeMap;
use tcl_lexer::Token;

/// One word of a definition-body member a class factory splices into every
/// class it manufactures, as a **template** over the creation call's own
/// arguments.
///
/// The template is call-site independent, which is the whole point: it is
/// derived once, where the metaclass is written, and then resolved against
/// each `Meta create …` call — including one in a different file, which is
/// the only way the per-file walk can classify such a call at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FactoryWord {
    /// A word the manufacturer writes literally in its own body
    /// (`superclass ::tk::MegawidgetClass`), with the token it occupies
    /// **in the metaclass's own document**.
    Literal {
        /// The word's text.
        text: String,
        /// Its token in the defining document — usable only while resolving
        /// a call in that same document (see
        /// [`ClassFactory::resolve_in_other_document`]).
        token: Token,
    },
    /// A `{*}$param` splice of the creation call's argument at this index
    /// (argument 0 being the manufacturer subcommand itself), so the words
    /// it contributes are the caller's own and carry the caller's tokens.
    CallerSplice(usize),
}

/// One definition-body member a class factory injects, as a template.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FactoryMember {
    /// Member keyword followed by its argument words.
    pub words: Vec<FactoryWord>,
}

/// The word layout one manufacturer subcommand of a class factory imposes,
/// plus what it splices into every body it makes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManufacturerSpec {
    /// Argument index of the new class's name.
    pub name_arg: usize,
    /// Argument index of the new class's definition body.
    pub body_arg: usize,
    /// The members the manufacturer always injects, or `None` when its
    /// prologue could not be read — the
    /// [`ClassDef::inheritance_unknown`] case.
    pub injected: Option<Vec<FactoryMember>>,
}

/// Workspace-wide class factories, keyed by fully-qualified metaclass name —
/// what a host hands the analyser so a `Meta create …` call can be classified
/// when `Meta` is written in another document.
pub type ClassFactoryIndex = BTreeMap<String, ClassFactory>;

/// A creation call the walk could not classify, kept for a **second verdict**
/// once the parameterised-class observation join has settled.
///
/// A metaclass proved only by that post-pass does not exist while the walk
/// runs: `::T::D::class create ::T::W { … }` in the same document reads as a
/// call to an unknown command, so the class it makes is recorded nowhere.
/// Which verdict the head deserves is therefore not answerable during the
/// walk — the same situation version-floor arity resolution meets, and the
/// same answer: buffer the inputs, decide afterwards.
///
/// Only calls whose head names **nothing this document knows** are kept, which
/// is the one shape a later proof can turn into a creation; a head that
/// already resolves has had its verdict, right or wrong, and is not revisited.
#[derive(Debug, Clone)]
pub struct DeferredClassCreation {
    /// The call's head word, exactly as `handle_oo_class_command` received it.
    pub cmd_name: String,
    /// Its argument words.
    pub args: Vec<String>,
    /// Their tokens, for the name and body spans the replay records.
    pub arg_tokens: Vec<Token>,
    /// The scope the call was written in, so the replayed class homes where
    /// the walk would have homed it.
    pub scope_path: Vec<usize>,
    /// The head's own token — the call offset a rename chain resolves at.
    pub cmd_tok: Token,
}

/// Instance methods dispatchable on some workspace **descendant** of each
/// class, keyed by the ancestor's fully-qualified name — what a host hands
/// the analyser so the template-method abstention can see a
/// subclass written in another document.
///
/// `map["::Formatter"]` holds every instance-side member name reachable on
/// any workspace class that lists `::Formatter` anywhere in its
/// linearisation — including names the descendant inherits from a mixin,
/// and excluding `private` members, which `TclOO` hides even from a base
/// class's own `my` dispatch.  `BTreeMap`/`BTreeSet` so equal contents
/// compare equal regardless of build order (the host's compare-then-set
/// depends on it).
pub type SubclassProvidedMethods = BTreeMap<String, std::collections::BTreeSet<String>>;

/// How a user-defined `TclOO` metaclass manufactures classes.
///
/// Recorded on the metaclass's own [`ClassDef`] when it is written, so a
/// `Meta create Name …` call anywhere — same file or, through the workspace
/// factory index, another one — is classified from a fact that was *proved*
/// at the definition rather than guessed from the call's shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ClassFactory {
    /// The registry metaclass command at the root of this factory's
    /// superclass chain (`oo::class`), whose definition-body grammar governs
    /// every body the factory makes.
    pub root_metaclass: String,
    /// Per manufacturer subcommand (`create` / `new` / …) word layout, for
    /// the subcommands the factory **overrides**.  A subcommand with no entry
    /// runs the inherited manufacturer, i.e. the builtin `create Name Body`
    /// layout with nothing injected.
    pub overrides: BTreeMap<String, ManufacturerSpec>,
    /// Manufacturer methods reachable through ordinary class-command
    /// dispatch after applying the metaclass object's `self export` and
    /// `self unexport` changes to the registry defaults.
    pub exported_manufacturers: std::collections::BTreeSet<String>,
    /// Whether calling one of the classes this factory makes with a **bare
    /// unrecognised word** both constructs an object and returns that word —
    /// Tk's `::tk::IconList .il` idiom.
    ///
    /// Proved where the metaclass is written, from its unrecognised-word
    /// fallback member (`TclOO`'s `unknown`), and `false` whenever the proof
    /// does not go through: no such member, a member that constructs but
    /// returns something else, or one that returns the word without
    /// constructing anything. See
    /// `Analyser::unknown_dispatch_binds_instance` for the exact rule and its
    /// residuals.
    ///
    /// A consumer reads this to decide whether `set w [Widget .w]` binds `w`
    /// to an instance of `Widget`; `false` means "not proved", so the handle
    /// stays untyped rather than being guessed.
    pub unknown_binds_instance: bool,
}

impl ClassFactory {
    /// The same factory with every literal word's token replaced by
    /// `elsewhere` — what a **cross-document** consumer must use, because the
    /// stored tokens index the metaclass's own document, not the caller's.
    ///
    /// A member whose registry spec *retracts* (`deletemethod` /
    /// `renamemethod`) reads its argument tokens for real, so an injection
    /// containing one cannot be resolved against a foreign document at all:
    /// the whole injection collapses to `None` (inheritance unknown) rather
    /// than being applied against a substituted span.  `retracts` is supplied
    /// by the caller, which owns the registry lookup.
    #[must_use]
    pub fn resolve_in_other_document(
        &self,
        elsewhere: Token,
        retracts: &dyn Fn(&str) -> bool,
    ) -> Self {
        let overrides = self
            .overrides
            .iter()
            .map(|(sub, spec)| {
                let injected = spec.injected.as_ref().and_then(|members| {
                    members
                        .iter()
                        .map(|m| rehome_member(m, elsewhere, retracts))
                        .collect::<Option<Vec<_>>>()
                });
                (sub.clone(), ManufacturerSpec { injected, ..*spec })
            })
            .collect();
        Self {
            root_metaclass: self.root_metaclass.clone(),
            overrides,
            exported_manufacturers: self.exported_manufacturers.clone(),
            // A property of the metaclass's own body, carrying no token, so
            // it crosses a document boundary unchanged.
            unknown_binds_instance: self.unknown_binds_instance,
        }
    }
}

/// One injected member's words re-homed onto `elsewhere`, or `None` when the
/// member's own effect reads the tokens it would lose.
fn rehome_member(
    member: &FactoryMember,
    elsewhere: Token,
    retracts: &dyn Fn(&str) -> bool,
) -> Option<FactoryMember> {
    let keyword = match member.words.first() {
        Some(FactoryWord::Literal { text, .. }) => text.as_str(),
        // A member whose *keyword* is spliced from the caller is not a shape
        // the prologue reader produces, and cannot be classified here.
        _ => return None,
    };
    if retracts(keyword) {
        return None;
    }
    Some(FactoryMember {
        words: member
            .words
            .iter()
            .map(|w| match w {
                FactoryWord::Literal { text, .. } => FactoryWord::Literal {
                    text: text.clone(),
                    token: elsewhere,
                },
                splice @ FactoryWord::CallerSplice(_) => splice.clone(),
            })
            .collect(),
    })
}
