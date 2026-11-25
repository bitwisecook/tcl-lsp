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

//! What a script loads: `package require`, `provide`, `ifneeded` and `prefer`, `source` and the package search path.

use super::super::state::Analyser;
use super::{role_position, role_texts};
use crate::registry_invocation::{load_trace_flag, load_trace_option};
use tcl_lexer::{Token, TokenType};

impl Analyser {
    /// Handle ``package require ?-exact? NAME ?requirement ...?`` — record
    /// the package dependency so later passes can gate W123
    /// (unresolved-command) suppression and dynamic-provider detection.
    ///
    /// Appends a ``SignaturePackageRequire`` record to
    /// ``result.package_requires`` and flips ``has_dynamic_providers`` when
    /// the name is a ``$``-substitution / ``[…]``-substitution. Where each
    /// word sits is the registry's: the name and the requirements are the
    /// roles `require` declares after its option run, and ``exact`` is
    /// whether that run is non-empty — the subcommand's one option,
    /// ``-exact``, narrows each requirement to an exact version for the
    /// resolver (the reading `signature_scan`'s handler gives the same
    /// call). A call too short to carry a name records nothing.
    ///
    /// The conditional flag is ``self.conditional_depth > 0``.
    ///
    /// `cmd_tok` is the command-head token (the ``package``
    /// word).  The recorded
    /// [`SignaturePackageRequire::range`](crate::signature_scan::types::SignaturePackageRequire::range)
    /// uses its span so code-action / quick-fix UX points at the
    /// ``package`` keyword rather than at the ``require``
    /// subcommand word.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageRequire`] (stamped
    /// on `package`'s `require` subcommand); `args[0]` is still the
    /// subcommand word.
    fn original_package_operand(
        &self,
        token: Option<&Token>,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        let token = token?;
        let policy = self.declaration_name_policy()?;
        if let Some(input) = self
            .head_identities
            .original_written_name_input_at_span(token.span, self.file_lexer_config())
        {
            return (input.policy() == policy
                && !matches!(
                    input,
                    crate::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(_)
                ))
            .then_some(input);
        }
        let original = self.head_identities.original_source_name_at_span(
            token.span,
            self.file_lexer_config(),
            self.word_rules(),
            policy,
        )?;
        Some(
            crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                original.name_input().clone(),
            ),
        )
    }

    fn original_package_name(
        &self,
        token: Option<&Token>,
    ) -> Option<crate::signature_scan::original_name::SourcePackageName> {
        self.original_package_operand(token)
            .map(crate::signature_scan::original_name::SourcePackageName::from_input)
    }

    /// Record an original package requirement and its independently selected
    /// byte key, without inferring that its loader has executed.
    pub fn handle_package_require(
        &mut self,
        _cmd_name: &str,
        cmd_tok: Token,
        _args: &[String],
        arg_tokens: &[Token],
    ) {
        use tcl_registry::arg_role::ArgRole;
        let Some(original) = self.original_diagnostic_call_at(cmd_tok, arg_tokens) else {
            return;
        };
        let Some((Some(roles), exact)) = original.with_schema(|call| {
            (
                call.arg_roles(),
                call.option_effects().option_end > call.semantics.argument_offset,
            )
        }) else {
            return;
        };
        let Some((arguments, tokens, _single)) = original.source_arguments() else {
            return;
        };
        let args = arguments.as_slice();
        let arg_tokens = tokens.as_slice();
        let Some(name_idx) = role_position(&roles, ArgRole::Name) else {
            return;
        };
        let name_text = args[name_idx].clone();
        let requirements = role_texts(&roles, ArgRole::Value, args);
        let version = requirements.first().cloned();

        // Dynamic-provider detection — non-literal package
        // name suppresses W123 unknown-command emission
        // because the dynamic provider may register the
        // missing command at runtime.
        if let Some(name_tok) = arg_tokens.get(name_idx)
            && (matches!(name_tok.kind, TokenType::Var | TokenType::Cmd)
                || name_text.contains('$')
                || name_text.contains('['))
        {
            self.result.has_dynamic_providers = true;
        }

        let original_name = self.original_package_name(arg_tokens.get(name_idx));
        let original_requirements = roles
            .iter()
            .filter_map(|&(ordinal, role)| (role == ArgRole::Value).then_some(ordinal))
            .map(|ordinal| self.original_package_operand(arg_tokens.get(ordinal)))
            .collect();
        self.result
            .package_requires
            .push(crate::signature_scan::types::SignaturePackageRequire {
                original_name,
                original_requirements,
                name: name_text,
                version,
                requirements,
                exact,
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
                control_flow: self.control_flow_body_depth > 0,
                origin: crate::signature_scan::types::PackageRequireOrigin::Source,
            });
    }

    /// Record ``package provide NAME ?VERSION?`` — the name and the version
    /// are the roles `provide` declares.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageProvide`] (stamped
    /// on `package`'s `provide` subcommand).  See
    /// [`Self::handle_package_require`] for the `cmd_tok` anchoring
    /// convention.
    pub fn handle_package_provide(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if args.len() < 2 {
            return;
        }
        let original_name = self.original_package_name(arg_tokens.get(1));
        let name = args[1].clone();
        let version = args.get(2).cloned();
        self.result
            .package_provides
            .push(super::super::types::PackageProvide {
                original_name,
                name,
                version,
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
            });
    }

    /// Record ``package ifneeded NAME VERSION ?SCRIPT?``.
    ///
    /// Only the setter form is a record: ``package ifneeded NAME
    /// VERSION`` with no script is a *query* that registers nothing,
    /// so treating it as a registration would abstain a
    /// package-derived load order on a document that merely asked
    /// what was registered. The name, the version and the script are the
    /// roles `ifneeded` declares; the setter form is the one with a script.
    ///
    /// The script is not kept — see
    /// [`PackageIfneeded`](super::super::types::PackageIfneeded) for why the
    /// bare fact of a registration is the useful part.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageIfneeded`]
    /// (stamped on `package`'s `ifneeded` subcommand).  See
    /// [`Self::handle_package_require`] for the `cmd_tok` anchoring
    /// convention.
    pub fn handle_package_ifneeded(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        // args[0] is the `ifneeded` subcommand word; the setter form
        // is NAME + VERSION + SCRIPT.
        if args.len() < 4 {
            return;
        }
        let original_name = self.original_package_name(arg_tokens.get(1));
        self.result
            .package_ifneededs
            .push(super::super::types::PackageIfneeded {
                original_name,
                name: args[1].clone(),
                version: args[2].clone(),
                range: cmd_tok.span,
            });
    }

    /// Record ``package prefer latest`` — the one form of ``package
    /// prefer`` that changes the interpreter's version-selection rule.
    ///
    /// ``package prefer`` with no argument is a query, and ``package
    /// prefer stable`` never changes anything: it is a no-op from the
    /// default and silently ineffective once ``latest`` has been set
    /// (see [`crate::signature_scan::types::SignaturePackagePrefer`]
    /// for the transcript), so neither is recorded and the state is
    /// the monotone "has a raise already run".
    ///
    /// A non-literal mode word (``package prefer $mode``) is skipped
    /// rather than guessed at — flipping the selection rule on a
    /// substitution would silently move go-to-definition onto a
    /// different release of a package.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackagePrefer`] (stamped
    /// on `package`'s `prefer` subcommand).  See
    /// [`Self::handle_package_require`] for the `cmd_tok` anchoring
    /// convention.
    pub fn handle_package_prefer(&mut self, cmd_tok: Token, args: &[String]) {
        if args.len() < 2 || args[1] != "latest" {
            return;
        }
        self.result.package_prefer_latest.push(
            crate::signature_scan::types::SignaturePackagePrefer {
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
            },
        );
    }

    /// How many of `args`' leading words the **top-level** command `cmd_name`
    /// consumes as options, entirely from registry data — the whole-command
    /// counterpart of [`Self::namespace_leading_flag_words`], which answers the
    /// same question for a `namespace` *subcommand*.
    ///
    /// A leading word counts when it matches one of the command's
    /// profile-available [`OptionSpec`](tcl_registry::OptionSpec)s, and a
    /// value-taking option (`source -encoding utf-8 f.tcl`) consumes its value
    /// word too.  The scan stops at the first word that is not a declared
    /// option, so the first *positional* is what remains — which is the only
    /// thing callers want to know.
    ///
    /// Matching is exact (`OptionSpec::matches`: canonical name or a declared
    /// alias), never the generic unique-prefix rule, for the same reason
    /// [`Self::namespace_leading_flag_words`] is exact: these are hand-parsed
    /// in C and an abbreviation is not accepted.
    ///
    /// `0` when no registry is attached (the analyser runs registry-less in
    /// some unit tests) or the command has no declared options — a flagless
    /// read then treats every word as positional, which is the pre-registry
    /// behaviour.
    fn leading_option_words(&self, cmd_name: &str, args: &[String]) -> usize {
        let Some(spec) = self.registry.as_deref().and_then(|r| r.get(cmd_name)) else {
            return 0;
        };
        let options = self
            .analysis_context()
            .context()
            .available_option_specs(spec);
        if options.is_empty() {
            return 0;
        }
        let mut consumed = 0usize;
        while let Some(word) = args.get(consumed) {
            let Some(opt) = options.iter().find(|o| o.matches(word.as_str())) else {
                break;
            };
            // A value-taking option eats the next word as well — but only when
            // that word is actually present, so a truncated `source -encoding`
            // does not report more words than were written.
            let width = 1 + usize::from(opt.takes_value());
            if consumed + width > args.len() {
                break;
            }
            consumed += width;
        }
        consumed
    }

    /// Record `source ?-encoding ENC? FILE` invocations.
    ///
    /// Which leading words are options — and therefore which word is the
    /// script path — comes from `source`'s own [`OptionSpec`
    /// list](tcl_registry::CommandSpec::options) via
    /// [`Self::leading_option_words`], not a `-encoding` literal here.  The
    /// option carries `SpecSurface::TCL85_PLUS` in the registry, so the
    /// dialect gating comes along for free: under an 8.4 profile — which has
    /// no `-encoding` — the word is not recognised as an option and the path
    /// word is read at index 0, matching that spec's own declaration rather
    /// than a hardcoded assumption here.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Source`].
    pub fn handle_source_command(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        let current_interpreter_scope = (0..=scope_path.len()).all(|depth| {
            super::super::scope::scope_at(&self.result.global_scope, &scope_path[..depth])
                .is_some_and(|scope| {
                    scope.original_interpreter_source_domain.is_none()
                        && !scope.is_independent_interpreter_source_body()
                })
        });
        let original_project_source_load = (load_trace_flag(
            "eligible.scope", None, current_interpreter_scope,
        ) && load_trace_flag("eligible.interpreter-stack", None, self.interp_path_stack.is_empty())
            && load_trace_flag("eligible.safe-stack", None, self.safe_interp_stack.is_empty()))
        .then(|| {
            load_trace_option("capture", None,
                crate::registry_invocation::source_structure::capture_original_project_source_load_at(
                    &self.source,
                    &self.result,
                    cmd_tok.span.start(),
                ),
            )
        })
        .flatten()
        .map(std::sync::Arc::new);
        let file_idx = original_project_source_load.as_ref().map_or_else(
            || self.leading_option_words("source", args),
            |load| load.path_argument(),
        );
        if file_idx >= args.len() || file_idx >= arg_tokens.len() {
            self.result
                .source_targets
                .push(crate::signature_scan::types::SignatureSource {
                    original_project_source_load: None,
                    original_interpreter_source_load: None,
                    raw_path: String::new(),
                    range: cmd_tok.span,
                    is_literal: false,
                    site_namespace: self.command_resolution_namespace(scope_path),
                });
            return;
        }
        let st = arg_tokens[file_idx];
        let original_project_source_load = original_project_source_load.filter(|load| {
            load_trace_flag(
                "path-span",
                None,
                load.path_word().is_some_and(|path| {
                    path.span() == st.span
                        || path
                            .tokens()
                            .first()
                            .is_some_and(|written| written.span == st.span)
                }),
            )
        });
        let path = &args[file_idx];
        let is_single = arg_single.get(file_idx).copied().unwrap_or(false);
        // A dynamic path — a bare `$var` or a concatenation like
        // `${dir}lit.tcl` — is first tried against the same last-write-wins
        // constant-string lattice already proven for `rename`'s OLD/NEW
        // words: the real corpus idiom is `set p
        // "e.tcl"; source $p`, a few lines apart in the same file, which
        // should resolve exactly like the literal `source e.tcl` it's
        // equivalent to — instead of going untracked and leaking the
        // sourced file's definitions into every caller as if unconditionally
        // global. A `[...]` command substitution
        // anywhere in the word (e.g. `[file join $dir lit.tcl]`) still
        // can't be folded this way — `resolve_dynamic_word` rejects it
        // outright, same as a variable whose value originates in a
        // different file — both stay conservatively dynamic, falling
        // through to `evaluate_auto_path_expr`'s narrower `[info script]`
        // subset unchanged.
        let (path, is_lit) = match self.resolve_dynamic_word(path, Some(st), is_single, scope_path)
        {
            Some(resolved) => {
                let is_lit = !crate::naming::is_dynamic_word(&resolved);
                (resolved, is_lit)
            }
            None => (path.clone(), false),
        };
        // `source` evaluates the file in the caller's current namespace:
        // record the command-resolution namespace at this call site so the
        // workspace index can re-home the sourced document's definitions.
        let site_namespace = self.command_resolution_namespace(scope_path);
        if let Some(source) = self
            .result
            .source_targets
            .iter_mut()
            .find(|source| source.range == st.span)
        {
            source.original_project_source_load = original_project_source_load;
            source.raw_path = path;
            source.is_literal = is_lit;
            source.site_namespace = site_namespace;
            return;
        }
        self.result
            .source_targets
            .push(crate::signature_scan::types::SignatureSource {
                original_project_source_load,
                raw_path: path,
                range: st.span,
                is_literal: is_lit,
                site_namespace,
                original_interpreter_source_load: None,
            });
    }

    /// Record a write to the package search path: `lappend auto_path
    /// PATH...` ([`AutoPathForm::Append`], one entry per word) or `set
    /// auto_path PATH` ([`AutoPathForm::Assign`], one entry for the list).
    ///
    /// The search path is the special variable the registry names
    /// `auto_path`, where the document's dialect provides it for user code to
    /// write ([`tcl_registry::VarAccess::ReadWrite`]) — one read of the
    /// special-variable registry, so a dialect with no auto-loader (iRules)
    /// records nothing. The check is on the written *variable*, not a
    /// command guard. Any such mutation flips ``has_dynamic_providers``:
    /// packages discovered at runtime can register commands the static
    /// analyser can't see, so W123 unknown-command diagnostics suppress on
    /// the document.
    ///
    /// `lappend` appends **one list element per argument word**, so an
    /// append records one entry per word and each is a whole directory —
    /// `lappend auto_path {p q}` names the single directory `p q`, not two.
    /// An assignment's right-hand side is a **list**, so its one record may
    /// name several directories; the split is deliberately *not* done here:
    /// the record keeps the source word verbatim (its `range` is the whole
    /// argument), and [`crate::auto_path_eval::evaluate_auto_path_entry`]
    /// applies the list grammar at consumption.
    ///
    /// `written` is the position of the written variable and `values` the
    /// words recorded. The assignment is recorded by
    /// [`Self::bind_value_word_assignment`] for any command whose declared
    /// semantics stores its value word into the variable it names; an
    /// append by the binder ([`Self::handle_var_binding_command`]) for any
    /// command whose descriptor appends list elements to the variable it
    /// writes ([`tcl_registry::VarElementsEffect::AppendsListElements`]).
    pub fn record_search_path_write(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        written: usize,
        values: std::ops::Range<usize>,
        form: super::super::types::AutoPathForm,
    ) {
        let Some(written) = args.get(written) else {
            return;
        };
        let context = self.analysis_context();
        let dialect = Some(context.context().authoring_query());
        if !context
            .commands()
            .special_var_in_dialect("auto_path", dialect)
            .is_some_and(|search_path| {
                search_path.access == tcl_registry::VarAccess::ReadWrite
                    && search_path.name == written
            })
        {
            return;
        }
        // An assignment with no value reads the path; it writes nothing. An
        // append of nothing still names the path as written.
        if values.is_empty() && form == super::super::types::AutoPathForm::Assign {
            return;
        }
        for i in values {
            let Some(tok) = arg_tokens.get(i) else {
                continue;
            };
            self.result
                .auto_path_entries
                .push(super::super::types::AutoPathEntry {
                    raw_path: args[i].clone(),
                    range: tok.span,
                    form,
                });
        }
        self.result.record_unresolved_provider_effect();
    }

    /// Record a `set auto_path PATH` mutation.
    ///
    /// Dispatched from the [`tcl_registry::hooks::AnalyserHookId::Set`]
    /// arm; see [`Self::handle_auto_path_lappend`] for why any
    /// ``auto_path`` mutation flips ``has_dynamic_providers``.
    ///
    /// The right-hand side is assigned as a **list**, so the one record this
    /// pushes may name several directories and is tagged
    /// [`AutoPathForm::Assign`] to say so.  The split is deliberately *not*
    /// done here: the record keeps the source word verbatim (its `range` is
    /// the whole argument), and
    /// [`crate::auto_path_eval::evaluate_auto_path_entry`] applies the list
    /// grammar at consumption.
    pub fn handle_auto_path_set(&mut self, args: &[String], arg_tokens: &[Token]) {
        // registry-axis-ok: special_vars — the auto_path special variable; until the special-variable table marks the package search path
        if args.first().map(String::as_str) != Some("auto_path") || args.len() < 2 {
            return;
        }
        if let Some(tok) = arg_tokens.get(1) {
            self.result
                .auto_path_entries
                .push(super::super::types::AutoPathEntry {
                    raw_path: args[1].clone(),
                    range: tok.span,
                    form: super::super::types::AutoPathForm::Assign,
                });
            self.result.record_unresolved_provider_effect();
        }
    }
}
