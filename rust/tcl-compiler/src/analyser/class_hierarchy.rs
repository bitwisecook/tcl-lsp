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

//! Class Hierarchy Analysis (`CHA`) for `TclOO`.
//!
//! Builds a complete class hierarchy from the analyser's class
//! index, computes the MRO (two-pass DFS with late-placement
//! deduplication, see [`super::mro::tcloo_linearise`]) for each
//! class, and answers queries about subtype relationships,
//! method providers, and method resolution.
//!
//! Inspired by the CHA techniques used in LLVM and JVM `HotSpot`
//! for devirtualisation and call graph construction.

/// Original byte-joined relation order for source declaration assistance.
pub mod original_metadata;

use std::collections::{HashMap, HashSet};

use super::mro::{build_mro_map, tcloo_linearise};
use super::types::{ClassDef, MethodDef};

/// The name a `constructor` body carries wherever a member is identified by
/// name — `ClassDef::constructors`' own `MethodDef::name`, the tail of the
/// `Method` scope `walk_method_body` opens for it, and the label C Tcl's
/// `info object call` / `self method` report for that frame (tclsh 9.0.4).
pub const CONSTRUCTOR_MEMBER: &str = "<constructor>";

/// [`CONSTRUCTOR_MEMBER`]'s twin for a `destructor` body.
pub const DESTRUCTOR_MEMBER: &str = "<destructor>";

/// Source-selected member axis, independent of runtime receiver or dispatch.
/// Ordinary names never stand for a nameless lifecycle slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMemberKind {
    /// Actual ordinary member name and its own definition table.
    Method {
        /// Retained source name value, not a lifecycle display label.
        name: String,
        /// Instance versus class-object definition table.
        side: super::types::MemberSide,
    },
    /// Native source naming units retain their genuine original producer and
    /// policy; they are not decoded as Unicode report text.
    NativeMethod {
        /// Actual canonical declared name input, without a new byte recipe.
        input: Box<crate::signature_scan::scope::SignatureSourceNameInput>,
        /// Actual definition receiver table, independent of receiver entry.
        side: super::types::MemberSide,
    },
    /// Independently selected Registry lifecycle declaration role.
    Special(tcl_registry::definer::DefinitionSpecialMemberKind),
}

/// Read a typed source member slot from the selected class snapshot.
/// This supplies reporting metadata, without entered receiver or MRO authority.
#[must_use]
pub fn class_member_def(class_def: &ClassDef, member: &SourceMemberKind) -> Option<MethodDef> {
    use super::types::MemberSide;
    use tcl_registry::definer::DefinitionSpecialMemberKind;
    match member {
        SourceMemberKind::Special(DefinitionSpecialMemberKind::Constructor) => {
            class_def.constructors.last().cloned()
        }
        SourceMemberKind::Special(DefinitionSpecialMemberKind::Destructor) => {
            class_def.destructor.clone()
        }
        SourceMemberKind::Method {
            name,
            side: MemberSide::Instance,
        } => class_def.methods.get(name).cloned(),
        SourceMemberKind::Method {
            name,
            side: MemberSide::ClassObject,
        } => class_def.class_methods.get(name).cloned(),
        SourceMemberKind::NativeMethod { input, side } => {
            native_source_member_def(class_def, *side, input)?.into_option()
        }
    }
}

enum SourceMethodPresence<T> {
    Absent,
    Present(T),
}

impl<T> SourceMethodPresence<T> {
    fn from_option(method: Option<T>) -> Self {
        match method {
            Some(method) => Self::Present(method),
            None => Self::Absent,
        }
    }

    fn into_option(self) -> Option<T> {
        match self {
            Self::Absent => None,
            Self::Present(method) => Some(method),
        }
    }
}

fn native_source_member_def(
    class: &ClassDef,
    side: super::types::MemberSide,
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
) -> Option<SourceMethodPresence<MethodDef>> {
    // Missing source inventory is not absence and cannot skip a provider.
    let _complete = class.original_members.methods(side)?;
    Some(SourceMethodPresence::from_option(
        class
            .original_members
            .method_for_input(side, input)
            .map(|method| method.metadata().clone()),
    ))
}

/// Reborrow the actual source member enclosing this current occurrence.
/// Conditional Logical syntax remains source advice, never an entered frame.
#[must_use]
pub fn original_source_member_at(
    analysis: &super::types::AnalysisResult,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    cursor: u32,
) -> Option<(String, SourceMemberKind, tcl_lexer::Span)> {
    let input = analysis.resolved_input.as_ref()?;
    if input.lexer_config() != config || !analysis.matches_original_source_image(image, config) {
        return None;
    }
    if analysis.allows_retained_logical_declaration_advice() {
        if let Some(member) = analysis.logical_source_member_frame_at(image, config, cursor) {
            let word = member.original_name_word()?;
            let name = String::from_utf8(
                tcl_syntax::word_rules::original_static_word_unicode_value(word)?,
            )
            .ok()?;
            let body = member.original_body_word()?.span();
            return Some((
                member.class().metadata().qualified_name.clone(),
                SourceMemberKind::Method {
                    name,
                    side: member.side(),
                },
                body,
            ));
        }
        return original_logical_special_member_at(analysis, image, config, cursor);
    }
    let mut pending = vec![&analysis.global_scope];
    let mut candidates = Vec::new();
    while let Some(scope) = pending.pop() {
        pending.extend(&scope.children);
        if let Some(candidate) = original_scope_member(analysis, image, config, cursor, scope) {
            candidates.push(candidate);
        }
    }
    unique_innermost_source_member(candidates)
}

