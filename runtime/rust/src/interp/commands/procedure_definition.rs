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

//! Defining a procedure: original parameter and body storage, the chosen slot and its publication.

use super::super::native_procedure_resources::NativeProcedureCommand;
use super::super::{
    native_i64_body, native_operation_currency, native_procedure_resources, new_string,
    ImportQueryPurpose, Interp, NativeProcEntry, OriginalCommandPublication,
    OriginalProcedureObjects, OriginalProcedurePublication, Param, ProcDef, ProcLocation,
};
use super::Command;
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;

impl Interp {
    /// Define a user proc (`proc name params body`). The proc's defining
    /// namespace (where its body runs, and where it is bound) is the namespace
    /// `name` lands in — **relative to the current namespace** (so `proc next`
    /// inside `namespace eval counter` binds `::counter::next`, not a global).
    #[cfg(test)]
    pub(crate) fn define_proc(&mut self, name: &[u8], params: Vec<Param>, body_obj: *mut TclObj) {
        self.define_proc_native(name, params, body_obj, None);
    }

    /// Test constructor forwarding an explicit compiled entry to the
    /// original procedure storage owner.
    #[cfg(test)]
    pub(crate) fn define_proc_native(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
    ) {
        self.define_proc_storage(name, params, body_obj, native, None);
    }

