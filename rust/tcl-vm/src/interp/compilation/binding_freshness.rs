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

//! Whether compiled code still holds: command bindings against the live table, inlined facts, identity manifests and spec-pack claims.

use super::super::commands::CommandSidecarKey;
use super::super::{NamespacePath, Vm};
use crate::command::Command;
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{ArtefactIdentityManifest, ROOT_NS};

impl Vm {
    /// Whether every executable namespace recorded by a bare function agrees
    /// with the namespace in which an embedder wants to admit it.
    ///
    /// [`FunctionAsm`] has no top-level namespace field or source from which
    /// the VM could rebuild it.  Its binding requirements and replayable source
    /// boundaries are therefore the only safe provenance available at this
    /// low-level entry point. Mixed-namespace inlined assembly is deliberately
    /// rejected here; callers retaining the surrounding [`ModuleAsm`] or a
    /// [`crate::embed::FunctionHandle`] have the typed provenance needed to run
    /// it without this conservative restriction.
    pub(crate) fn function_resolution_namespace_matches(
        asm: &FunctionAsm,
        expected: &NamespacePath,
    ) -> bool {
        asm.command_bindings.iter().all(|binding| {
            binding.namespace_context.as_ref().map_or_else(
                || Self::legacy_source_namespace(&binding.resolution_namespace) == *expected,
                |context| context.path() == expected,
            )
        }) && asm.instructions.iter().all(|instruction| {
            let boundary = instruction.source_command_boundary.is_start()
                || instruction.source_command_boundary.is_inline_replay();
            !boundary || instruction.source_command_namespace == *expected
        })
    }