fn original_scope_member(
    analysis: &super::types::AnalysisResult,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    cursor: u32,
    scope: &super::types::Scope,
) -> Option<(String, SourceMemberKind, tcl_lexer::Span)> {
    let input = analysis.resolved_input.as_ref()?;
    let (class, body, member) = if let Some(context) = scope.original_member_context.as_deref() {
        if context.resolved_input() != input || !context.matches_source(image, config) {
            return None;
        }
        let member = if let Some(method) = context.ordinary_method() {
            SourceMemberKind::NativeMethod {
                input: Box::new(method.declaration().original_name_input().clone()),
                side: context.side(),
            }
        } else {
            SourceMemberKind::Special(context.special_member()?.kind())
        };
        (context.source_class(analysis)?, context.body_word(), member)
    } else {
        let context = scope.original_receiver_body_declaration.as_deref()?;
        if context.resolved_input() != input || !context.matches_source(image, input) {
            return None;
        }
        let mut classes = analysis.original_class_declarations().filter(|class| {
            class
                .metadata()
                .original_members
                .declarations()
                .any(|method| method == context.method())
        });
        let class = classes.next()?;
        if classes.next().is_some() {
            return None;
        }
        let member = SourceMemberKind::NativeMethod {
            input: Box::new(context.method().declaration().original_name_input().clone()),
            side: context.side(),
        };
        (class, context.body_word(), member)
    };
    let content = body.content_span().ok()?;
    (content.start() <= cursor && cursor <= content.end())
        .then(|| (class.metadata().qualified_name.clone(), member, body.span()))
}

fn original_logical_special_member_at(
    analysis: &super::types::AnalysisResult,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    cursor: u32,
) -> Option<(String, SourceMemberKind, tcl_lexer::Span)> {
    let input = analysis.resolved_input.as_ref()?;
    let mut candidates = Vec::new();
    for class in analysis.all_classes.values() {
        let Some(owner) = analysis.logical_source_class_declaration(class) else {
            continue;
        };
        if !owner.matches_source(image, input) {
            continue;
        }
        for member in class.original_special_members.declarations() {
            let word = member.body_word();
            if word.image() != image || word.config() != config || word.group().expand {
                continue;
            }
            let body = word.content_span().ok()?;
            if body.start() <= cursor && cursor <= body.end() {
                candidates.push((
                    owner.metadata().qualified_name.clone(),
                    SourceMemberKind::Special(member.kind()),
                    word.span(),
                ));
            }
        }
    }
    unique_innermost_source_member(candidates)
}

fn unique_innermost_source_member(
    mut candidates: Vec<(String, SourceMemberKind, tcl_lexer::Span)>,
) -> Option<(String, SourceMemberKind, tcl_lexer::Span)> {
    candidates.sort_by_key(|(_, _, span)| span.len());
    let mut candidates = candidates.into_iter();
    let selected = candidates.next()?;
    if candidates
        .next()
        .is_some_and(|(_, _, span)| span.len() == selected.2.len())
    {
        return None;
    }
    Some(selected)
}

/// Immutable snapshot of the complete class hierarchy.
///
/// Built once via [`build_class_hierarchy`] and queried via
/// [`Self::is_subtype`] / [`Self::method_target`] /
/// [`Self::all_implementations`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassHierarchy {
    /// All known classes, keyed by qualified name.
    pub classes: HashMap<String, ClassDef>,
    /// Class → linearised MRO (including self).
    pub mro_map: HashMap<String, Vec<String>>,
    /// Original relation lookups behind unresolved hierarchy rows.
    /// Conflicting rows retain no lookup; report text never restores one.
    original_unresolved_relations:
        HashMap<String, Option<crate::signature_scan::scope::SignatureSourceLookup>>,
    /// Class → its owner-resolved **direct** `mixin` edges, in declaration
    /// order — the same resolution [`Self::mro_map`] linearises through, kept
    /// so a consumer can tell which *branch* of the call chain a provider was
    /// reached on without re-resolving the names itself.
    ///
    /// C walks each mixin branch with a fresh copy of the dispatch flags, so
    /// the branch a provider sits on is what decides its export state (see
    /// [`Self::spine_map`]).
    pub mixin_map: HashMap<String, Vec<String>>,
    /// Class → its **mixin-free** linearisation (including self) — the
    /// `superclass` spine alone, in the order `TclOO`'s call-chain builder
    /// walks it.
    ///
    /// Separate from [`Self::mro_map`] because C's
    /// `AddSimpleClassChainToCallContext` treats the two paths differently:
    /// each mixin is entered with a *fresh copy* of the dispatch flags, so a
    /// mixin that unexports a name empties only its own branch, whereas the
    /// same word on the spine decides the whole dispatch.  Oracle,
    /// byte-identical on tclsh 8.6.16 and 9.0.4:
    ///
    /// ```tcl
    /// oo::class create A   { method m {} { return A-m } }
    /// oo::class create Mix { method m {} { return Mix-m } }
    /// oo::define Mix { unexport m }
    /// oo::class create D { superclass A ; mixin Mix }
    /// [D new] m          ;# -> A-m  (the mixin's unexport did not suppress A's m)
    /// oo::class create B { superclass A }
    /// oo::define B { unexport m }
    /// [B new] m          ;# -> unknown method "m"  (the spine's unexport did)
    /// ```
    ///
    /// Built by the same linearisation owner as `mro_map`, over the same
    /// resolved superclass edges, so the two orders can never disagree about
    /// which qualified name a bare `superclass Device` meant.
    pub spine_map: HashMap<String, Vec<String>>,
    /// Class → direct subclasses.
    pub subclasses: HashMap<String, HashSet<String>>,
    /// Class → all descendants (transitive closure).
    pub transitive_subtypes: HashMap<String, HashSet<String>>,
    /// `(class, method_name)` → qualified name of the class
    /// that provides the method in the MRO chain.  Used for
    /// ``next`` / ``nextto`` resolution and devirtualisation.
    pub method_providers: HashMap<(String, String), String>,
    /// Hierarchy errors (cycles, inconsistent linearisation).
    pub errors: Vec<super::MroError>,
}

