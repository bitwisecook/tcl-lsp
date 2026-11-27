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

//! The ensemble trampoline: the rewrite record, subcommand resolution, the target dispatch and the `-unknown` handler.

use super::super::{
    error_code_list, native_arguments, native_operation_currency, new_string, Code,
    EnsembleRewrite, EnsembleRewriteWord, Interp,
};
use super::CommandDispatchSelection;
use crate::namespace::{NsId, GLOBAL};
use crate::obj::TclObj;
use crate::{obj, parse};
use std::rc::Rc;
use tcl_registry::command_lookup::EnsembleDispatchDialect;

/// The outcome of an ensemble `-unknown` handler (`EnsembleUnknownCallback`).
pub(in super::super) enum EnsembleUnknown {
    /// A non-empty result: the replacement command prefix to dispatch.
    Prefix(obj::Owned),
    /// An empty result: the handler defined the subcommand — reparse the call.
    Reparse,
    /// The handler errored (or returned a bad code); `Code` carries the failure.
    Failed(Code),
}

impl Interp {
    /// Begin an ensemble-rewrite (a forward / ensemble / constructor replacing
    /// the original command words). Returns `true` if this is the *root* rewrite
    /// (no rewrite was active) — the caller must `clear_ensemble_rewrite` when
    /// its dispatch returns. Nested rewrites retain the root's original words
    /// and compose their removed/inserted counts.
    pub(crate) fn begin_ensemble_rewrite(
        &self,
        source: Vec<crate::obj::Owned>,
        removed: usize,
        inserted: usize,
    ) -> bool {
        self.begin_original_ensemble_rewrite(
            source.into_iter().map(EnsembleRewriteWord::Owned).collect(),
            removed,
            inserted,
        )
    }

    fn begin_original_ensemble_rewrite(
        &self,
        source: Vec<EnsembleRewriteWord>,
        removed: usize,
        inserted: usize,
    ) -> bool {
        let mut rw = self.ensemble_rewrite.borrow_mut();
        match rw.as_mut() {
            None => {
                *rw = Some(EnsembleRewrite {
                    birth: Rc::new(()),
                    source,
                    removed,
                    inserted,
                });
                true
            }
            // A nested rewrite chains onto the root (C's `TclInitRewriteEnsemble`):
            // the root `source` is kept, but its removed/inserted counts absorb the
            // inner step so a deeply forwarded `wrong # args` still prints the full
            // original prefix.
            Some(r) => {
                r.birth = Rc::new(());
                if r.inserted < removed {
                    r.removed += removed - r.inserted;
                    r.inserted = inserted;
                } else {
                    r.inserted = (r.inserted + inserted).saturating_sub(removed);
                }
                false
            }
        }
    }

