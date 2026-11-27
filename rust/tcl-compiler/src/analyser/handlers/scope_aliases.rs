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

//! Names linked to a cell in another scope: state-transition aliases, `global`, `variable` and `upvar`.

use super::super::state::Analyser;
use tcl_lexer::{Span, Token};

impl Analyser {
    /// Apply the scope aliases one invocation's state transitions state: each
    /// [`tcl_registry::VariableCellAliasTransition`] defines its local and
    /// links it to the cell it aliases, whichever command — `global`,
    /// `variable`, `upvar`, `namespace upvar`, or a pack command whose
    /// `state_transitions` resolver states alias facts — said so. Nothing
    /// here knows a command's layout: the registry resolver located every
    /// word, and [`tcl_registry::AliasWords`] says where each is spelled.
    ///
    /// Called from the dispatch tail for every command the walk's context
    /// proves, with `invocation` resolved over the words' source facts, so a
    /// computed word reaches the resolver as computed and the fact abstains
    /// on it (`alias_and_binding_resolvers_abstain_on_a_dynamic_word` pins
    /// the registry side).
    pub fn apply_state_transitions(
        &mut self,
        transitions: &tcl_registry::StateTransitions,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        for fact in transitions.facts() {
            if let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition {
                self.apply_variable_alias(alias, args, arg_tokens, scope_path);
            }
        }
    }

    /// Define one alias fact's local and link it to its cell.
    ///
    /// The local is a declaration (`warn_if_unused = false`), never a "set
    /// but unused" write. A computed local names no static variable, so the
    /// fact binds nothing. The link unifies every alias of one cell for
    /// Find-References and Rename; `target_span` is the word that names the
    /// cell — the declaration word itself for `global v`, one word earlier
    /// for `upvar #0 v local` — see [`super::super::types::VarDef::link_target_span`].
    fn apply_variable_alias(
        &mut self,
        alias: &tcl_registry::VariableCellAliasTransition,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let Some(local) = alias.local.literal() else {
            return;
        };
        let Some(local_tok) = arg_tokens.get(alias.words.local).copied() else {
            return;
        };
        self.define_var(local, local_tok, scope_path, false, None);
        let Some(target_tok) = arg_tokens.get(alias.words.target).copied() else {
            return;
        };
        let namespace =
            self.original_alias_target_namespace((local_tok.span, target_tok.span), scope_path);
        let original = self.record_original_variable_alias_target(
            local_tok.span,
            target_tok.span,
            namespace,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
        );
        if let Some(report) = self.alias_target_report(&alias.target, args, scope_path) {
            self.set_var_link_report(local, scope_path, report, target_tok.span);
        }
        if matches!(
            alias.target,
            tcl_registry::VariableAliasTarget::CallerSelectedFrame { .. }
        ) && let Some(original) = original
        {
            self.define_original_namespace_alias_target(&original);
        }
    }