impl ClassHierarchy {
    pub(super) fn original_unresolved_relation(
        &self,
        row: &str,
    ) -> Option<&crate::signature_scan::scope::SignatureSourceLookup> {
        self.original_unresolved_relations.get(row)?.as_ref()
    }

    /// Return `true` when `child` is a subtype of `parent`
    /// (transitively, via the linearised MRO).
    #[must_use]
    pub fn is_subtype(&self, child: &str, parent: &str) -> bool {
        self.mro_map
            .get(child)
            .is_some_and(|mro| mro.iter().any(|c| c == parent))
    }

    /// Exact reported declaration on one member side, without inherited
    /// lookup. Lexical compatibility consumers must independently select that
    /// advice mode; this query supplies no original or runtime member identity.
    #[must_use]
    pub fn declared_member(
        &self,
        class: &str,
        name: &str,
        side: super::types::MemberSide,
    ) -> Option<(&ClassDef, &MethodDef)> {
        let class = self.classes.get(class)?;
        let method = match side {
            super::types::MemberSide::Instance => class.methods.get(name),
            super::types::MemberSide::ClassObject => class.class_methods.get(name),
        }?;
        Some((class, method))
    }

    /// Resolve which class provides `method_name` for
    /// `class_name`, walking the MRO chain.  Returns the
    /// qualified name of the providing class, or `None` if no
    /// class on the chain implements the method.
    #[must_use]
    pub fn method_target(&self, class_name: &str, method_name: &str) -> Option<&str> {
        self.method_providers
            .get(&(class_name.to_string(), method_name.to_string()))
            .map(String::as_str)
    }

    /// Resolve which class provides an *explicit* constructor for
    /// `class_name`, walking the MRO chain — mirrors [`Self::method_target`],
    /// but for `TclOO`'s single per-class constructor slot
    /// (`ClassDef::constructors`) rather than a named method.
    ///
    /// Returns `None` when no class on the chain declares an *effective*
    /// one: `TclOO` falls back to a permissive default constructor
    /// (inherited from `oo::object`) that accepts and ignores any number
    /// of arguments both when no `constructor` was ever written
    /// (confirmed against tclsh 9.0.4: `oo::class create Foo {}` then `Foo
    /// new 1 2 3` succeeds) **and** when the effective (most recent)
    /// `constructor` declaration has a literally empty body — `TclOO`
    /// treats `constructor {a b} {}` as if no constructor were declared at
    /// all (confirmed against tclsh 9.0.4: `info class constructor Foo`
    /// returns empty, and `Foo new`/`Foo new 1 2 3` both succeed for that
    /// class; a body with any content at all — even a single space or a
    /// comment — *is* a real, arity-enforcing constructor). `source` is the
    /// full document text `body_span` indexes into (see
    /// [`is_empty_method_body`]) — this hierarchy holds no source text of
    /// its own. Callers must treat `None` as "any argument count is
    /// valid", not "the constructor takes no arguments".
    #[must_use]
    pub fn constructor_provider(&self, class_name: &str, source: &str) -> Option<&str> {
        let mro = self.mro_map.get(class_name)?;
        mro.iter().find_map(|ancestor| {
            self.classes
                .get(ancestor)
                .filter(|cd| {
                    cd.constructors
                        .last()
                        .is_some_and(|c| !is_empty_method_body(source, c.body_span))
                })
                .map(|_| ancestor.as_str())
        })
    }

    /// Resolve `TclOO` `next` / `nextto`: the class *after* the current
    /// provider in `class`'s MRO that provides `method`.
    ///
    /// For plain `next`, pass the class currently servicing the method as
    /// `current` and `start_from = None`; the search begins one past
    /// `current`.  For `nextto SomeClass`, pass `start_from =
    /// Some("::SomeClass")`; the search begins *at* that class.  Returns the
    /// qualified name of the next providing class, or `None` when the chain
    /// is exhausted (a `next` past the last provider — a runtime error in
    /// Tcl — surfaces here as "no next").
    #[must_use]
    pub fn next_provider(
        &self,
        class: &str,
        method: &str,
        current: &str,
        start_from: Option<&str>,
    ) -> Option<&str> {
        let mro = self.mro_map.get(class)?;
        let anchor = start_from.unwrap_or(current);
        let anchor_pos = mro.iter().position(|c| c == anchor)?;
        let scan_from = if start_from.is_some() {
            anchor_pos
        } else {
            anchor_pos + 1
        };
        mro.iter().skip(scan_from).find_map(|c| {
            self.classes
                .get(c)
                .filter(|cd| {
                    cd.methods.contains_key(method) || cd.class_methods.contains_key(method)
                })
                .map(|_| c.as_str())
        })
    }

    /// Resolve `TclOO` `next` / `nextto` from *within* `class`'s own
    /// constructor: the class after `class` in its MRO that provides an
    /// *effective* constructor (mirrors [`Self::next_provider`], but for the
    /// unnamed constructor slot rather than a named method — "effective"
    /// carries the same empty-body caveat as [`Self::constructor_provider`],
    /// whose doc explains it).
    ///
    /// For plain `next`, pass `start_from = None`; the search begins one
    /// past `class` itself. For `nextto SomeClass`, pass `start_from =
    /// Some("::SomeClass")`; the search begins *at* that class. `source` is
    /// the document text `body_span`s index into. Returns `None` when the
    /// chain is exhausted or `class` is unknown.
    #[must_use]
    pub fn constructor_next_provider(
        &self,
        class: &str,
        start_from: Option<&str>,
        source: &str,
    ) -> Option<&str> {
        let mro = self.mro_map.get(class)?;
        let anchor = start_from.unwrap_or(class);
        let anchor_pos = mro.iter().position(|c| c == anchor)?;
        let scan_from = if start_from.is_some() {
            anchor_pos
        } else {
            anchor_pos + 1
        };
        mro.iter().skip(scan_from).find_map(|c| {
            self.classes
                .get(c)
                .filter(|cd| {
                    cd.constructors
                        .last()
                        .is_some_and(|ctor| !is_empty_method_body(source, ctor.body_span))
                })
                .map(|_| c.as_str())
        })
    }