    /// Clear the active ensemble-rewrite (paired with a root `begin_…`).
    pub(crate) fn clear_ensemble_rewrite_checked(
        &self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self);
        let before = match &scope {
            Ok(scope) => scope.currency().ensure_current_or_refuse(),
            Err(first) => Err(first.clone()),
        };
        let retired = self.ensemble_rewrite.borrow_mut().take();
        drop(retired);
        let after = match &scope {
            Ok(scope) => scope.currency().ensure_current_or_refuse(),
            Err(first) => Err(first.clone()),
        };
        before.and(after)
    }

    #[cfg(test)]
    pub(crate) fn clear_ensemble_rewrite(&self) {
        let _ = self.clear_ensemble_rewrite_checked();
    }

    /// The active ensemble-rewrite, if any.
    #[cfg(test)]
    pub(crate) fn ensemble_rewrite(&self) -> Option<EnsembleRewrite> {
        self.ensemble_rewrite.borrow().clone()
    }

    pub(in super::super::super) fn reset_native_ensemble_rewrite(
        &mut self,
        event: tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent,
    ) -> Result<(), Code> {
        use tcl_native_model::native_ensemble_rewrite::LogicalEnsembleRewriteProvider;
        let Some(protocol) = self
            .native_invocation_dialect()
            .ensemble_rewrite_protocol(Some(LogicalEnsembleRewriteProvider::Tcl84CoreSimulation))
        else {
            return Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "ensemble rewrite reset",
                ),
            ));
        };
        if protocol.resets_at(event) {
            self.clear_ensemble_rewrite_checked()
                .map_err(|first| self.refuse_native_execution(first))?;
        }
        Ok(())
    }

    fn ensemble_wrong_args_for_invocation(
        &mut self,
        argv: &[*mut TclObj],
        parameters: &[Vec<u8>],
    ) -> Code {
        let arguments = match native_arguments::OriginalArguments::enter(self, argv) {
            Ok(arguments) => arguments,
            Err(error) => return self.report_cmd_error(error),
        };
        let code = (|| {
            let Some(configuration) = self
                .native_invocation_dialect()
                .native_ensemble_configuration_protocol()
            else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "ensemble argument usage",
                    )
                    .into(),
                );
            };
            let name = match arguments.bytes(0) {
                Ok(name) => name,
                Err(error) => return self.report_cmd_error(error),
            };
            let mut message = b"wrong # args: should be \"".to_vec();
            message.extend_from_slice(&name);
            for parameter in parameters {
                message.push(b' ');
                crate::list::append_list_element(&mut message, parameter, false);
            }
            message.push(b' ');
            message.extend_from_slice(configuration.missing_selector_usage());
            message.push(b'"');
            self.wrong_arguments_message(&message)
        })();
        arguments.finish(code)
    }

    /// The ensemble trampoline: resolve `argv[1]` against the subcommand set
    /// (exact, then unambiguous prefix unless `-prefixes 0`), map it to a target
    /// command prefix (`-map`, else `<ns>::<sub>`), and re-dispatch
    /// `[target… , argv[2..]…]`. Mirrors C Tcl's `tclEnsemble.c` (the A3 contract).
    pub(in super::super) fn dispatch_ensemble(
        &mut self,
        token: &crate::ensemble::EnsembleToken,
        argv: &[*mut TclObj],
    ) -> Code {
        let arguments = match native_arguments::OriginalArguments::enter(self, argv) {
            Ok(arguments) => arguments,
            Err(error) => return self.report_cmd_error(error),
        };
        let code = self.dispatch_ensemble_with_original(token, argv, &arguments);
        arguments.finish(code)
    }

    fn dispatch_ensemble_with_original(
        &mut self,
        token: &crate::ensemble::EnsembleToken,
        argv: &[*mut TclObj],
        arguments: &native_arguments::OriginalArguments,
    ) -> Code {
        let mut current = token.config_shared();
        let mut reparsed = false;
        loop {
            // Re-read every structural field after an empty `-unknown` result:
            // the callback can reconfigure the ensemble before asking Tcl to
            // parse the same invocation again.
            let cfg: &crate::ensemble::EnsembleConfig = &current;
            let Some(layout) =
                tcl_cmd_core::ensemble::invocation_layout(argv.len(), 1, cfg.parameters.len())
            else {
                return self.ensemble_wrong_args_for_invocation(argv, &cfg.parameters);
            };
            let Some(configuration) = self
                .native_invocation_dialect()
                .native_ensemble_configuration_protocol()
            else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "ensemble member names",
                    )
                    .into(),
                );
            };
            let original_sub = match arguments.bytes(layout.subcommand) {
                Ok(original) => original,
                Err(error) => return self.report_cmd_error(error),
            };
            if token.is_deleted() {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original ensemble token retired",
                    )
                    .into(),
                );
            }
            let sub = configuration.member_name(&original_sub);
            let subs = self.ensemble_subcommands(cfg);
            if let Some(idx) = configuration.resolve_member(&subs, &original_sub, cfg.prefixes) {
                let resolved = &subs[idx];
                let built = self.build_original_ensemble_table(cfg);
                if let Err(error) = arguments.check() {
                    return self.report_cmd_error(error);
                }
                if let Err(error) = built {
                    return self.report_cmd_error(error.into());
                }
                let Some((prefix, mapped)) = cfg.originals.table.prefix(resolved) else {
                    return self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "ensemble original selected prefix",
                        )
                        .into(),
                    );
                };
                let prefix = obj::Owned::retain(prefix);
                let default_target = !mapped;
                // Spell-fix the subcommand to its resolved name in the recorded
                // source, so an abbreviated `ev` is reported as `event` (C's
                // `TclSpellFix`).
                let mut source: Vec<_> = argv
                    .iter()
                    .map(|&word| {
                        EnsembleRewriteWord::Borrowed(crate::obj::NativeObjectLifetime::retain(
                            word,
                        ))
                    })
                    .collect();
                source[layout.subcommand] =
                    EnsembleRewriteWord::Owned(crate::obj::Owned::fresh(new_string(resolved)));
                return self.dispatch_ensemble_target(
                    prefix,
                    cfg.ns,
                    argv,
                    &layout,
                    source,
                    default_target.then_some(resolved.as_slice()),
                );
            }
            // Miss: try the `-unknown` handler once.
            if !cfg.unknown.is_empty() && !reparsed {
                reparsed = true;
                let outcome = self.ensemble_unknown(token, cfg, argv);
                if let Err(error) = arguments.check() {
                    drop(outcome);
                    return self.report_cmd_error(error);
                }
                match outcome {
                    EnsembleUnknown::Prefix(prefix) => {
                        let live = token.config();
                        let Some(live_layout) = tcl_cmd_core::ensemble::invocation_layout(
                            argv.len(),
                            1,
                            live.parameters.len(),
                        ) else {
                            return self.ensemble_wrong_args_for_invocation(argv, &live.parameters);
                        };
                        let source: Vec<_> = argv
                            .iter()
                            .map(|&word| {
                                EnsembleRewriteWord::Borrowed(
                                    crate::obj::NativeObjectLifetime::retain(word),
                                )
                            })
                            .collect();
                        return self.dispatch_ensemble_target(
                            prefix,
                            live.ns,
                            argv,
                            &live_layout,
                            source,
                            None,
                        );
                    }
                    EnsembleUnknown::Reparse => {
                        current = token.config_shared();
                        continue;
                    }
                    EnsembleUnknown::Failed(code) => return code,
                }
            }
            // A namespace ensemble with no subcommands at all gets a distinct
            // message; otherwise "unknown or ambiguous" (prefixes on) / "unknown"
            // (prefixes off) followed by the candidate list (C's
            // `NsEnsembleImplementationCmdNR`).
            let ecode = error_code_list(&[b"TCL", b"LOOKUP", b"SUBCOMMAND", sub]);
            let ns_fqn = self.namespaces.borrow().qualified_name(cfg.ns);
            let m = tcl_cmd_core::ensemble::unknown_subcommand_message(
                &subs,
                sub,
                cfg.prefixes,
                &ns_fqn,
            );
            return self.error_with_code(&m, &ecode);
        }
    }

    /// The ensemble's valid subcommand set (sorted, deduped): explicit
    /// `-subcommands`, else the `-map` keys, else the namespace's exports.
    fn ensemble_subcommands(&self, cfg: &crate::ensemble::EnsembleConfig) -> Vec<Vec<u8>> {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_ensemble_configuration_protocol()
        else {
            return Vec::new();
        };
        let keys: Vec<_> = cfg
            .map
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|(key, _)| key.as_slice())
            .collect();
        let explicit = cfg
            .subcommands
            .as_ref()
            .map(|names| names.iter().map(Vec::as_slice).collect::<Vec<_>>());
        let exports = self.namespaces.borrow().exported_commands(cfg.ns);
        let exports: Vec<_> = exports.iter().map(Vec::as_slice).collect();
        let map = crate::ensemble::NativeEnsembleObjects::pointer(&cfg.originals.map);
        let same = map.is_some()
            && map == crate::ensemble::NativeEnsembleObjects::pointer(&cfg.originals.subcommands);
        protocol
            .table_plan(explicit.as_deref(), &keys, &exports, same)
            .entries
            .into_iter()
            .map(|entry| entry.member)
            .collect()
    }

    /// Dispatch a resolved ensemble subcommand: `[prefix…, params…, rest…]` (the
    /// `-parameters` values thread in right after the target prefix; the
    /// subcommand's own args follow). `layout` is always computed by the shared
    /// ensemble owner from the live token configuration.
    fn dispatch_ensemble_target(
        &mut self,
        prefix: obj::Owned,
        namespace: NsId,
        argv: &[*mut TclObj],
        layout: &tcl_cmd_core::ensemble::InvocationLayout,
        source: Vec<EnsembleRewriteWord>,
        default_name: Option<&[u8]>,
    ) -> Code {
        let mut retained = argv.to_vec();
        retained.push(prefix.as_ptr());
        let originals = match native_arguments::OriginalArguments::enter(self, &retained) {
            Ok(originals) => originals,
            Err(error) => return self.report_cmd_error(error),
        };
        let code = (|| {
            let Some(dispatch) = self
                .native_invocation_dialect()
                .native_ensemble_dispatch_protocol()
            else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native ensemble forwarding",
                    )
                    .into(),
                );
            };
            let Some(string) = self
                .eval_frame_dialect()
                .native_string_materialization(None)
            else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native ensemble invocation List",
                    )
                    .into(),
                );
            };
            let protocol = string.protocol();
            let members = match crate::list::list_elements_native_with_currency(
                prefix.as_ptr(),
                protocol,
                Some(originals.currency()),
            ) {
                Ok(members) => members,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            if let Err(error) = originals.check() {
                return self.report_cmd_error(error);
            }
            let prefix_len = members.len();
            let invocation = if dispatch.always_copy_prefix || argv.len() == 2 {
                match crate::list::native_list_copy(prefix.as_ptr(), protocol) {
                    Ok(copy) => copy,
                    Err(error) => return self.report_cmd_error(error.into()),
                }
            } else {
                let mut words = members;
                words.extend_from_slice(&argv[layout.parameters.clone()]);
                words.extend_from_slice(&argv[layout.arguments..]);
                obj::Owned::fresh(crate::list::new_list_obj_native(&words, protocol))
            };
            let mut words = match crate::list::list_elements_native_with_currency(
                invocation.as_ptr(),
                protocol,
                Some(originals.currency()),
            ) {
                Ok(words) => words,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            if let Err(error) = originals.check() {
                return self.report_cmd_error(error);
            }
            if dispatch.always_copy_prefix {
                words.extend_from_slice(&argv[layout.parameters.clone()]);
                words.extend_from_slice(&argv[layout.arguments..]);
            }
            let prefix = if dispatch.always_copy_prefix {
                Some(prefix)
            } else {
                drop(prefix);
                None
            };
            if let Err(error) = originals.check() {
                return self.report_cmd_error(error);
            }
            let lookup = if dispatch.use_ensemble_namespace {
                namespace
            } else {
                GLOBAL
            };
            let selected_words = match native_arguments::OriginalArguments::enter(self, &words) {
                Ok(arguments) => arguments,
                Err(error) => return self.report_cmd_error(error),
            };
            let default_head = if default_name.is_some() {
                match selected_words.bytes(0) {
                    Ok(name) => Some(name),
                    Err(error) => return self.report_cmd_error(error),
                }
            } else {
                None
            };
            let default_target_was_missing = default_head.as_ref().is_some_and(|name| {
                self.resolve_dispatchable_with_generation(lookup, name)
                    .is_none()
            });
            if let Err(error) = originals.check() {
                return self.report_cmd_error(error);
            }
            let root = self.begin_original_ensemble_rewrite(
                source,
                layout.arguments,
                prefix_len + layout.parameters.len(),
            );
            let code = self.dispatch_selection_with_entry(
                &words,
                CommandDispatchSelection::LookupAt(
                    lookup,
                    tcl_registry::command_lookup::CommandLookupOrigin::EnsembleInvocation,
                ),
                false,
            );
            let current = originals.check().and_then(|()| selected_words.check());
            if current.is_ok()
                && code == Code::Error
                && default_target_was_missing
                && dispatch.use_ensemble_namespace
            {
                if let Some(name) = default_name {
                    let reached_head = selected_words.bytes(0);
                    if let Err(error) = originals
                        .check_for(self)
                        .and_then(|()| selected_words.check_for(self))
                    {
                        if root {
                            let _ = self.clear_ensemble_rewrite_checked();
                        }
                        return self.report_cmd_error(error);
                    }
                    let actual_head = match reached_head {
                        Ok(head) => head,
                        Err(error) => {
                            if root {
                                let _ = self.clear_ensemble_rewrite_checked();
                            }
                            return self.report_cmd_error(error);
                        }
                    };
                    let expected = [
                        b"invalid command name \"".as_slice(),
                        actual_head.as_ref(),
                        b"\"",
                    ]
                    .concat();
                    let result = self.get_obj_result();
                    let selected_result =
                        match native_arguments::OriginalArguments::enter(self, &[result]) {
                            Ok(arguments) => arguments,
                            Err(error) => {
                                if root {
                                    let _ = self.clear_ensemble_rewrite_checked();
                                }
                                return self.report_cmd_error(error);
                            }
                        };
                    let presented = selected_result.bytes(0);
                    if self.result.get() != result {
                        self.refuse_completion_value_access(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original ensemble error result replaced",
                            ),
                        );
                    }
                    if originals
                        .check()
                        .and_then(|()| selected_words.check())
                        .is_ok()
                    {
                        match presented {
                            Ok(presented) if presented.as_ref() == expected => {
                                let message =
                                    [b"invalid command name \"".as_slice(), name, b"\""].concat();
                                self.set_result_bytes(&message);
                            }
                            Ok(_) => {}
                            Err(error) => {
                                self.report_cmd_error(error);
                            }
                        }
                    }
                }
            }
            let cleared = if root {
                self.clear_ensemble_rewrite_checked()
            } else {
                Ok(())
            };
            drop(invocation);
            drop(prefix);
            if let Err(first) = cleared {
                self.refuse_native_execution(first);
            }
            code
        })();
        originals.finish(code)
    }

    /// Invoke an ensemble's `-unknown` handler on a subcommand miss
    /// (`EnsembleUnknownCallback`): `handler… ensembleFQN argv[1..]…`. An empty
    /// `TCL_OK` result asks for a reparse (the handler defined the subcommand); a
    /// non-empty one is the replacement command prefix; anything else fails.
    pub(in super::super) fn ensemble_unknown(
        &mut self,
        token: &crate::ensemble::EnsembleToken,
        cfg: &crate::ensemble::EnsembleConfig,
        argv: &[*mut TclObj],
    ) -> EnsembleUnknown {
        let Some(handler) = crate::ensemble::NativeEnsembleObjects::pointer(&cfg.originals.unknown)
        else {
            return EnsembleUnknown::Failed(
                self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "ensemble unknown configured root",
                    )
                    .into(),
                ),
            );
        };
        let mut retained = argv.to_vec();
        retained.push(handler);
        let originals = match native_arguments::OriginalArguments::enter(self, &retained) {
            Ok(originals) => originals,
            Err(error) => return EnsembleUnknown::Failed(self.report_cmd_error(error)),
        };
        let outcome = (|| {
            let ens_fqn = token.name();
            let Some(protocol) = self
                .eval_frame_dialect()
                .native_string_materialization(None)
                .map(|recipe| recipe.protocol())
            else {
                return EnsembleUnknown::Failed(
                    self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "ensemble unknown original prefix",
                        )
                        .into(),
                    ),
                );
            };
            if let Err(error) = originals
                .check_original_for(self, argv.len(), handler)
                .and_then(|()| originals.original(argv.len()).map(|_| ()))
            {
                return EnsembleUnknown::Failed(self.report_cmd_error(error));
            }
            let call = obj::Owned::fresh(obj::duplicate(handler));
            if let Err(error) = originals
                .check_original_for(self, argv.len(), handler)
                .and_then(|()| originals.original(argv.len()).map(|_| ()))
            {
                return EnsembleUnknown::Failed(self.report_cmd_error(error));
            }
            let selected_call =
                match native_arguments::OriginalArguments::enter(self, &[call.as_ptr()]) {
                    Ok(originals) => originals,
                    Err(error) => return EnsembleUnknown::Failed(self.report_cmd_error(error)),
                };
            let fqn = obj::Owned::fresh(new_string(&ens_fqn));
            let mut additions = vec![fqn.as_ptr()];
            additions.extend_from_slice(&argv[1..]);
            let checked = || {
                originals
                    .check_for(self)
                    .and_then(|()| selected_call.check_original_for(self, 0, call.as_ptr()))
            };
            let prepared = crate::list::append_prepared_native_elements_using(
                call.as_ptr(),
                &additions,
                protocol,
                &checked,
                &|error| originals.retain_refusal(error.into()),
            );
            if let Err(error) = checked() {
                return EnsembleUnknown::Failed(self.report_cmd_error(error));
            }
            if let Err(error) = prepared {
                return EnsembleUnknown::Failed(self.report_cmd_error(error));
            }
            drop(fqn);
            if let Err(error) = originals
                .check_for(self)
                .and_then(|()| selected_call.check_original_for(self, 0, call.as_ptr()))
            {
                return EnsembleUnknown::Failed(self.report_cmd_error(error));
            }
            let checked = || {
                originals
                    .check_for(self)
                    .and_then(|()| selected_call.check_original_for(self, 0, call.as_ptr()))
            };
            let reached = crate::list::list_elements_native_using(
                call.as_ptr(),
                protocol,
                &checked,
                &|error| originals.retain_refusal(error.into()),
            );
            if let Err(error) = checked() {
                return EnsembleUnknown::Failed(self.report_cmd_error(error));
            }
            let hv = match reached {
                Ok(words) => words,
                Err(error) => return EnsembleUnknown::Failed(self.report_cmd_error(error)),
            };
            let selected_words = match native_arguments::OriginalArguments::enter(self, &hv) {
                Ok(originals) => originals,
                Err(error) => return EnsembleUnknown::Failed(self.report_cmd_error(error)),
            };
            let mut handler_call = Vec::new();
            for index in 0..hv.len() {
                let reached = selected_words.bytes(index);
                if let Err(error) = originals
                    .check_for(self)
                    .and_then(|()| selected_words.check_for(self))
                {
                    return EnsembleUnknown::Failed(self.report_cmd_error(error));
                }
                let word = match reached {
                    Ok(word) => word,
                    Err(error) => return EnsembleUnknown::Failed(self.report_cmd_error(error)),
                };
                if index != 0 {
                    handler_call.push(b' ');
                }
                crate::list::append_list_element(&mut handler_call, &word, false);
            }
            let code = self.dispatch(&hv);
            if let Err(error) = originals
                .check_for(self)
                .and_then(|()| selected_words.check_for(self))
            {
                return EnsembleUnknown::Failed(self.report_cmd_error(error));
            }
            match code {
                Code::Ok => {
                    if token.is_deleted() {
                        let code = self.error_with_code(
                            tcl_cmd_core::ensemble::UNKNOWN_DELETED_MESSAGE.as_bytes(),
                            tcl_cmd_core::ensemble::UNKNOWN_DELETED_ERROR_CODE.as_bytes(),
                        );
                        self.append_frame_noline(b"ensemble unknown subcommand handler");
                        return EnsembleUnknown::Failed(code);
                    }
                    let result = obj::Owned::retain(self.get_obj_result());
                    let selected_result = match native_arguments::OriginalArguments::enter(
                        self,
                        &[result.as_ptr()],
                    ) {
                        Ok(originals) => originals,
                        Err(error) => return EnsembleUnknown::Failed(self.report_cmd_error(error)),
                    };
                    self.set_result_bytes(b"");
                    drop(call);
                    if let Err(error) = originals
                        .check_for(self)
                        .and_then(|()| selected_result.check_original_for(self, 0, result.as_ptr()))
                    {
                        return EnsembleUnknown::Failed(self.report_cmd_error(error));
                    }
                    let checked = || {
                        originals.check_for(self).and_then(|()| {
                            selected_result.check_original_for(self, 0, result.as_ptr())
                        })
                    };
                    let parsed = crate::list::list_elements_native_using(
                        result.as_ptr(),
                        protocol,
                        &checked,
                        &|error| originals.retain_refusal(error.into()),
                    );
                    if let Err(error) = checked() {
                        return EnsembleUnknown::Failed(self.report_cmd_error(error));
                    }
                    match parsed {
                        Ok(words) if !words.is_empty() => EnsembleUnknown::Prefix(result),
                        Ok(_) => EnsembleUnknown::Reparse,
                        Err(error) => {
                            let code = self.report_cmd_error(error);
                            self.append_error_info_context(
                                b"while parsing result of ensemble unknown subcommand handler",
                            );
                            EnsembleUnknown::Failed(code)
                        }
                    }
                }
                Code::Error => {
                    if let Some(command) =
                        parse::parse_script_with_config(&handler_call, self.lexer_config()).first()
                    {
                        self.log_command_info(&handler_call, command);
                    }
                    self.append_frame_noline(b"ensemble unknown subcommand handler");
                    EnsembleUnknown::Failed(Code::Error)
                }
                other => {
                    let mut message = b"unknown subcommand handler returned bad code: ".to_vec();
                    match other {
                        Code::Return => message.extend_from_slice(b"return"),
                        Code::Break => message.extend_from_slice(b"break"),
                        Code::Continue => message.extend_from_slice(b"continue"),
                        Code::Other(value) => {
                            message.extend_from_slice(value.to_string().as_bytes())
                        }
                        Code::Ok | Code::Error => unreachable!("checked above"),
                    }
                    let code = self.error_with_code(&message, b"TCL ENSEMBLE UNKNOWN_RESULT");
                    let mut context = b"result of ensemble unknown subcommand handler: ".to_vec();
                    context.extend_from_slice(&handler_call);
                    self.append_error_info_context(&context);
                    EnsembleUnknown::Failed(code)
                }
            }
        })();
        if let Err(error) = originals.check_for(self) {
            drop(outcome);
            let _ = originals.check_for(self);
            return EnsembleUnknown::Failed(self.report_cmd_error(error));
        }
        outcome
    }
}