    fn original_alias_target_namespace(
        &self,
        operands: (Span, Span),
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        let input = self.resolved_analysis_input();
        let target_input = self.original_qualified_variable_input(operands.1, false)?;
        let current = if input.has_logical_source_name_context() {
            self.head_identities
                .original_logical_variable_scope_at(
                    &input,
                    &tcl_lexer::SourceImage::document(&self.source),
                    operands.1,
                    &target_input,
                )?
                .namespace()
        } else {
            self.original_variable_namespace_at(scope_path)?
        };
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                &input,
                self.source_analysis_entry.as_deref(),
                self.declaration_name_policy()?,
            );
        let mut selected = None;
        for source in &self.result.original_variable_alias_source_operands {
            if !source
                .operands()
                .iter()
                .any(|&(local, target, _)| (local, target) == operands)
            {
                continue;
            }
            let namespace = source.target_namespace_scope(
                &self.source,
                &self.result,
                operands,
                &target_input,
                &current,
                purpose,
            )?;
            if selected.as_ref().is_some_and(|prior| prior != &namespace) {
                return None;
            }
            selected = Some(namespace);
        }
        selected
    }

    /// Reporting spelling and protective unknown markers. No symbol or target
    /// definition is constructed from this text.
    pub(in super::super) fn alias_target_report(
        &self,
        target: &tcl_registry::VariableAliasTarget,
        args: &[String],
        scope_path: &[usize],
    ) -> Option<String> {
        use tcl_registry::VariableAliasTarget as Target;
        /// A subject's value when the resolver knew it, else its source word
        /// as written — the spelling, substitution marker included, never
        /// taken for the value it computes.
        fn spelt<'a>(
            subject: &'a tcl_registry::TransitionSubject,
            args: &'a [String],
        ) -> Option<&'a str> {
            match subject {
                tcl_registry::TransitionSubject::Literal(name)
                | tcl_registry::TransitionSubject::LocatedLiteral { value: name, .. } => Some(name),
                tcl_registry::TransitionSubject::LocatedNativeBytes { value, .. } => {
                    std::str::from_utf8(value).ok()
                }
                tcl_registry::TransitionSubject::Unknown { argument_index, .. } => {
                    args.get(*argument_index).map(String::as_str)
                }
            }
        }
        let path = match target {
            Target::Global { variable } => crate::naming::qualify("::", variable.literal()?),
            Target::CurrentNamespace { variable } => crate::naming::qualify(
                &self.command_resolution_namespace(scope_path),
                variable.literal()?,
            ),
            Target::Namespace {
                namespace,
                variable,
            } => {
                let namespace = crate::naming::qualify_namespace(
                    &self.command_resolution_namespace(scope_path),
                    spelt(namespace, args)?,
                );
                crate::naming::qualify(&namespace, spelt(variable, args)?)
            }
            Target::CallerSelectedFrame { frame, variable } => {
                // The array an element word lives in is named by the word's
                // base whatever its key, so a computed key still leaves the
                // written base readable: the source spelling is read for that
                // base alone, never taken as the variable's value.
                let other = spelt(variable, args)?;
                let path = Self::upvar_link_target(other, self.alias_frame_level(frame))?;
                return Some(path);
            }
        };
        Some(path)
    }

    /// The frame an alias's caller-frame selection reaches, in the analysed
    /// release's level grammar (`uplevel 010` is 8 frames up on 8.6 and 10
    /// on 9.0). With no registry loaded there is no release to name, so the
    /// parse abstains with `Dynamic` wherever the releases disagree, as does
    /// a computed level word — the direction every consumer of this level
    /// already treats conservatively.
    fn alias_frame_level(
        &self,
        frame: &tcl_registry::CallerFrameSelection,
    ) -> tcl_registry::frame_effect::FrameLevel {
        use tcl_registry::frame_effect::FrameLevel;
        match frame {
            tcl_registry::CallerFrameSelection::DefaultCaller => FrameLevel::DEFAULT,
            tcl_registry::CallerFrameSelection::Explicit(level) => level
                .literal()
                .and_then(|level| {
                    self.registry.as_ref().map_or_else(
                        || FrameLevel::parse(level),
                        |r| FrameLevel::parse_in(level, r),
                    )
                })
                .unwrap_or(FrameLevel::Dynamic),
        }
    }

    /// Handle a `global` declaration: a flat list of names; each gets
    /// a var binding with `warn_if_unused = false` (declared, not
    /// "set but unused").
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Global`].
    #[cfg(test)]
    pub fn handle_global_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        // `global ::ns::v` makes the *unqualified tail* (`v`) a local alias
        // to the global variable, so define the tail name locally (matches
        // Tcl + completion's expectation of both `$v` and `$::ns::v`).
        for (i, name) in args.iter().enumerate() {
            let Some(local) = self.declaration_local_alias(
                name,
                i,
                tcl_registry::state_transition::VariableAliasNamePurpose::Global,
            ) else {
                continue;
            };
            // A dynamic name (`global $dyn`) computes its name at runtime — not
            // a static declaration.
            if let Some(tok) = arg_tokens.get(i)
                && !crate::naming::is_dynamic_word(name)
            {
                self.define_var(&local, *tok, scope_path, false, None);
                // `global v` aliases the global cell `::v`; `global ::ns::v`
                // aliases `::ns::v` as written.  Record the target so every
                // `global` alias and the global declaration unify.
                let Some(target) = self.declaration_alias_target("::", name) else {
                    continue;
                };
                // The declaration word *is* the cell's name here (`global v`
                // / `global ::ns::v`), so a rename of the cell rewrites this
                // very word — see `VarDef::link_target_span`.
                self.set_var_link_target(
                    &local,
                    scope_path,
                    super::super::scope::VariableLinkTarget {
                        target,
                        target_span: tok.span,
                        local_span: tok.span,
                        original_target_namespace: self.original_variable_root_namespace(),
                        original_target_receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
                    },
                );
            }
        }
    }

    /// Handle a `variable` declaration: alternating `name ?value?`
    /// pairs; only the names get bindings. The optional value words
    /// are skipped (the IR pass handles their assignment if the value
    /// form actually fires).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Variable`].
    #[cfg(test)]
    pub fn handle_variable_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        // `variable name ?value? name ?value? ...`
        // Each `name` aliases the cell `<current-namespace>::<name>`; every
        // `variable name` across that namespace's procs, plus the namespace
        // -level declaration, shares that target and so unifies.
        let ns = self.command_resolution_namespace(scope_path);
        let mut i = 0;
        while i < args.len() {
            // A dynamic name (`variable $dyn` / `variable [f]`) is computed at
            // runtime — its literal text is not the variable's name, so it is
            // not a static declaration.  Skip it rather than record the
            // substitution text as a variable.
            if let Some(tok) = arg_tokens.get(i)
                && !crate::naming::is_dynamic_word(&args[i])
            {
                // Mirror `global` above: the *unqualified tail* is the local
                // alias name, but the target keeps the FULL qualified path so a
                // relative `variable child::v` aliases `<ns>::child::v` (and an
                // absolute `variable ::x::v` aliases `::x::v`), never the
                // tail-collapsed `<ns>::v`.  Keying the link on the tail is what
                // lets a later `$v` reference share the target and unify.
                let Some(local) = self.declaration_local_alias(
                    &args[i],
                    i,
                    tcl_registry::state_transition::VariableAliasNamePurpose::NamespaceVariable,
                ) else {
                    i += if i + 1 < args.len() { 2 } else { 1 };
                    continue;
                };
                self.define_var(&local, *tok, scope_path, false, None);
                let Some(target) = self.declaration_alias_target(&ns, &args[i]) else {
                    i += if i + 1 < args.len() { 2 } else { 1 };
                    continue;
                };
                // As with `global`, the declaration word names the cell.
                self.set_var_link_target(
                    &local,
                    scope_path,
                    super::super::scope::VariableLinkTarget {
                        target,
                        target_span: tok.span,
                        local_span: tok.span,
                        original_target_namespace: self.original_variable_namespace_at(scope_path),
                        original_target_receiver: if self.head_identities.source_bindings_ref().original_variable_uses_global_frame_at_span(tok.span, self.file_lexer_config())
                        && self.original_variable_namespace_at(scope_path).is_some_and(|namespace| match namespace {
                            crate::signature_scan::scope::SignatureNamespaceScope::C(path) => path.is_root(),
                            crate::signature_scan::scope::SignatureNamespaceScope::Jim(object) => object.is_empty(),
                            crate::signature_scan::scope::SignatureNamespaceScope::Symbolic(_) => false,
                        })
                    {
                        crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceVariableWithoutLink
                    } else {
                        crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias
                    },
                    },
                );
            }
            i += if i + 1 < args.len() { 2 } else { 1 };
        }
    }

    #[cfg(test)]
    fn declaration_local_alias(
        &self,
        name: &str,
        argument_index: usize,
        purpose: tcl_registry::state_transition::VariableAliasNamePurpose,
    ) -> Option<String> {
        let dialect = self.declaration_name_dialect();
        let subject = tcl_registry::state_transition::local_alias_name(
            &tcl_registry::TransitionSubject::Literal(name.to_owned()),
            argument_index,
            purpose,
            Some(dialect),
        )?;
        subject.literal().map(str::to_owned)
    }

    #[cfg(test)]
    fn declaration_alias_target(&self, namespace: &str, name: &str) -> Option<String> {
        let dialect = self.declaration_name_dialect();
        if dialect.native_name_protocol().is_none()
            && dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            && !name.contains('\0')
        {
            return Some(crate::naming::qualify(namespace, name));
        }
        Some(match dialect.native_name_protocol()? {
            tcl_syntax::naming::NativeNameProtocol::C(_) => crate::naming::qualify(namespace, name),
            tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                format!(
                    "::{}",
                    crate::naming::jim_global_variable_key(namespace, name)
                )
            }
        })
    }

    // Registry clause plans supply body depths and per-clause variable bindings.
    // Selected handler bodies retain conditional control flow for name mutations.

    /// The fixed cell a frame-crossing alias's `otherVar` word names, or
    /// `None` when it names a frame-relative variable with no stable path.
    ///
    /// `otherVar` normally names a variable in *another frame*, which has no
    /// namespace path to link to — a bare `x` at level 1 is whatever local
    /// the caller happens to have. Two spellings escape that and name one
    /// fixed cell whatever the call depth (without them `upvar
    /// ::tk::FocusGrab($index) data` leaves the array completely
    /// unregistered):
    ///
    /// * a **fully-qualified** target — `upvar 1 ::myns::q g` binds
    ///   `::myns::q` from any depth (tclsh 9.0.4 / 8.6.14: reached
    ///   identically through `upvar 1` and `upvar 2`);
    /// * any target at **`#0`** — the global frame, so `upvar #0 counter c`
    ///   binds `::counter` however deep the call stack is.
    ///
    /// An array element (`::tk::FocusGrab($index)`) links to its **base**:
    /// the element key is runtime data, but the array it lives in is the
    /// named cell every sibling access shares.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Upvar`].
    pub fn handle_upvar_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        use tcl_registry::frame_effect::FrameLevel;

        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let pair_start = usize::from(args.len() % 2 == 1);
        let level = if pair_start == 1 {
            // The level word's grammar is the analysed release's (`uplevel 010`
            // is 8 frames up on 8.6 and 10 on 9.0). With no registry loaded
            // there is no release to name, so `parse` abstains with `Dynamic`
            // wherever the releases disagree — the direction every consumer of
            // this level already treats conservatively.
            self.registry
                .as_ref()
                .map_or_else(
                    || FrameLevel::parse(&args[0]),
                    |r| FrameLevel::parse_in(&args[0], r),
                )
                .unwrap_or(FrameLevel::Dynamic)
        } else {
            FrameLevel::DEFAULT
        };
        let mut i = pair_start + 1;
        while i < args.len() && i < arg_tokens.len() {
            // The local alias name may be dynamic (`upvar 1 x $local`) — skip
            // it rather than record the substitution text.
            if !crate::naming::is_dynamic_word(&args[i]) {
                self.define_var(&args[i], arg_tokens[i], scope_path, false, None);
                if let Some(target) = Self::upvar_link_target(&args[i - 1], level)
                    && let Some(other_tok) = arg_tokens.get(i - 1)
                {
                    // `otherVar` (at `i - 1`) names the cell; `args[i]` is an
                    // independent local spelling.  Renaming the cell must
                    // rewrite the former, never the latter.
                    let original_namespace = self
                        .original_qualified_variable_input(other_tok.span, false)
                        .and_then(|input| {
                            let absolute = input
                                .policy()
                                .recipe()
                                .variable_root_input(input.bytes())
                                .qualification()
                                == tcl_syntax::naming::NativeNameQualification::Absolute;
                            let global = if pair_start == 1 {
                                self.original_variable_operand_input(arg_tokens[0].span)
                                    .and_then(|level| {
                                        let dialect =
                                            tcl_native_model::InvocationDialect::of_profile(
                                                self.profile,
                                            );
                                        FrameLevel::parse_native_bytes(level.bytes(), dialect)?
                                            .map(FrameLevel::is_global_frame)
                                    })
                                    == Some(true)
                            } else {
                                false
                            };
                            (absolute || global)
                                .then(|| self.original_variable_root_namespace())
                                .flatten()
                        });
                    self.set_var_link_target(
                        &args[i],
                        scope_path,
                        super::super::scope::VariableLinkTarget {
                            target: target.clone(),
                            target_span: other_tok.span,
                            local_span: arg_tokens[i].span,
                            original_target_namespace: original_namespace,
                            original_target_receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
                        },
                    );
                    // The fixed cell itself gets a definition at the
                    // `otherVar` word: `upvar ::tk::FocusGrab($index) data`
                    // is the
                    // only place the array ever comes to exist, so without
                    // a `VarDef` for `::tk::FocusGrab` every occurrence —
                    // this word, a sibling proc's `$::tk::FocusGrab($idx)`
                    // read, its `info exists` / `unset` — answered nothing.
                    // Defined in the GLOBAL scope: both qualifying
                    // spellings (a `::`-qualified target, any target at
                    // `#0`) name a cell reachable from everywhere.
                    self.define_var(&target, *other_tok, &[], false, None);
                }
            }
            i += 2;
        }
    }

    /// The fixed cell an `upvar` `otherVar` word names, or `None` when it
    /// names a frame-relative variable with no stable path.
    ///
    /// See [`Self::handle_upvar_command`] for the two qualifying spellings
    /// and their oracle transcripts.
    fn upvar_link_target(
        other: &str,
        level: tcl_registry::frame_effect::FrameLevel,
    ) -> Option<String> {
        // `a($k)` names the array `a`; the element key is runtime data, so
        // the base is tested for dynamism, not the whole word.
        let base = crate::naming::split_element_ref(other).map_or(other, |(base, _)| base);
        if base.is_empty() || crate::naming::is_dynamic_word(base) {
            return None;
        }
        (tcl_syntax::naming::is_absolute(base) || level.is_global_frame())
            .then(|| crate::naming::qualify("::", base))
    }
}