    /// Resolve `TclOO` `next` / `nextto` from *within* `class`'s own
    /// destructor: the class after `class` in its MRO that declares a
    /// destructor.
    ///
    /// Unlike [`Self::constructor_next_provider`], a destructor's
    /// "effective" test is plain existence (`ClassDef::destructor.is_some()`)
    /// — `TclOO`'s empty-body-elides-the-constructor quirk is verified
    /// (against tclsh 9.0.4) for constructors specifically; no equivalent
    /// claim has been checked for destructors, so an explicitly empty
    /// destructor body is conservatively still treated as a real override
    /// here rather than assumed to share the constructor's special case.
    ///
    /// Same `start_from` convention as [`Self::constructor_next_provider`].
    /// Returns `None` when the chain is exhausted or `class` is unknown.
    #[must_use]
    pub fn destructor_next_provider(&self, class: &str, start_from: Option<&str>) -> Option<&str> {
        let mro = self.mro_map.get(class)?;
        let anchor = start_from.unwrap_or(class);
        let anchor_pos = mro.iter().position(|c| c == anchor)?;
        let scan_from = if start_from.is_some() {
            anchor_pos
        } else {
            anchor_pos + 1
        };
        mro.iter().skip(scan_from).find_map(|c| {
            self.classes
                .get(c)
                .filter(|cd| cd.destructor.is_some())
                .map(|_| c.as_str())
        })
    }

    /// Resolve a source-selected typed member slot through the retained MRO.
    /// Lifecycle roles are never reconstructed from ordinary method names.
    #[must_use]
    pub fn member_next_provider(
        &self,
        class: &str,
        member: &SourceMemberKind,
        current: &str,
        start_from: Option<&str>,
        source: &str,
    ) -> Option<&str> {
        use tcl_registry::definer::DefinitionSpecialMemberKind;
        match member {
            SourceMemberKind::Special(DefinitionSpecialMemberKind::Constructor) => {
                self.constructor_next_provider(class, start_from, source)
            }
            SourceMemberKind::Special(DefinitionSpecialMemberKind::Destructor) => {
                self.destructor_next_provider(class, start_from)
            }
            SourceMemberKind::Method { .. } | SourceMemberKind::NativeMethod { .. } => {
                let mro = self.mro_map.get(class)?;
                let anchor = start_from.unwrap_or(current);
                let position = mro.iter().position(|name| name == anchor)?;
                let start = if start_from.is_some() {
                    position
                } else {
                    position + 1
                };
                for name in mro.iter().skip(start) {
                    let Some(class) = self.classes.get(name) else {
                        if matches!(member, SourceMemberKind::NativeMethod { .. }) {
                            return None;
                        }
                        continue;
                    };
                    let selected = match member {
                        SourceMemberKind::NativeMethod { input, side } => {
                            native_source_member_def(class, *side, input)?.into_option()
                        }
                        _ => class_member_def(class, member),
                    };
                    if selected.is_some() {
                        return Some(name.as_str());
                    }
                }
                None
            }
        }
    }

    /// Return all `(class, defining_class)` pairs that
    /// implement `method_name`.  Order matches the iteration
    /// order of `method_providers` (`HashMap` — non-deterministic
    /// across runs; sort externally if determinism matters).
    #[must_use]
    pub fn all_implementations(&self, method_name: &str) -> Vec<(String, String)> {
        self.method_providers
            .iter()
            .filter(|((_cls, meth), _provider)| meth == method_name)
            .map(|((cls, _meth), provider)| (cls.clone(), provider.clone()))
            .collect()
    }

    /// The linearised MRO of `class_name` (self first), or a single-element
    /// `[class_name]` when the hierarchy has no entry for it — an external or
    /// otherwise unindexed class is still itself.
    #[must_use]
    pub fn mro_or_self(&self, class_name: &str) -> Vec<String> {
        self.mro_map
            .get(class_name)
            .cloned()
            .unwrap_or_else(|| vec![class_name.to_string()])
    }

    /// Whether any known **strict descendant** of `class_name` resolves
    /// `method` — the template-method abstention for `my` dispatch.
    ///
    /// An abstract base whose method bodies call `my M`, with `M` written
    /// only by concrete subclasses, runs fine: `my` late-binds on the
    /// actual receiver, which is always a subclass instance (the base is
    /// never instantiated), and bypasses export filtering, so even an
    /// unexported (capitalised) subclass method is reachable.  A known
    /// subclass that defines `M` is direct evidence the call is that
    /// pattern and not a typo — with **no** defining subclass anywhere in
    /// the index, W308 still fires, which is what keeps this from
    /// swallowing the check on self-dispatch entirely.
    ///
    /// Descendants are found through [`Self::mro_map`] (any class whose
    /// MRO contains `class_name`), and `method` resolves through
    /// [`Self::method_target`] on the descendant, so a template method the
    /// subclass itself inherits from a mixin still counts.
    #[must_use]
    pub fn subclass_provides_method(&self, class_name: &str, method: &str) -> bool {
        self.mro_map.iter().any(|(cls, mro)| {
            cls != class_name
                && mro.iter().any(|ancestor| ancestor == class_name)
                && self.method_target(cls, method).is_some()
        })
    }