    #[cfg(test)]
    pub(crate) fn define_proc_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) {
        self.define_proc_original_storage(name, params, None, body_obj, native, statics);
    }

    pub(crate) fn define_proc_original_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) {
        let _ = self.install_proc_original_storage(
            name,
            params,
            parameters_obj,
            body_obj,
            native,
            statics,
        );
    }

    /// Install original procedure storage and return its actual command generation.
    pub(crate) fn install_proc_original_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<u64> {
        self.install_proc_original_storage_publication(
            name,
            params,
            parameters_obj,
            body_obj,
            native,
            statics,
        )
        .map(|publication| publication.generation)
    }

    pub(in super::super) fn install_proc_original_storage_publication(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<OriginalProcedurePublication> {
        self.install_proc_original_storage_publication_using(
            name,
            params,
            OriginalProcedureObjects {
                parameters_obj,
                body_obj,
            },
            native,
            statics,
            None,
        )
    }

    pub(in super::super) fn install_proc_original_storage_publication_using(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        OriginalProcedureObjects {
            parameters_obj,
            body_obj,
        }: OriginalProcedureObjects,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
        import_source: Option<native_procedure_resources::OriginalJimNamespaceHelperSource>,
    ) -> Option<OriginalProcedurePublication> {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return None;
            }
        };
        let reached = self.install_proc_original_storage_entered(
            name,
            params,
            OriginalProcedureObjects {
                parameters_obj,
                body_obj,
            },
            native,
            statics,
            import_source,
        );
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first);
            return None;
        }
        let publication = reached?;
        self.check_original_procedure_publication(
            publication.namespace,
            &publication.tail,
            publication.generation,
            &publication.binding,
        )
        .ok()?;
        Some(publication)
    }

    fn install_proc_original_storage_entered(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        OriginalProcedureObjects {
            parameters_obj,
            body_obj,
        }: OriginalProcedureObjects,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
        import_source: Option<native_procedure_resources::OriginalJimNamespaceHelperSource>,
    ) -> Option<OriginalProcedurePublication> {
        if self.host_refusal_pending() {
            return None;
        }
        let chosen = match self.choose_original_procedure_body(body_obj) {
            Ok(chosen) => chosen,
            Err(error) => {
                self.report_cmd_error(error.into());
                return None;
            }
        };
        self.install_proc_chosen_storage_publication(
            name,
            params,
            parameters_obj,
            (body_obj, chosen, import_source),
            native,
            statics,
        )
    }

    /// Bind the chosen original procedure and return its installed generation.
    pub(crate) fn install_proc_chosen_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (*mut TclObj, obj::Owned),
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<u64> {
        self.install_proc_chosen_storage_publication(
            name,
            params,
            parameters_obj,
            (body.0, body.1, None),
            native,
            statics,
        )
        .map(|publication| publication.generation)
    }

    fn install_proc_chosen_storage_publication(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (
            *mut TclObj,
            obj::Owned,
            Option<native_procedure_resources::OriginalJimNamespaceHelperSource>,
        ),
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<OriginalProcedurePublication> {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return None;
            }
        };
        let reached = self.install_proc_chosen_storage_entered(
            name,
            params,
            parameters_obj,
            body,
            native,
            statics,
        );
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first);
            return None;
        }
        let publication = reached?;
        self.check_original_procedure_publication(
            publication.namespace,
            &publication.tail,
            publication.generation,
            &publication.binding,
        )
        .ok()?;
        Some(publication)
    }

    fn install_proc_chosen_storage_entered(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (
            *mut TclObj,
            obj::Owned,
            Option<native_procedure_resources::OriginalJimNamespaceHelperSource>,
        ),
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<OriginalProcedurePublication> {
        let original_key = match self.generated_jim_command_key(
            self.current_ns.get(),
            name,
            tcl_syntax::naming::NativeNamePurpose::CommandPublication,
        ) {
            Ok(key) => key,
            Err(error) => {
                self.report_cmd_error(error.into());
                return None;
            }
        };
        self.install_proc_chosen_storage_with_original_key_publication(
            (name, original_key),
            params,
            parameters_obj,
            body,
            native,
            statics,
        )
    }

    /// The original proc command supplies the already-qualified actual key.
    /// This does not reconstruct a header from the publication report.
    pub(crate) fn install_proc_chosen_storage_with_original_key(
        &mut self,
        name: (&[u8], Option<Rc<obj::Owned>>),
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (*mut TclObj, obj::Owned),
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<u64> {
        self.install_proc_chosen_storage_with_original_key_publication(
            name,
            params,
            parameters_obj,
            (body.0, body.1, None),
            native,
            statics,
        )
        .map(|publication| publication.generation)
    }

    pub(crate) fn install_proc_chosen_storage_with_original_key_candidate(
        &mut self,
        name: (&[u8], Option<Rc<obj::Owned>>),
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (*mut TclObj, obj::Owned),
        candidate: native_i64_body::NativeProcedureBodyCandidate,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<u64> {
        let Some(plan) = candidate.i64_add else {
            return self.install_proc_chosen_storage_with_original_key(
                name,
                params,
                parameters_obj,
                body,
                candidate.native,
                statics,
            );
        };
        if plan.parameters().is_empty() || plan.body().is_empty() {
            return self.install_proc_chosen_storage_with_original_key(
                name,
                params,
                parameters_obj,
                body,
                candidate.native,
                statics,
            );
        }
        let publication = self.install_proc_chosen_storage_with_original_key_publication(
            name,
            params,
            parameters_obj,
            (body.0, body.1, None),
            candidate.native,
            statics,
        )?;
        let declaration = publication.binding.published_declaration()?;
        declaration.future_i64_add.set(plan).ok()?;
        native_i64_body::observe("publication-attached", None);
        Some(publication.generation)
    }

    fn install_proc_chosen_storage_with_original_key_publication(
        &mut self,
        name: (&[u8], Option<Rc<obj::Owned>>),
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (
            *mut TclObj,
            obj::Owned,
            Option<native_procedure_resources::OriginalJimNamespaceHelperSource>,
        ),
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<OriginalProcedurePublication> {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return None;
            }
        };
        let registration_name = name.1.as_ref().map(Rc::clone);
        let reached = self.install_proc_chosen_storage_with_original_key_entered(
            name,
            params,
            parameters_obj,
            body,
            native,
            statics,
        );
        drop(registration_name);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first);
            return None;
        }
        let publication = reached?;
        self.check_original_procedure_publication(
            publication.namespace,
            &publication.tail,
            publication.generation,
            &publication.binding,
        )
        .ok()?;
        Some(publication)
    }

    fn install_proc_chosen_storage_with_original_key_entered(
        &mut self,
        name: (&[u8], Option<Rc<obj::Owned>>),
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (
            *mut TclObj,
            obj::Owned,
            Option<native_procedure_resources::OriginalJimNamespaceHelperSource>,
        ),
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<OriginalProcedurePublication> {
        let (name, original_key) = name;
        // JimQualifyName's temporary spans CreateCommand and the procedure's
        // namespace/header handoff, independently of the table's keyDup role.
        if self.host_refusal_pending() {
            return None;
        }
        // A Rust-authored formal list has no original object. C's compile hook
        // depends only on whether the list is exactly `args`, so spell that.
        let args_only = matches!(
            params.as_slice(),
            [Param { name, default: None }] if name.as_slice() == b"args"
        );
        let authored_formals: Option<&[u8]> =
            parameters_obj
                .is_none()
                .then_some(if args_only { b"args" } else { b"" });
        let (original_body, body, import_source) = body;
        if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            if let Err(error) = self.native_jim_object_context() {
                self.report_cmd_error(error.into());
                return None;
            }
        }
        let publication = self
            .namespaces_mut()
            .procedure_publication_at(self.current_ns.get(), name);
        let (binding_ns, tail) = match publication {
            Some(slot) => slot,
            None => {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "procedure publication namespace",
                    ),
                );
                return None;
            }
        };
        let ns = if self
            .name_policy_protocol()
            .is_some_and(|protocol| protocol.recipe().is_jim084())
        {
            let context = self.namespaces_mut().jim_procedure_context(&tail);
            match context {
                Some(context) => context,
                None => {
                    self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("Jim procedure namespace object"));
                    return None;
                }
            }
        } else {
            binding_ns
        };
        let fqn = self.namespaces().command_fqn_at(binding_ns, &tail);
        // The proc's body frame reports `type source` with file-absolute lines
        // when its body argument is a located literal (TIP 280 LABC) — the body
        // word, not the `proc` command, carries the location, so a `proc` whose
        // body opens on a later line than the command is still file-accurate, and
        // a *dynamic* body (`proc p {} $bodyVar`, or a body from a dynamically
        // built list) has no location and stays body-relative (`type proc`),
        // matching C's literal line table rather than a whole-file "am I
        // sourcing" flag.
        let (source, body_line_base) = match self.arg_loc(original_body) {
            Some((file @ Some(_), line)) => (file, line.saturating_sub(1)),
            _ => (None, 0),
        };
        let jim = self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        let jim_namespace = if jim {
            self.namespaces_mut().take_jim_namespace_owner(ns)
        } else {
            None
        };
        let command = NativeProcedureCommand::new(
            params,
            body,
            native_procedure_resources::NativeProcedureDefinition {
                location: ProcLocation {
                    namespace: ns,
                    qualified_name: fqn.clone(),
                    jim_namespace,
                },
                jim_parameters: if jim {
                    parameters_obj.map(obj::Owned::retain)
                } else {
                    None
                },
                source,
                body_line_base,
                native,
                statics,
            },
        );
        let def = command.declaration();
        if let Some(import_source) = import_source {
            if let Err(error) = def.retain_original_jim_namespace_helper_source(import_source) {
                self.report_cmd_error(error.into());
                return None;
            }
        }

        let original_binding = command.observe_original_binding();
        let report = self.namespaces().jim_command_table_key_at(
            self.current_ns.get(),
            name,
            tcl_syntax::naming::NativeNamePurpose::CommandPublication,
        );
        let OriginalCommandPublication::Published(generation) = self
            .bind_command_replacement_with_original_jim_key(
                binding_ns,
                &tail,
                Command::Proc(command),
                report,
                original_key,
            )
        else {
            return None;
        };
        if let Err(first) = self.check_entered_native_operation() {
            self.refuse_native_execution(first);
            return None;
        }
        let current = |interpreter: &mut Self| {
            interpreter.check_original_procedure_publication(
                binding_ns,
                &tail,
                generation,
                &original_binding,
            )
        };
        if current(self).is_err() {
            return None;
        }
        let authored_formals =
            authored_formals.map(|formals| obj::Owned::fresh(new_string(formals)));
        let header = match self.original_procedure_compiler_header(
            parameters_obj.or(authored_formals.as_ref().map(obj::Owned::as_ptr)),
            original_body,
            &current,
        ) {
            Ok(header) => header,
            Err(error) => {
                self.report_cmd_error(error);
                return None;
            }
        };
        if current(self).is_err() {
            return None;
        }
        def.compiler_header.set(header);
        {
            let recipe = match header {
                tcl_dialect::NativeProcedureHeaderCompilation::Absent => {
                    crate::namespace::NativeCompilerRecipe::Absent
                }
                tcl_dialect::NativeProcedureHeaderCompilation::NoOp => {
                    crate::namespace::NativeCompilerRecipe::ProcedureNoOp
                }
                tcl_dialect::NativeProcedureHeaderCompilation::Unknown => {
                    crate::namespace::NativeCompilerRecipe::Unknown
                }
            };
            self.namespaces_mut()
                .set_native_compiler_recipe(generation, recipe);
        }
        Some(OriginalProcedurePublication {
            namespace: binding_ns,
            tail,
            generation,
            binding: original_binding,
        })
    }

    /// The proc definition bound to `name` (for `info body`/`args`/`default`).
    pub(crate) fn proc_def(&self, name: &[u8]) -> Option<Rc<ProcDef>> {
        let target = self.resolve_import_query(
            self.import_query_binding(name)?,
            ImportQueryPurpose::Introspection,
        )?;
        match target.command {
            Command::Proc(def) => Some(def.declaration()),
            _ => None,
        }
    }
}
