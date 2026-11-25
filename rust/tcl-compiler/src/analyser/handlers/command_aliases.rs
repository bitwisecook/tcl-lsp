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

//! Commands reachable under another name: `interp alias` and `rename`.

use super::super::state::Analyser;
use crate::alias::{command_table_transitions, is_current_interpreter};
use crate::signature_scan::types::SignatureCommandAlias;
use tcl_lexer::Token;

impl Analyser {
    /// Handle `interp alias {} ALIAS {} TARGET ?ARG ...?` —
    /// records the alias for later argument-role resolution.
    ///
    /// Reads the registry's one command-table transition vocabulary
    /// (`CommandBindingTransition`) rather than destructuring
    /// `interp alias`'s word layout here. `offset` is the command token's
    /// start, recorded in [`Analyser::alias_offsets`] for the same-file
    /// arity resolver's top-level order gate.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::InterpAlias`] (stamped on
    /// `interp`'s `alias` subcommand); `args[0]` is still the subcommand
    /// word, which is what selects the alias descriptor.
    pub fn handle_interp_alias(&mut self, args: &[String], scope_path: &[usize], offset: u32) {
        self.handle_interp_alias_with_original_arguments(args, &[], scope_path, offset);
    }

    pub(in super::super) fn handle_interp_alias_with_original_arguments(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        _scope_path: &[usize],
        offset: u32,
    ) {
        // What this call did to the command table is registry data, read
        // through the one transition vocabulary. The
        // interpreter *paths* are not — a path may be a tracked
        // `set VAR [interp create …]` binding this analyser resolves
        // itself — so the cross-domain arm below still reads the path
        // words; the alias and target *command* names come from the facts.
        let registry = self.registry.as_deref();
        let facts: Vec<tcl_registry::CommandBindingTransition> =
            registry.map_or_else(Vec::new, |registry| {
                command_table_transitions(registry, "interp", args)
                    .command_bindings()
                    .cloned()
                    .collect()
            });
        if let Some(tcl_registry::CommandBindingTransition::Delete { interpreter, name }) =
            facts.first()
            && interpreter.as_ref().is_none_or(is_current_interpreter)
            && let Some(deleted) = name.literal()
        {
            // Deleting an alias destroys the command object, so every
            // `namespace import` edge pointing at it dies too.
            // An empty srcPath means the interpreter this command *runs in*,
            // so inside a child's eval body the deletion homes under that
            // child's domain rather than the parent's command table.
            let deleted = format!(
                "{}{}",
                self.current_interp_domain_prefix(),
                crate::naming::normalise_qualified_name(deleted)
            );
            self.result
                .destroyed_commands
                .insert(deleted.clone(), offset);
            self.deleted_commands.insert(deleted, offset);
            return;
        }
        let Some(tcl_registry::CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        }) = facts.first()
        else {
            return;
        };
        if *target_lookup != tcl_registry::AliasTargetLookup::Global {
            // Caller-dependent aliases are represented by the source kernel.
            // This legacy global-only presentation cannot invent a target.
            return;
        }
        // The `interp alias {} A {} B` fast path homes both names at the
        // global root, which is only right in the main interpreter: inside a
        // child's eval body `{}` names *that child*. There, fall through to
        // the domain-aware branch below, which qualifies both sides through
        // `resolve_alias_domain_prefix` exactly as the explicit-path form
        // already does.
        if self.interp_path_stack.is_empty()
            && is_current_interpreter(source_interpreter)
            && is_current_interpreter(target_interpreter)
            && let (Some(alias_name), Some(target_cmd)) = (alias.literal(), target.literal())
        {
            let qualified = if alias_name.is_empty() {
                String::new()
            } else {
                crate::naming::normalise_qualified_name(alias_name)
            };
            let prepended = arguments
                .iter()
                .filter_map(tcl_registry::TransitionSubject::literal)
                .map(str::to_owned)
                .collect();
            self.record_interp_alias(
                qualified,
                self.registry
                    .as_deref()
                    .and_then(tcl_registry::CommandRegistry::profile)
                    .and_then(|profile| {
                        tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
                    })
                    .and_then(|policy| {
                        crate::signature_scan::scope::SignatureSourceCommand::alias_at_root(
                            policy, alias_name,
                        )
                    }),
                crate::signature_scan::types::SignatureCommandAliasTarget::WrittenGlobal(
                    target_cmd.to_owned(),
                ),
                prepended,
                offset,
            );
            return;
        }
        // Cross-domain alias: `interp alias PATH name
        // TPATH target ?arg…?` deliberately crosses interpreter domains —
        // the alias `name` becomes callable in the *source* interpreter,
        // running `target` resolved in the *target* interpreter.  With
        // literal paths both sides home under their `@interp@` domains, so
        // calls of the alias inside the child's eval bodies resolve to the
        // target through the ordinary alias link machinery. A path need
        // not itself be a source literal: it also
        // resolves through a tracked `set VAR [interp create ...]`
        // binding — only the alias/target *command* names stay hard
        // literal requirements, which is exactly what the registry's typed
        // subjects already say.
        let (Some(alias_name), Some(target_cmd)) = (alias.literal(), target.literal()) else {
            return;
        };
        let (Some(src_path), Some(target_path)) = (args.get(1), args.get(3)) else {
            return;
        };
        if let Some(src_prefix) =
            self.resolve_alias_domain_prefix(src_path, arg_tokens.get(1).copied())
            && let Some(target_prefix) =
                self.resolve_alias_domain_prefix(target_path, arg_tokens.get(3).copied())
            && !(src_prefix.is_empty() && target_prefix.is_empty())
        {
            let qualified =
                crate::signature_scan::types::SignatureCommandAliasTarget::in_interpreter(
                    &src_prefix,
                    alias_name,
                )
                .constructed_key()
                .into_owned();
            let target = crate::signature_scan::types::SignatureCommandAliasTarget::in_interpreter(
                &target_prefix,
                target_cmd,
            );
            let prepended: Vec<String> = args[5..].to_vec();
            self.record_interp_alias(qualified, None, target, prepended, offset);
        }
    }

    /// Record one resolved `interp alias` fact into the three alias
    /// tables (shared by the current-interp and cross-domain forms).
    fn record_interp_alias(
        &mut self,
        qualified: String,
        source_name: Option<crate::signature_scan::scope::SignatureSourceCommand>,
        target_cmd: crate::signature_scan::types::SignatureCommandAliasTarget,
        prepended: Vec<String>,
        offset: u32,
    ) {
        // Binding a second name onto `source` (or another external-unit
        // loader) means calls through that name never reach the `Source` hook,
        // so the files they pull in are invisible — see
        // [`Self::note_external_unit_command_moved`].
        let policy = self
            .registry
            .as_deref()
            .and_then(tcl_registry::CommandRegistry::profile)
            .and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
            });
        self.command_aliases.remove(&qualified);
        if let Some(target_key) = target_cmd
            .checked_global_key(policy)
            .map(std::borrow::Cow::into_owned)
        {
            self.note_external_unit_command_moved(&target_key);
            self.command_aliases
                .insert(qualified.clone(), (target_key, prepended.clone()));
        }
        self.alias_offsets.insert(qualified.clone(), offset);
        self.result.alias_offsets.insert(qualified.clone(), offset);
        self.result.command_aliases.insert(
            qualified.clone(),
            SignatureCommandAlias {
                source_name,
                qualified_name: qualified,
                target: target_cmd,
                extras: prepended,
            },
        );
    }

    /// Handle `rename OLD NEW` — record a static rename so calls to
    /// `NEW` resolve to whatever `OLD` denoted (the same proc, unchanged
    /// signature — a rename is a pure name move, never an arity change).
    /// Also records that `OLD` itself is no longer a callable command
    /// from this point on (confirmed against tclsh 9.0.4: `OLD` fails
    /// "invalid command name" afterwards, not a "wrong # args" against
    /// its original signature).
    ///
    /// `OLD`/`NEW` need not themselves be literal words: each is first
    /// run through [`Self::resolve_dynamic_word`], which also resolves a
    /// constant-foldable dynamic word (`rename $old ::new`, `set key
    /// impl; rename ::foo_$key ::foo`) — a bare variable read of a
    /// runtime value (`rename [somecommand] ::new`) or one that folding
    /// can't pin down still can't be, and only *that* residual case
    /// returns `true` (dynamic) — the caller widens
    /// `has_dynamic_providers` then, the same wildcard-collapse
    /// convention `command_binding.rs`'s flow-sensitive lattice uses for
    /// the identical shape. A malformed `rename` (wrong argument count,
    /// already flagged by the registry arity check) is not treated as
    /// dynamic — there is no new binding to widen for. `offset` is the
    /// command token's start, recorded in [`Analyser::rename_offsets`] /
    /// [`Analyser::deleted_commands`] for the same-file arity resolver's
    /// top-level order gate. A deleting `rename OLD {}` records only
    /// `OLD`'s deletion (confirmed against tclsh 9.0.4: also "invalid
    /// command name" afterwards) — there is no `NEW` to map it to.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Rename`].
    pub fn handle_rename(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        offset: u32,
    ) -> bool {
        if args.len() != 2 {
            return false;
        }
        let Some(old_resolved) = self.resolve_dynamic_word(
            &args[0],
            arg_tokens.first().copied(),
            arg_single.first().copied().unwrap_or(false),
            scope_path,
        ) else {
            return true;
        };
        let old = crate::naming::normalise_qualified_name(&old_resolved);
        if old.is_empty() {
            return false;
        }
        // Required-name navigation and advice use the post-argv lookup site.
        // Recording the source operand does not require the destination to be
        // known, and does not prove that the rename itself succeeds.
        if let Some(tok) = arg_tokens.first() {
            self.push_command_reference_with_policy(
                args[0].clone(),
                tok.span,
                old.clone(),
                None,
                false,
                crate::signature_scan::types::SignatureCommandLookup::ConsumedName {
                    invocation_offset: offset,
                },
            );
        }
        let Some(new_resolved) = self.resolve_dynamic_word(
            &args[1],
            arg_tokens.get(1).copied(),
            arg_single.get(1).copied().unwrap_or(false),
            scope_path,
        ) else {
            return true;
        };
        let new = crate::naming::normalise_qualified_name(&new_resolved);
        // Moving an external-unit loader takes its loading out of static view.
        self.note_external_unit_command_moved(&old);
        // `OLD`'s own deletion must not appear to have already happened *at*
        // the reference just pushed above for it — `deleted_commands` is
        // keyed by a single load-order offset compared with `>=` (see
        // `registry_name_deleted_before`), and `OLD`'s token always sits
        // textually after the `rename` command's own start (`offset`), so
        // recording the deletion at `offset` would make that offset compare
        // as "at or after" a call site that is really the deletion's own
        // trigger — wrongly W123-flagging `rename puts myputs`'s own `puts`
        // as a call to an already-dead builtin. Anchoring the deletion to
        // just past `OLD`'s own token keeps every *other*, later call site's
        // ordering unaffected (nothing else can sit between `OLD`'s token
        // and here) while excluding this one.
        let deletion_offset = arg_tokens.first().map_or(offset, |t| t.span.end());
        // A rename moves whatever real Tcl object `OLD` currently denotes —
        // including a live interpreter's own handle command — to `NEW` (or,
        // for a deleting `rename OLD {}`, off the command table entirely).
        // Keep `self.interpreters` in step so a later, unrelated definition
        // that reuses `OLD`'s freed name is never misidentified as still
        // being that interpreter's handle (confirmed against tclsh 9.0.4:
        // after `rename sandbox {}` the child interpreter is gone —
        // `interp slaves` no longer lists it — and a later `proc sandbox
        // {sub body} {...}` dispatches to the new proc, never to the
        // interpreter; `rename sandbox t` instead keeps the same
        // interpreter reachable, now only as `t`). Bumping the epoch on
        // both the vacated key and any interpreter the rename overwrites at
        // `NEW` mirrors `handle_interp_delete_command`, so a later
        // `interp create` recreating either name never merges with the
        // interpreter tracked under that name before.
        // Keyed off the *resolved* text (identical to `args[N]` for an
        // already-static rename) so a resolvable dynamic handle
        // (`set h sandbox; rename $h moved`) migrates the tracked state
        // too, not just the ones spelled out literally.
        let old_interp_key = self.qualified_interp_key(&old_resolved);
        if let Some(state) = self.interpreters.remove(&old_interp_key) {
            *self.interp_epochs.entry(old_interp_key).or_insert(0) += 1;
            if !new.is_empty() {
                let new_interp_key = self.qualified_interp_key(&new_resolved);
                if self.interpreters.remove(&new_interp_key).is_some() {
                    *self
                        .interp_epochs
                        .entry(new_interp_key.clone())
                        .or_insert(0) += 1;
                }
                self.interpreters.insert(new_interp_key, state);
            }
        }
        // Child-interpreter renames cannot mutate the parent's command facts.
        if !self.interp_path_stack.is_empty() {
            return false;
        }
        // A rename nested inside a control-flow body may not run at all, so it
        // is not evidence that `OLD` is gone. Recording it anyway draws a
        // W123 on a command that is demonstrably still callable, and — since
        // class liveness consults the same facts — withdraws the
        // W308 on objects of that class too.
        //
        // Oracle (tclsh8.6): with `if {0} { rename
        // Dog {} }`, `Dog new` succeeds and `$d fly` fails with `unknown
        // method "fly"`. The analyser proves the same branch dead in the same
        // run — it emits I230 on it — so honouring the deletion would
        // contradict its own verdict.
        //
        // Only straight-line deletions count. This is the *syntactic* rule:
        // `if {1} { rename Dog {} }` is equally not recorded, even though it
        // does run. Reading the branch's real value needs SCCP, which runs
        // later over the IR this walk feeds, so it is not available here. The
        // narrower rule errs towards treating commands as live, matching the
        // existing "a deletion inside a definition body does not count"
        // rule in `fact_live_for_call`.
        if self.control_flow_body_depth > 0 {
            if !new.is_empty() {
                self.renamed_commands.insert(new.clone(), old.clone());
                self.rename_offsets.insert(new.clone(), offset);
                self.result.rename_offsets.insert(new.clone(), offset);
                self.result.renamed_commands.insert(new, old);
            }
            return false;
        }
        if new.is_empty() {
            // `rename OLD {}` destroys the command object — unlike `rename OLD
            // NEW`, which hands it over and leaves every import edge alive.
            self.result
                .destroyed_commands
                .insert(old.clone(), deletion_offset);
            self.deleted_commands.insert(old, deletion_offset);
            return false;
        }
        self.renamed_commands.insert(new.clone(), old.clone());
        self.rename_offsets.insert(new.clone(), offset);
        self.deleted_commands.insert(old.clone(), deletion_offset);
        self.result.rename_offsets.insert(new.clone(), offset);
        self.result.renamed_commands.insert(new, old);
        false
    }

    /// Note that the command whose name is being moved/bound loads an external
    /// unit — a `source`, a `load`, an `auto_load` — and, if so, widen
    /// [`AnalysisResult::has_dynamic_providers`](super::super::types::AnalysisResult::has_dynamic_providers).
    ///
    /// Hook dispatch keys off the **written** head, so once `source` has been
    /// renamed or aliased to another name, calls through that name no longer
    /// reach [`Self::handle_source_command`] and the files they pull in become
    /// invisible. Rather than let W120 / W123 then confidently report a package
    /// or command "missing" that the moved command loads, this widens to the
    /// same unknowable state a dynamic `namespace import` pattern or a
    /// `namespace unknown` handler produces.
    ///
    /// The test is [`Traits::LOADS_EXTERNAL_UNIT`](tcl_registry::Traits), read
    /// off the spec: no command name appears here, so `load` and `auto_load`
    /// are covered by the same code that covers `source`, and a dialect that
    /// adds another file-loading command is covered by declaring the trait.
    fn note_external_unit_command_moved(&mut self, name: &str) {
        if self
            .registry
            .as_deref()
            .and_then(|r| r.get(name))
            .is_some_and(|spec| {
                spec.traits
                    .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
            })
        {
            self.result.record_unresolved_provider_effect();
        }
    }
}