    /// Every instance method some class on `class_name`'s MRO **generates**
    /// from its declared `property` members — `oo::configurable`'s
    /// `configure` (Tcl 9.0+).
    ///
    /// Registry data throughout: the names come from the manufacturing
    /// metaclass's
    /// [`property_accessor_methods`](tcl_registry::definer::DefinitionBodyGrammar::property_accessor_methods),
    /// so no consumer spells `configure` and none can invent a generated
    /// method the class system does not have. `cget` is not one of them:
    /// `configurable.n` documents no `cget` in either 9.0 or 9.1, and
    /// dispatching one really does fail.
    ///
    /// The whole MRO is asked, not just the receiver's own class, because the
    /// generated method is a real method on the configurable ancestor and a
    /// subclass inherits it.  A class that declares `property` members but
    /// whose metaclass resolves to no grammar — a user metaclass derived from
    /// `oo::configurable` — falls back to the dialect-wide
    /// [`CommandRegistry::property_accessor_methods`](tcl_registry::CommandRegistry::property_accessor_methods),
    /// so such a class is not told its accessor is missing.
    ///
    /// Empty when nothing on the MRO is configurable, and for a registry the
    /// caller does not have.
    #[must_use]
    pub fn property_accessor_methods(
        &self,
        registry: Option<&tcl_registry::CommandRegistry>,
        class_name: &str,
    ) -> Vec<&'static str> {
        let Some(registry) = registry else {
            return Vec::new();
        };
        let mut out: Vec<&'static str> = Vec::new();
        let mut declares_properties = false;
        for ancestor in self.mro_or_self(class_name) {
            let Some(cd) = self.classes.get(&ancestor) else {
                continue;
            };
            declares_properties |= !cd.properties.is_empty();
            if let Some(grammar) = registry.get(&cd.metaclass).and_then(|s| s.definition_body) {
                out.extend(grammar.property_accessor_methods.iter().copied());
            }
        }
        if out.is_empty() && declares_properties {
            out = registry.property_accessor_methods();
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `method` is one the class system generates for `class_name`'s
    /// properties — see [`Self::property_accessor_methods`].
    #[must_use]
    pub fn is_property_accessor(
        &self,
        registry: Option<&tcl_registry::CommandRegistry>,
        class_name: &str,
        method: &str,
    ) -> bool {
        self.property_accessor_methods(registry, class_name)
            .contains(&method)
    }

    /// Every method name callable on `class_name` — instance and class
    /// methods declared anywhere on its linearised MRO, plus whatever its
    /// class system generates from declared properties — sorted and
    /// deduplicated.  Feeds the W308 "did you mean…?" suggestion list; empty
    /// for an unknown class.
    ///
    /// The generated-method fold and the W308 *existence* check
    /// (`Analyser::validate_method_on_class`) both go through
    /// [`Self::property_accessor_methods`] precisely so they cannot disagree:
    /// were only this side to fold them in, `$obj configure` on a configurable
    /// class would draw "Unknown method 'configure' … did you mean
    /// 'configure'?".
    #[must_use]
    pub fn known_methods(
        &self,
        registry: Option<&tcl_registry::CommandRegistry>,
        class_name: &str,
    ) -> Vec<String> {
        let mut out: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        if let Some(mro) = self.mro_map.get(class_name) {
            for ancestor in mro {
                if let Some(cd) = self.classes.get(ancestor) {
                    out.extend(cd.methods.keys().cloned());
                    out.extend(cd.class_methods.keys().cloned());
                }
            }
        }
        out.extend(
            self.property_accessor_methods(registry, class_name)
                .into_iter()
                .map(str::to_owned),
        );
        out.into_iter().collect()
    }
}

/// Whether a `TclOO` method/constructor body span is *effectively* empty —
/// `{}` / `[]` / `""` with **nothing at all** between the delimiters.
/// `TclOO` treats exactly this shape as "no constructor was written" (see
/// [`ClassHierarchy::constructor_provider`]'s doc comment); any body
/// content at all, even a single space or a comment, keeps it real
/// (confirmed against tclsh 9.0.4).
///
/// `body_span` follows this project's general delimited-word convention
/// (`docs/design/contracts/lexing.md`): for
/// non-empty content the span covers the opening delimiter through the
/// *last inner character*, excluding the closer (e.g. `{ }`'s span slices
/// to `"{ "`, two bytes); for genuinely empty content the span instead
/// extends one further to sit *on* the closer (`{}`'s span slices to the
/// full `"{}"`, also two bytes, but ending on `}` rather than before it).
/// So an empty body is exactly the two-byte slice `"{}"` / `"[]"` /
/// `"\"\""` — no non-empty content is ever pair-shaped like that (a
/// single-inner-character body such as `{x}` slices to `"{x"`, not `"{}"`)
/// — a cheap, unambiguous string-equality test rather than re-parsing.
pub(in crate::analyser) fn is_empty_method_body(source: &str, body_span: tcl_lexer::Span) -> bool {
    let start = body_span.start() as usize;
    let end = body_span.end() as usize;
    matches!(source.get(start..end), Some("{}" | "[]" | "\"\""))
}

/// Resolve a possibly-bare superclass / mixin name written in the body of
/// class `owner` to a qualified name keyed in `classes`.
///
/// Explicitly constructed graph keys resolve in the enclosing namespace,
/// then globally. A unique tail supplies no import or caller ownership.
/// Source declarations use their retained relation lookup instead.
fn resolve_super_name(
    name: &str,
    owner_qname: &str,
    classes: &HashMap<String, ClassDef>,
    tail_index: &HashMap<String, Vec<String>>,
) -> String {
    // The MRO builder wants a name back even when nothing resolves (it leaves
    // the edge unlinked rather than dropping the class), so fall back to the
    // written name; the resolution logic itself lives in `resolve_class_name`.
    resolve_class_name(name, owner_qname, |q| classes.contains_key(q), tail_index)
        .unwrap_or_else(|| name.to_string())
}

/// Compatibility assistance for explicitly constructed class graph keys.
/// Source declarations require original caller lookup receipts instead. This
/// adapter supplies no native class, provider, import or execution authority.
pub fn resolve_class_name<S: std::hash::BuildHasher>(
    name: &str,
    owner_qname: &str,
    is_known: impl Fn(&str) -> bool,
    tail_index: &HashMap<String, Vec<String>, S>,
) -> Option<String> {
    if is_known(name) {
        return Some(name.to_string());
    }
    // C Tcl resolves a bare `superclass`/`mixin` name relative to the enclosing
    // (`oo::define` call-site) namespace in exactly two scopes — the current
    // namespace, then global — with NO walk through intermediate ancestors.
    // Verified against the VM's `cmd_oo::resolve_class` and C's
    // `GetClassInOuterContext` (`tclOODefineCmds.c`).  An ancestor walk
    // manufactures a wrong inheritance edge (e.g. a bare `superclass Base` in
    // `::a::b::Sub` linked to `::a::Base` even though real Tcl errors — `Base`
    // is reachable from neither `::a::b` nor global).  Uses the shared command
    // -resolution candidate order so class-name resolution can never diverge
    // from it (a class name *is* a command name).
    let (owner_ns, _) = crate::naming::key_holder_and_tail(owner_qname);
    for cand in crate::naming::bareword_resolution_candidates(owner_ns, name) {
        if is_known(&cand) {
            return Some(cand);
        }
    }
    // Constructed graph compatibility has no source import/context receipt.
    let _ = tail_index;
    None
}

/// Positively retained Logical reporting lookup in the actual lexical
/// namespace. This shares command candidate ordering, without joining Native
/// source-name records, guessing a unique tail or issuing a class identity.
#[must_use]
pub fn resolve_retained_logical_class_name(
    analysis: &super::types::AnalysisResult,
    name: &str,
    namespace: &str,
) -> Option<String> {
    analysis
        .allows_retained_logical_declaration_advice()
        .then_some(())?;
    crate::naming::bareword_resolution_candidates(namespace, name)
        .into_iter()
        .find(|candidate| analysis.all_classes.contains_key(candidate))
}

/// Resolve original source naming against retained class publication slots.
/// Missing, colliding or foreign-policy records do not establish class identity.
#[must_use]
pub fn resolve_class_lookup<S: std::hash::BuildHasher>(
    lookup: &crate::signature_scan::scope::SignatureSourceLookup,
    classes: &HashMap<String, ClassDef, S>,
) -> Option<String> {
    for candidate in lookup.candidates()? {
        if classes.values().any(|class| {
            class.source_name_ambiguous.is_observed()
                && class.source_name.as_ref().is_some_and(|source| {
                    source.policy() == lookup.policy() && source.slot() == &candidate
                })
        }) {
            return None;
        }
        let mut matches = classes.iter().filter(|(_, class)| {
            !class.source_name_ambiguous.is_observed()
                && class.source_name.as_ref().is_some_and(|source| {
                    source.policy() == lookup.policy() && source.slot() == &candidate
                })
        });
        if let Some((key, _)) = matches.next() {
            return matches.next().is_none().then(|| key.clone());
        }
    }
    None
}

/// Source lookup in an independently supplied original caller context.
#[must_use]
pub fn resolve_written_class_name_in_context<S: std::hash::BuildHasher>(
    name: &str,
    namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    classes: &HashMap<String, ClassDef, S>,
) -> Option<String> {
    let lookup = crate::signature_scan::scope::SignatureSourceLookup::new(
        policy,
        namespace.clone(),
        name.to_owned(),
    )?;
    resolve_class_lookup(&lookup, classes)
}

/// Source query in a retained scope, without guessing a missing object frame.
#[must_use]
pub fn resolve_written_class_name_in_scope<S: std::hash::BuildHasher>(
    name: &str,
    namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
    classes: &HashMap<String, ClassDef, S>,
) -> Option<String> {
    let policies: HashSet<_> = classes
        .values()
        .filter_map(|class| {
            class
                .source_name
                .as_ref()
                .map(crate::signature_scan::scope::SignatureSourceCommand::policy)
        })
        .collect();
    let mut selected = None;
    for policy in policies {
        if let Some(key) = resolve_written_class_name_in_context(name, namespace, policy, classes) {
            if selected.as_ref().is_some_and(|previous| previous != &key) {
                return None;
            }
            selected = Some(key);
        }
    }
    selected
}

/// Exact innermost source scope containing a written query position.
#[must_use]
pub fn source_namespace_at(
    scope: &super::types::Scope,
    offset: u32,
) -> Option<&crate::signature_scan::scope::SignatureNamespaceScope> {
    let mut children = scope.children.iter().filter(|child| {
        child
            .body_span
            .is_some_and(|span| span.start() <= offset && offset < span.end())
    });
    if let Some(child) = children.next() {
        if children.next().is_some() {
            return None;
        }
        return source_namespace_at(child, offset);
    }
    scope.naming_scope.as_ref()
}

/// Unpositioned root lookup using retained declaration policies only.
/// A missing original caller context cannot borrow a globally unique tail.
#[must_use]
pub fn resolve_written_class_name<S: std::hash::BuildHasher>(
    name: &str,
    classes: &HashMap<String, ClassDef, S>,
) -> Option<String> {
    let policies: HashSet<_> = classes
        .values()
        .filter_map(|class| {
            class
                .source_name
                .as_ref()
                .map(crate::signature_scan::scope::SignatureSourceCommand::policy)
        })
        .collect();
    let mut selected = None;
    for policy in policies {
        if let Some(key) = resolve_written_class_name_in_context(
            name,
            &crate::signature_scan::scope::SignatureNamespaceScope::root(Some(policy)),
            policy,
            classes,
        ) {
            if selected.as_ref().is_some_and(|previous| previous != &key) {
                return None;
            }
            selected = Some(key);
        }
    }
    selected
}

/// Presentation-only simple-name index; it grants no class lookup authority.
pub fn build_tail_index<'a>(
    qnames: impl Iterator<Item = &'a String>,
) -> HashMap<String, Vec<String>> {
    let mut tail_index: HashMap<String, Vec<String>> = HashMap::new();
    for qname in qnames {
        // `qname` is a constructed key — construction-inverse tail.
        let tail = crate::naming::key_tail(qname);
        tail_index
            .entry(tail.to_string())
            .or_default()
            .push(qname.clone());
    }
    tail_index
}

