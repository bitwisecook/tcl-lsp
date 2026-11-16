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

//! Resolving the registry's inline codegen hooks and the command bindings an inline specialisation retains.

use super::super::{CodegenCtx, Op};
use super::{
    NestedLocalName, names_builtin, nested_local_name_value, source_direct_local_name_value,
};
use tcl_compiler::ir::CommandTokens;
use tcl_registry::hooks::InlineCodegenHookId;

impl CodegenCtx<'_> {
    /// Wrapping metadata follows the same actual selected invocation as hooks.
    pub(in super::super) fn selected_invocation_traits(
        &self,
        command: &str,
        args: &[(String, bool)],
    ) -> Option<tcl_registry::Traits> {
        if !self.invocation_specialisation_proved() {
            return None;
        }
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens.source_binding.is_some()
        {
            if !tokens.words_align_with_argv_text()
                || tokens.argv_texts.first().is_none_or(|head| head != command)
                || tokens.argv_texts.len() != args.len().checked_add(1)?
                || tokens
                    .argv_texts
                    .iter()
                    .skip(1)
                    .zip(args)
                    .any(|(original, (text, _))| original != text)
            {
                return None;
            }
            return Some(
                tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                    self.registry,
                    None,
                    tokens,
                )?
                .traits(),
            );
        }
        let arguments = args
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>();
        let selected =
            self.registry
                .resolve_call(command, &arguments, self.invocation_surface_query())?;
        names_builtin(command, selected.spec.name).then_some(selected.spec.traits)
    }

    /// Resolve the registry-stamped [`InlineCodegenHookId`] for a
    /// command-substitution head word, mirroring how
    /// `emitter::bytecoded::try_bytecoded` resolves statement-position
    /// hooks (`resolve_call`, so a subcommand-keyed hook such as
    /// `dict get` / `info exists` wins over the command level).
    ///
    /// The head must name the builtin itself ([`names_builtin`]): its own
    /// spelling or the global one (`[::expr …]`), which a release compiles
    /// alike. Any other qualified spelling stays on the generic-invoke path.
    pub(crate) fn inline_cmd_subst_hook(
        &mut self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<InlineCodegenHookId> {
        self.inline_cmd_subst_resolution(cmd, args)
            .map(|(hook, _binding)| hook)
    }

    /// Read a registry hook without retaining a command-binding dependency.
    ///
    /// This only gates a later source-form decision. The caller that actually
    /// emits a specialised opcode must enter through
    /// [`Self::inline_cmd_subst_resolution`] and retain its binding there.
    pub(crate) fn inline_cmd_subst_hook_candidate(
        &self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<InlineCodegenHookId> {
        if self.plain_command_dispatch || !self.invocation_specialisation_proved() {
            return None;
        }
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens.source_binding.is_some()
        {
            return tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )?
            .inline_codegen_hook();
        }
        let arg_refs: Vec<&str> = args.iter().map(|(arg, _)| arg.as_str()).collect();
        let resolved =
            self.registry
                .resolve_call(cmd, &arg_refs, self.invocation_surface_query())?;
        names_builtin(cmd, resolved.spec.name).then_some(resolved.inline_codegen_hook?)
    }

    /// `args` with the subcommand word of `cmd`, an ensemble, spelt as the
    /// subcommand it selects in the emulated release: an ensemble compiles a
    /// unique prefix as the subcommand itself (`TclCompileEnsemble`). `None`
    /// where the word is not literal or selects no one subcommand, which
    /// leaves the command to run. The subcommand resolves under the
    /// invocation's own surface, as the hook did.
    pub(super) fn canonical_subcommand_args(
        &self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<Vec<(String, bool)>> {
        let ((word, braced), rest) = args.split_first()?;
        if tcl_syntax::naming::word_is_dynamic(word, *braced) {
            return None;
        }
        let arg_refs: Vec<&str> = args.iter().map(|(a, _)| a.as_str()).collect();
        let query = self.invocation_surface_query();
        let resolved = self.registry.resolve_call(cmd, &arg_refs, query)?;
        if resolved.spec.subcommands.is_empty() {
            return Some(args.to_vec());
        }
        let sub = resolved
            .sub
            .or_else(|| resolved.spec.resolve_subcommand_for_dialect(word, query))?;
        Some(
            std::iter::once((sub.name.to_owned(), true))
                .chain(rest.iter().cloned())
                .collect(),
        )
    }

    /// Resolve and retain the hook together with the exact entered command
    /// identity selected for this source invocation.
    pub(super) fn inline_cmd_subst_resolution(
        &mut self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<(InlineCodegenHookId, tcl_runtime_api::CommandBindingIdentity)> {
        let arg_refs: Vec<&str> = args.iter().map(|(a, _)| a.as_str()).collect();
        let (hook, site) = self.inline_codegen_resolution(cmd, &arg_refs)?;
        let shape = self.invocation_shape(&arg_refs, |index| args[index].1);
        self.require_site_binding(&site, &shape);
        Some((hook, site.binding))
    }

    /// Resolve and retain one registry-described inline specialisation.
    ///
    /// All inline contexts use this owner: ordinary command substitutions,
    /// catch/try bodies, and structured try emitters. A specialised emitter
    /// therefore cannot consume a command head without also retaining the
    /// source spelling and canonical implementation it assumed.
    pub(crate) fn inline_codegen_hook(
        &mut self,
        cmd: &str,
        args: &[&str],
    ) -> Option<InlineCodegenHookId> {
        let (hook, site) = self.inline_codegen_resolution(cmd, args)?;
        self.require_site_binding(&site, &self.invocation_shape(args, |_| false));
        Some(hook)
    }

    /// Resolve the hook and live command identity assumed by an inline
    /// specialisation. Emitters which retain an entered token use the same
    /// registry decision as the ordinary hook dispatcher.
    pub(super) fn inline_codegen_resolution(
        &self,
        cmd: &str,
        args: &[&str],
    ) -> Option<(InlineCodegenHookId, super::super::SiteBinding)> {
        if self.plain_command_dispatch || !self.invocation_specialisation_proved() {
            return None;
        }
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens.source_binding.is_some()
        {
            let admitted = tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )?;
            return Some((
                admitted.inline_codegen_hook()?,
                super::super::SiteBinding {
                    binding: self
                        .command_binding_identity(cmd, admitted.canonical_registration_name()),
                    claim: None,
                },
            ));
        }
        // Whole-unit mutation discovery is deliberately not an admission
        // decision here: a later proc body may mutate this name only after the
        // current invocation has entered it. Retain the typed binding below;
        // the VM validates it at each actual execution boundary and recompiles
        // stale future invocations through plain dispatch.
        // The registry's own point — see `emitter::bytecoded::try_bytecoded`.
        let resolved = self
            .registry
            .resolve_call(cmd, args, self.invocation_surface_query())?;
        if !names_builtin(cmd, resolved.spec.name) {
            return None;
        }
        let hook = resolved.inline_codegen_hook?;
        if hook == InlineCodegenHookId::InfoCommandsResolve
            && !self
                .invocation_tokens
                .as_deref()
                .and_then(|tokens| tokens.source_binding.as_ref())
                .is_some_and(|binding| {
                    matches!(
                        binding.native_compilation_admission_selection(),
                        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
                            operation: tcl_registry::SemanticOperationId::Intrinsic(
                                tcl_registry::IntrinsicId::InfoCommandsResolve
                            ),
                            ..
                        }
                    )
                })
        {
            return None;
        }
        // As `registry_codegen_hook`: the `alias_of` target's identity, and
        // the claim behind it, where the hook is the target's own; the
        // command's own name otherwise.
        let site = self.stamped_binding(
            cmd,
            &resolved,
            tcl_registry::codegen_stamp::CodegenStamp::InlineCodegen(hook),
        );
        Some((hook, site))
    }

    /// Preserve source-proven local-name semantics before value-position
    /// specialisation. Returns whether it emitted the conservative fallback.
    pub(super) fn emit_dynamic_local_introspection(
        &mut self,
        tokens: Option<&CommandTokens>,
        parts: &mut [(String, bool)],
    ) -> bool {
        let source_local_name =
            nested_local_name_value(tokens, parts, 1, self.escapes, self.word_rules);
        let source_hook = self.inline_cmd_subst_hook_candidate(&parts[0].0, &parts[1..]);
        let source_is_local_introspection = match source_hook {
            Some(InlineCodegenHookId::InfoExists) => parts.len() == 3,
            Some(InlineCodegenHookId::Array) => parts.len() == 3 && parts[1].0 == "exists",
            _ => false,
        };
        if !source_is_local_introspection {
            return false;
        }
        match source_local_name {
            NestedLocalName::Direct(name) => {
                parts[2].0 = name;
                false
            }
            // The specialised emitters place their final name value on the
            // stack verbatim. Without an aligned source word that proves the
            // value is already final, that would suppress a live `$` or
            // command substitution (notably a Return value, whose
            // compatibility path retains no CommandTokens).
            NestedLocalName::Stack if source_hook == Some(InlineCodegenHookId::InfoExists) => false,
            NestedLocalName::Stack | NestedLocalName::Unavailable => {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst(&parts[0].0, &parts[1..]);
                true
            }
        }
    }

    /// Resolve one fully-consuming inline specialisation whose source name
    /// must also remain untouched throughout this compilation unit.
    ///
    /// Unlike entered-token emitters, the list-concat and inline loop-control
    /// paths below have no ordinary invocation left to redispatch. They need
    /// both the whole-unit trust fact and the active registry's typed hook;
    /// the returned binding is the exact dependency their bytecode consumes.
    pub(super) fn trusted_inline_codegen_binding(
        &self,
        command: &str,
        args: &[&str],
        expected: InlineCodegenHookId,
    ) -> Option<tcl_runtime_api::CommandBindingIdentity> {
        if !self.trusts_builtin(command) {
            return None;
        }
        let (hook, site) = self.inline_codegen_resolution(command, args)?;
        // These callers name shipped commands, whose bindings carry no
        // pack claim; one that did would need it recorded, so it declines.
        (hook == expected && site.claim.is_none()).then_some(site.binding)
    }

    /// Try an inline emitter whose registry hook is also valid for an ordinary
    /// command statement.
    ///
    /// Inline hooks normally produce one stack value for a command
    /// substitution. String's `INVOKE_REPLACE` forms have the same argument
    /// substitution and command-entry semantics in statement position, so
    /// reuse their single applicability guard without the nested-substitution
    /// `START_CMD`, then discard the result just as the generic statement
    /// invoke does. Hook selection and the retained command identity still
    /// come from the active registry/profile; this bridge contains no command
    /// identity lookup.
    pub(crate) fn try_inline_statement_codegen(
        &mut self,
        cmd: &str,
        args: &[String],
        tokens: Option<&CommandTokens>,
        used_generic_invoke: &mut bool,
    ) -> bool {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some((hook, site)) = self.inline_codegen_resolution(cmd, &arg_refs) else {
            return false;
        };
        let mut inline_args: Vec<(String, bool)> = args
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                (
                    arg.clone(),
                    self.cmd_arg_braced.get(i).copied().unwrap_or(false),
                )
            })
            .collect();

        let local_name = self.original_hook_argument(1).and_then(|index| {
            source_direct_local_name_value(tokens, index, self.escapes, self.word_rules)
        });

        // These hooks are also valid for a complete command statement. The
        // statement dispatch checks the source-word facts, then reuses the
        // value emitter's local opcode sequence instead of recognising command
        // names itself. The result is discarded just as a normal statement
        // invoke would discard it.
        match hook {
            InlineCodegenHookId::Error if (1..=3).contains(&inline_args.len()) => {
                if !self.emit_inline_error(&inline_args, "") {
                    return false;
                }
            }
            InlineCodegenHookId::NamespaceCurrent => {
                self.emit(Op::CURRENT_NAMESPACE, vec![]);
            }
            InlineCodegenHookId::NamespaceOrigin => {
                if !self.emit_inline_namespace_origin(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::NamespaceCode => {
                if !self.emit_inline_namespace_code(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::Yield if inline_args.len() <= 1 => {
                self.emit_inline_yield(&inline_args);
            }
            InlineCodegenHookId::YieldTo => {
                self.emit_inline_yield_to(
                    inline_args
                        .iter()
                        .map(|(word, braced)| (word.as_str(), *braced, false)),
                );
            }
            InlineCodegenHookId::Lindex => {
                if !self.emit_inline_lindex(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::InfoLevel => {
                if !self.emit_inline_info_level(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::InfoExists => {
                if !self.emit_inline_info_exists(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::InfoCommandsResolve if inline_args.len() == 2 => {
                self.emit_inline_info_commands_resolve(&inline_args);
            }
            InlineCodegenHookId::Array
                if inline_args.len() == 2
                    && inline_args[0].0 == "exists"
                    && local_name.is_some() =>
            {
                inline_args[1].0 = local_name.expect("guard proved a source local name");
                self.emit_inline_array(cmd, &inline_args);
            }
            InlineCodegenHookId::String => {
                if !self.try_inline_statement_string(cmd, &site.binding, &inline_args) {
                    return false;
                }
            }
            InlineCodegenHookId::Regexp if self.try_emit_compiled_regexp(&inline_args) => {}
            hook if self.try_emit_string_subcommand(hook, &inline_args) => {}
            InlineCodegenHookId::Lreplace
                if self.try_emit_compiled_index_command(cmd, &inline_args) => {}
            _ => return false,
        }
        self.emit(Op::POP, vec![]);
        *used_generic_invoke = true;
        let shape = self.invocation_shape(args, |index| {
            self.cmd_arg_braced.get(index).copied().unwrap_or(false)
        });
        self.require_site_binding(&site, &shape);
        true
    }

    fn try_inline_statement_string(
        &mut self,
        cmd: &str,
        binding: &tcl_runtime_api::CommandBindingIdentity,
        args: &[(String, bool)],
    ) -> bool {
        let previous_inline = self.used_inline_cmd_subst;
        let Some(canonical) = self.canonical_subcommand_args(cmd, args) else {
            return false;
        };
        self.try_emit_inline_string_invoke_replace(previous_inline, cmd, binding, &canonical, false)
            || self.try_emit_compiled_index_command(cmd, args)
    }
}