    /// Whether `name`, resolved from its compiler-recorded source namespace,
    /// still reaches the registry implementation named by `identity`.
    /// Same-interpreter aliases are followed only when they prepend no
    /// arguments: that is the binding shape the compiler may treat as the
    /// target command itself.
    pub(crate) fn command_binding_matches(
        &self,
        binding: &tcl_runtime_api::CommandBindingIdentity,
    ) -> bool {
        let Some(mut key) = self.resolve_compiled_binding_name(
            &binding.resolution_namespace,
            binding.namespace_context.as_ref(),
            &binding.name,
        ) else {
            return false;
        };
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert(key.clone()) {
                return false;
            }
            // C Tcl refuses command-specific compilation when the exact token
            // has an execution trace. Apply that rule at every registry/alias
            // hop so all compiled consumers share one specialisation gate.
            if self.command_has_execution_trace(&CommandSidecarKey::visible(key.clone())) {
                return false;
            }
            match self.commands.get(&key) {
                Some(Command::Builtin(_) | Command::Ensemble(_)) => {
                    return !self.is_host_builtin(&key)
                        && self
                            .builtin_identity_for_key(&key)
                            .is_some_and(|found| found == binding.identity);
                }
                Some(Command::Object(_)) => {
                    return self
                        .registry_object_roots
                        .get(&key)
                        .is_some_and(|found| found == &binding.identity);
                }
                Some(Command::Alias { words, .. }) if words.len() == 1 => {
                    let Some(target) = words[0].resident_string_bytes() else {
                        return false;
                    };
                    if words[0].native_object_reference_count() == 0 {
                        return false;
                    }
                    let Some(next) = self.resolve_command_from_token_bytes(ROOT_NS, &target, false)
                    else {
                        return false;
                    };
                    key = next;
                }
                _ => return false,
            }
        }
    }

    pub(crate) fn function_command_bindings_match(&self, asm: &FunctionAsm) -> bool {
        self.function_command_bindings_match_at(asm, true, None)
    }

    pub(crate) fn function_command_bindings_match_with_manifest(
        &self,
        asm: &FunctionAsm,
        manifest: Option<&ArtefactIdentityManifest>,
    ) -> bool {
        self.function_command_bindings_match_at(asm, true, manifest)
    }

    /// Active native chunks retain their admitted chunk-entry operations.
    /// Document-procedure inlining dependencies, retained artifact identity and
    /// pack claims remain live at every command boundary.
    pub(crate) fn function_live_command_bindings_match(
        &self,
        asm: &FunctionAsm,
        selected: &[&tcl_runtime_api::CommandBindingIdentity],
        manifest: Option<&ArtefactIdentityManifest>,
    ) -> bool {
        self.compiled_local_layout_matches(asm)
            && asm
                .command_bindings
                .iter()
                .filter(|binding| {
                    binding.guard == tcl_runtime_api::CommandBindingGuard::BeforeArguments
                })
                .all(|binding| selected.contains(&binding) || self.command_binding_matches(binding))
            && asm
                .procedure_bindings
                .iter()
                .all(|binding| self.procedure_binding_matches(binding))
            && self.function_identity_claims_match(asm, manifest)
    }

    fn function_command_bindings_match_at(
        &self,
        asm: &FunctionAsm,
        admission: bool,
        manifest: Option<&ArtefactIdentityManifest>,
    ) -> bool {
        if !self.compiled_local_layout_matches(asm) {
            return false;
        }
        if admission
            && (!self.native_math_table_prerequisite_matches(asm)
                || !self.native_compiler_prerequisites_match(asm))
        {
            return false;
        }
        self.function_bindings_hold(asm, admission)
            && self.function_identity_claims_match(asm, manifest)
    }

    fn function_bindings_hold(&self, asm: &FunctionAsm, admission: bool) -> bool {
        asm.command_bindings
            .iter()
            .filter(|binding| {
                admission || binding.guard == tcl_runtime_api::CommandBindingGuard::BeforeArguments
            })
            .all(|binding| self.command_binding_matches(binding))
            && asm
                .procedure_bindings
                .iter()
                .all(|binding| self.procedure_binding_matches(binding))
    }

    /// [`Self::function_command_bindings_match_with_manifest`] for a retained
    /// body. Release builds trust bindings that held under an unchanged binding
    /// stamp; debug builds re-prove them and check the memo agreed.
    pub(super) fn retained_function_bindings_match(
        &self,
        asm: &Rc<FunctionAsm>,
        manifest: Option<&ArtefactIdentityManifest>,
    ) -> bool {
        if !self.compiled_local_layout_matches(asm)
            || !self.native_math_table_prerequisite_matches(asm)
            || !self.native_compiler_prerequisites_match(asm)
            || !self.function_identity_claims_match(asm, manifest)
        {
            return false;
        }
        let stamp = self.binding_stamp();
        let held = self
            .compilation_epochs
            .held_bindings
            .borrow()
            .iter()
            .any(|(seen, at)| {
                *at == stamp
                    && seen.strong_count() != 0
                    && std::ptr::eq(seen.as_ptr(), Rc::as_ptr(asm))
            });
        if held && !crate::VERIFY_CURRENCY {
            return true;
        }
        let holds = self.function_bindings_hold(asm, true);
        debug_assert!(!held || holds, "binding stamp outlived a binding change");
        if holds && !held {
            let mut memo = self.compilation_epochs.held_bindings.borrow_mut();
            memo.retain(|(seen, at)| seen.strong_count() != 0 && *at == stamp);
            if memo.len() >= 32 {
                memo.remove(0);
            }
            memo.push((Rc::downgrade(asm), stamp));
        }
        holds
    }

    /// What a release that keeps the commands running bytecode compiled
    /// still relies on after a command-table change: the procedures inlined
    /// into `asm`, its compiled-local layout, and its artifact identity and
    /// pack claims.
    pub(crate) fn function_inlined_facts_hold(
        &self,
        asm: &FunctionAsm,
        manifest: Option<&ArtefactIdentityManifest>,
    ) -> bool {
        self.compiled_local_layout_matches(asm)
            && asm
                .procedure_bindings
                .iter()
                .all(|binding| self.procedure_binding_matches(binding))
            && self.function_identity_claims_match(asm, manifest)
    }

    /// Retained artifact identity and pack claims at admission or live revalidation.
    /// This does not supply compiler, command, procedure or activation authority.
    pub(crate) fn function_identity_claims_match(
        &self,
        asm: &FunctionAsm,
        manifest: Option<&ArtefactIdentityManifest>,
    ) -> bool {
        self.site_claims_hold(asm) && self.manifest_admits(asm, manifest)
    }

    /// The manifest check, per rung: the fields in which the module's
    /// manifest disagrees with this VM's identity refuse the rungs that rest
    /// on them, and `asm` is admitted unless it has a site at one of those. A
    /// function with only generic-dispatch sites is admitted under a changed
    /// pack set; one with a pack-fact site is not. Assembly with no manifest is
    /// admitted by its bindings and claims alone.
    pub(in super::super) fn manifest_admits(
        &self,
        asm: &FunctionAsm,
        manifest: Option<&ArtefactIdentityManifest>,
    ) -> bool {
        manifest.is_none_or(|manifest| {
            !manifest
                .refused_rungs(match self.selected_source_identity() {
                    Ok(identity) => identity,
                    Err(_) => return false,
                })
                .intersects(asm.rungs())
        })
    }

    /// The claims check: every spec-pack claim `asm`'s sites make stamps
    /// facts this VM holds — the same pack, content hash, vocabulary
    /// version, overlay generation and evaluator revision — and says what its
    /// kind of claim must. A unit that claims nothing, as every unit compiled
    /// without a pack does, holds trivially.
    fn site_claims_hold(&self, asm: &FunctionAsm) -> bool {
        asm.site_claims.iter().all(|claim| {
            self.pack_facts.contains(claim.facts()) && Self::claim_is_coherent(claim, asm)
        })
    }

    /// What a claim states beyond the pack facts it stamps. A reference-body
    /// claim must name a procedure binding the function carries — the binding is
    /// what holds the live command to the body, so a claim of a body nothing
    /// checks is the artefact contradicting itself — and its backing must be a
    /// Tcl body. An exact match of a procedure's text is a true statement about
    /// the procedure and says nothing about whether the command *is* that
    /// procedure: a command the host registered natively, or one nothing
    /// executes, is not made one by a procedure of the same text.
    fn claim_is_coherent(claim: &tcl_runtime_api::SiteClaim, asm: &FunctionAsm) -> bool {
        match claim {
            tcl_runtime_api::SiteClaim::ReferenceBody {
                procedure, backing, ..
            } => {
                *backing == tcl_runtime_api::BackingKind::TclBody
                    && asm.procedure_bindings.contains(procedure)
            }
            tcl_runtime_api::SiteClaim::PackFacts(_)
            | tcl_runtime_api::SiteClaim::BuiltinAlias { .. } => true,
        }
    }

    /// Run under the facts of the spec-pack set code is compiled against —
    /// one [`tcl_runtime_api::PackFactStamp`] per pack file the set installs
    /// (`tcl_spectcl::PackSet::fact_stamps`). A compiled unit is admitted
    /// only when every site claim it carries stamps one of them; anything
    /// else is plain dispatch when the unit has source and a compile service
    /// is installed, and an admission error otherwise. Replacing the facts
    /// advances the compilation-deopt epoch, so a unit admitted under the
    /// old facts is checked again at its next entry.
    pub fn set_pack_facts(&mut self, stamps: Vec<tcl_runtime_api::PackFactStamp>) {
        if self.pack_facts != stamps {
            self.advance_native_operation_context();
        }
        let state = &mut *self.state;
        state.pack_facts = stamps;
        state.pin.restate(&state.pack_facts);
        if let Some(core) = &mut state.native_core_pin {
            core.restate(&state.pack_facts);
        }
        self.bump_trace_deopt_epoch();
    }

    /// Whether a compiler-inlined user-procedure body still belongs to the
    /// exact source binding and live command token whose definition the
    /// compiler copied.
    ///
    /// Resolving the source invocation again is essential: a newly-created
    /// command in its namespace may shadow the previously-selected global or
    /// namespace-path target without changing that old target's definition.
    /// The compiler also supplies the selected target as an already-constructed
    /// rooted key, so remove its root marker exactly once instead of resolving
    /// it as a written name. The procedure's creation name rejects an unrelated
    /// same-source proc renamed onto the binding; matching source permits a
    /// behaviour-equivalent same-source redefinition. An execution trace always
    /// declines the optimisation because inlining erased the traced call
    /// boundary.
    fn procedure_binding_matches(
        &self,
        binding: &tcl_runtime_api::ProcedureBindingIdentity,
    ) -> bool {
        let Some(target_key) = tcl_syntax::naming::unroot_rooted_key(&binding.name) else {
            return false;
        };
        let Some(resolved_key) = self.resolve_compiled_binding_name(
            &binding.resolution_namespace,
            binding.namespace_context.as_ref(),
            &binding.invocation_name,
        ) else {
            return false;
        };
        if resolved_key != target_key {
            return false;
        }
        if self.command_has_execution_trace(&CommandSidecarKey::visible(resolved_key.clone())) {
            return false;
        }
        let Some(Command::Proc(proc)) =
            self.command_at_resolved_key(&binding.invocation_name, &resolved_key)
        else {
            return false;
        };
        if proc.actual_name() != resolved_key || proc.body_src.to_str().as_ref() != binding.body {
            return false;
        }
        let Ok((expected, expected_has_args)) =
            crate::command::parse_params_in(&binding.parameters, proc.parameter_grammar)
        else {
            return false;
        };
        expected_has_args == proc.has_args
            && expected.len() == proc.params.len()
            && expected.iter().zip(&proc.params).all(|(want, got)| {
                want.name == got.name
                    && want.default.as_ref().map(Value::to_str)
                        == got.default.as_ref().map(Value::to_str)
            })
    }
}