fn unresolved_class_relation_row(class: &str, written: &str) -> String {
    format!("@unresolvedclass@{class} {written}")
}

struct ClassLinearisationMaps {
    mro_map: HashMap<String, Vec<String>>,
    errors: Vec<super::MroError>,
    spine_map: HashMap<String, Vec<String>>,
}

fn build_class_linearisation_maps(
    classes: &HashMap<String, ClassDef>,
    supers_map: &HashMap<String, Vec<String>>,
    mixins_map: &HashMap<String, Vec<String>>,
) -> ClassLinearisationMaps {
    // Compute MRO for every class. `build_mro_map` walks `supers_map.keys()`
    // only; classes that appear as parents but aren't in the index get a
    // single-element MRO when queried directly.
    let (mro_map_raw, errors_pass1) = build_mro_map(supers_map, mixins_map);
    let mut mro_map: HashMap<String, Vec<String>> = mro_map_raw;
    let mut errors = errors_pass1;
    for qname in classes.keys() {
        if has_relation_cycle(qname, supers_map, mixins_map) {
            let error = super::MroError::new(super::mro::MroErrorKind::RelationCycle, qname);
            if !errors.contains(&error) {
                errors.push(error);
            }
        }
    }
    for qname in classes.keys() {
        if !mro_map.contains_key(qname) {
            mro_map.insert(qname.clone(), vec![qname.clone()]);
        }
    }
    // Re-run linearise for classes that got the backfilled self-only MRO;
    // build_mro_map may have stopped at a cycle before visiting them.
    for qname in classes.keys() {
        if mro_map[qname] == vec![qname.clone()] && !classes[qname].superclasses.is_empty() {
            match tcloo_linearise(qname, supers_map, mixins_map) {
                Ok(mro) => {
                    mro_map.insert(qname.clone(), mro);
                }
                Err(e) => {
                    if !errors.contains(&e) {
                        errors.push(e);
                    }
                }
            }
        }
    }

    // The mixin-free spine uses the same resolved superclasses, with only
    // mixin edges withheld, and is backfilled so every indexed class exists.
    let no_mixins: HashMap<String, Vec<String>> = HashMap::new();
    let (mut spine_map, _spine_errors) = build_mro_map(supers_map, &no_mixins);
    for qname in classes.keys() {
        if !spine_map.contains_key(qname) {
            spine_map.insert(qname.clone(), vec![qname.clone()]);
        }
    }
    ClassLinearisationMaps {
        mro_map,
        errors,
        spine_map,
    }
}

/// Build the supers/mixins maps used by the MRO algorithm.
fn build_supers_mixins_maps(
    classes: &HashMap<String, ClassDef>,
) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
    let mut supers_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut mixins_map: HashMap<String, Vec<String>> = HashMap::new();
    for (qname, class) in classes {
        let normalise = |names: &[String]| -> Vec<String> {
            names
                .iter()
                .map(|written| match class.relation_lookups.get(written) {
                    Some(Some(lookup)) => resolve_class_lookup(lookup, classes)
                        .unwrap_or_else(|| unresolved_class_relation_row(qname, written)),
                    None if class.source_name.is_none() => {
                        resolve_super_name(written, qname, classes, &HashMap::new())
                    }
                    Some(None) | None => unresolved_class_relation_row(qname, written),
                })
                .collect()
        };
        supers_map.insert(qname.clone(), normalise(&class.superclasses));
        if !class.mixins.is_empty() {
            mixins_map.insert(qname.clone(), normalise(&class.mixins));
        }
    }
    (supers_map, mixins_map)
}

/// Whether the combined, already-normalised superclass/mixin graph reachable
/// from `start` revisits a node on its active path. The shared MRO primitive
/// deliberately cuts off mixin revisits, but an analysis hierarchy records
/// every relation cycle as an error so downstream proofs never mistake that
/// truncated order for a complete method chain.
fn has_relation_cycle(
    start: &str,
    supers_map: &HashMap<String, Vec<String>>,
    mixins_map: &HashMap<String, Vec<String>>,
) -> bool {
    fn visit(
        class_name: &str,
        supers_map: &HashMap<String, Vec<String>>,
        mixins_map: &HashMap<String, Vec<String>>,
        active: &mut HashSet<String>,
        complete: &mut HashSet<String>,
    ) -> bool {
        if active.contains(class_name) {
            return true;
        }
        if complete.contains(class_name) {
            return false;
        }
        active.insert(class_name.to_owned());
        let relations = supers_map
            .get(class_name)
            .into_iter()
            .flatten()
            .chain(mixins_map.get(class_name).into_iter().flatten());
        for relation in relations {
            if visit(relation, supers_map, mixins_map, active, complete) {
                return true;
            }
        }
        active.remove(class_name);
        complete.insert(class_name.to_owned());
        false
    }

    visit(
        start,
        supers_map,
        mixins_map,
        &mut HashSet::new(),
        &mut HashSet::new(),
    )
}

/// Build the method-provider map: `(class, method) -> first ancestor
/// in MRO order that provides the method`.
fn build_method_providers(
    classes: &HashMap<String, ClassDef>,
    mro_map: &HashMap<String, Vec<String>>,
) -> HashMap<(String, String), String> {
    let mut method_providers: HashMap<(String, String), String> = HashMap::new();
    for (qname, cd) in classes {
        let mro = mro_map
            .get(qname)
            .cloned()
            .unwrap_or_else(|| vec![qname.clone()]);
        let mut all_methods: HashSet<String> = HashSet::new();
        for n in cd.methods.keys() {
            all_methods.insert(n.clone());
        }
        for n in cd.class_methods.keys() {
            all_methods.insert(n.clone());
        }
        for ancestor_name in &mro {
            if let Some(ancestor) = classes.get(ancestor_name) {
                for n in ancestor.methods.keys() {
                    all_methods.insert(n.clone());
                }
                for n in ancestor.class_methods.keys() {
                    all_methods.insert(n.clone());
                }
            }
        }
        for method_name in all_methods {
            for ancestor_name in &mro {
                if let Some(ancestor) = classes.get(ancestor_name)
                    && (ancestor.methods.contains_key(&method_name)
                        || ancestor.class_methods.contains_key(&method_name))
                {
                    method_providers
                        .insert((qname.clone(), method_name.clone()), ancestor_name.clone());
                    break;
                }
            }
        }
    }
    method_providers
}

/// Build the [`ClassHierarchy`] from a flat class-def index.  Runs
/// MRO linearisation, computes direct + transitive subclass sets, and
/// resolves method-provider chains.
#[must_use]
pub fn build_class_hierarchy<S: std::hash::BuildHasher>(
    classes: HashMap<String, ClassDef, S>,
) -> ClassHierarchy {
    // Normalise to the default-hasher map used by `ClassHierarchy::classes`
    // so the rest of the routine (and the public field) stay hasher-agnostic.
    let classes: HashMap<String, ClassDef> = classes.into_iter().collect();
    let (supers_map, mixins_map) = build_supers_mixins_maps(&classes);
    let mut original_unresolved_relations = HashMap::new();
    for (qname, class) in &classes {
        for written in class.superclasses.iter().chain(&class.mixins) {
            let Some(Some(lookup)) = class.relation_lookups.get(written) else {
                continue;
            };
            if resolve_class_lookup(lookup, &classes).is_some() {
                continue;
            }
            let row = unresolved_class_relation_row(qname, written);
            let captured = original_unresolved_relations
                .entry(row)
                .or_insert_with(|| Some(lookup.clone()));
            if captured.as_ref().is_some_and(|previous| previous != lookup) {
                *captured = None;
            }
        }
    }

    let ClassLinearisationMaps {
        mro_map,
        mut errors,
        spine_map,
    } = build_class_linearisation_maps(&classes, &supers_map, &mixins_map);

    // Build direct-subclass map.  Initialise with empty sets
    // for every class so callers see a non-None entry even when
    // a class has no subclasses.
    //
    // A class is a subtype of both its superclasses **and** its mixins — the
    // MRO (and thus `is_subtype` / `method_target`) already treats a mixin as
    // a supertype, so the subclass map must be built from both edges too, or
    // it disagrees with `is_subtype` (asymmetrically, depending on which file
    // a mixing-in subclass happens to live in).  Union supers + mixins here.
    let mut direct_subs: HashMap<String, HashSet<String>> = HashMap::new();
    for qname in classes.keys() {
        direct_subs.insert(qname.clone(), HashSet::new());
    }
    for qname in classes.keys() {
        let parents = supers_map
            .get(qname)
            .into_iter()
            .flatten()
            .chain(mixins_map.get(qname).into_iter().flatten());
        for parent in parents {
            if let Some(set) = direct_subs.get_mut(parent) {
                set.insert(qname.clone());
            }
        }
    }

    // Build transitive-subtype closure via BFS over direct subs.
    let mut transitive: HashMap<String, HashSet<String>> = HashMap::new();
    for qname in classes.keys() {
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = direct_subs
            .get(qname)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        while let Some(child) = stack.pop() {
            if visited.contains(&child) {
                continue;
            }
            visited.insert(child.clone());
            if let Some(grandchildren) = direct_subs.get(&child) {
                for g in grandchildren {
                    stack.push(g.clone());
                }
            }
        }
        transitive.insert(qname.clone(), visited);
    }

    let method_providers = build_method_providers(&classes, &mro_map);

    // `errors` is accumulated while walking `classes.keys()` (HashMap order), so
    // its `Vec` order is otherwise nondeterministic.  Sort it: the order carries
    // no meaning, and a stable order keeps two logically-equal hierarchies
    // `==` — which is what lets salsa backdate `project_class_index` on a
    // body-only edit instead of needlessly recomputing dependent token queries.
    errors.sort_unstable();

    ClassHierarchy {
        classes,
        mro_map,
        original_unresolved_relations,
        mixin_map: mixins_map,
        spine_map,
        subclasses: direct_subs,
        transitive_subtypes: transitive,
        method_providers,
        errors,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod source_naming_tests;

#[cfg(test)]
mod constructed_projection_tests;

#[cfg(test)]
mod source_member_kind_controls;

#[cfg(test)]
mod source_class_object_member_kind_controls;

#[cfg(test)]
mod source_class_object_unicode_body_controls;

#[cfg(test)]
mod source_native_counted_member_name_controls;
