// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native procedure resources and actual command-binding references.

use super::*;
use obj::{Owned, ProcedureObject};
use tcl_runtime_api::native_procedure_roles::{
    NativeProcedureBinding, NativeProcedureRoleLedger, NativeProcedureRoleOwner,
};

#[derive(Default)]
pub(crate) struct NativeProcedureResources {
    roles: NativeProcedureRoleLedger,
    objects: RefCell<Option<Vec<Owned>>>,
    statics: RefCell<Option<Rc<crate::frame::StaticVariables>>>,
    compiled_names: RefCell<Option<Vec<Option<tcl_core_types::NameBytes>>>>,
}

impl NativeProcedureRoleOwner for ProcDef {
    fn native_procedure_role_ledger(&self) -> &NativeProcedureRoleLedger {
        &self.native_resources.roles
    }
    fn retire_native_procedure_resources(owner: &Rc<Self>) {
        let objects = owner.native_resources.objects.borrow_mut().take();
        let namespace = owner.location.borrow_mut().jim_namespace.take();
        let statics = owner.native_resources.statics.borrow_mut().take();
        let names = owner.native_local_names.borrow_mut().take();
        let compiled_names = owner.native_resources.compiled_names.borrow_mut().take();
        // No owner borrow spans a primary free hook, which can release another
        // declaration reference or enter the interpreter.
        drop(namespace);
        drop(statics);
        drop(names);
        drop(compiled_names);
        drop(objects);
    }
}

/// One actual command binding. Clones keep the binding transport alive without
/// adding native procedure references; queries borrow only the declaration.
#[derive(Clone)]
pub struct NativeProcedureCommand {
    declaration: Rc<ProcDef>,
    binding: NativeProcedureBinding<ProcDef>,
    identity: Rc<()>,
}

/// A genuine method or lambda clientData role on a commandless native Proc.
/// Query clones do not postpone the native owner's retirement. An entered
/// ProcedureMethod keeps its clientData independently until post-call cleanup.
pub(crate) struct NativeCallableProcedure {
    client_data: Rc<NativeCallableClientData>,
    owns_client_data: bool,
}

struct NativeCallableClientData {
    declaration: Rc<ProcDef>,
    reference:
        RefCell<Option<tcl_runtime_api::native_procedure_roles::NativeProcedureReference<ProcDef>>>,
    installed: Cell<bool>,
    entered: Cell<usize>,
}

impl NativeCallableClientData {
    fn release_if_unused(&self) {
        if !self.installed.get() && self.entered.get() == 0 {
            let reference = self.reference.borrow_mut().take();
            drop(reference);
        }
    }
}

impl Clone for NativeCallableProcedure {
    fn clone(&self) -> Self {
        Self {
            client_data: Rc::clone(&self.client_data),
            owns_client_data: false,
        }
    }
}

impl Drop for NativeCallableProcedure {
    fn drop(&mut self) {
        if self.owns_client_data {
            self.client_data.installed.set(false);
            self.client_data.release_if_unused();
        }
    }
}

/// The actual entered ProcedureMethod reference, distinct from Proc execution.
pub(crate) struct NativeCallableMethodExecution {
    client_data: Rc<NativeCallableClientData>,
}

impl Drop for NativeCallableMethodExecution {
    fn drop(&mut self) {
        self.client_data.entered.set(
            self.client_data
                .entered
                .get()
                .checked_sub(1)
                .expect("native ProcedureMethod reference underflow"),
        );
        self.client_data.release_if_unused();
    }
}

impl NativeCallableProcedure {
    pub(crate) fn new(
        params: Vec<Param>,
        body: Owned,
        definition: NativeProcedureDefinition,
    ) -> Self {
        let declaration = create_declaration(params, body, definition);
        Self::from_declaration(declaration)
    }

    fn from_declaration(declaration: Rc<ProcDef>) -> Self {
        let reference = tcl_runtime_api::native_procedure_roles::NativeProcedureReference::acquire(
            &declaration,
        )
        .expect("live commandless native procedure");
        Self {
            client_data: Rc::new(NativeCallableClientData {
                declaration,
                reference: RefCell::new(Some(reference)),
                installed: Cell::new(true),
                entered: Cell::new(0),
            }),
            owns_client_data: true,
        }
    }

    pub(crate) fn declaration(&self) -> Rc<ProcDef> {
        Rc::clone(&self.client_data.declaration)
    }

    pub(crate) fn duplicate_lambda_role(&self) -> Self {
        Self::from_declaration(self.declaration())
    }

    /// Retain the selected ProcedureMethod after compilation, before its core
    /// binds formals and executes. Metadata queries cannot acquire this role
    /// after the real clientData and all entered invocations have retired.
    pub(crate) fn enter_method(
        &self,
    ) -> Result<NativeCallableMethodExecution, tcl_syntax::value::ValueError> {
        if self.client_data.reference.borrow().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native ProcedureMethod clientData lifetime",
            ));
        }
        self.client_data.entered.set(
            self.client_data
                .entered
                .get()
                .checked_add(1)
                .expect("native ProcedureMethod references exhausted"),
        );
        Ok(NativeCallableMethodExecution {
            client_data: Rc::clone(&self.client_data),
        })
    }

    /// TclOO DetailsCloner creates a new Proc with a duplicated, string-only
    /// body, while defaults remain the same original objects.
    pub(crate) fn duplicate_method(&self) -> Result<Self, tcl_syntax::value::ValueError> {
        let original = self.declaration();
        original.check_native_liveness()?;
        let params = original
            .params
            .iter()
            .map(|parameter| {
                Ok(Param {
                    name: parameter.name.clone(),
                    default: parameter
                        .default
                        .as_ref()
                        .map(|value| value.checked_ptr().map(Owned::retain))
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, tcl_syntax::value::ValueError>>()?;
        let source = original.body.checked_ptr()?;
        // The owned Proc body is an issued source String or Bytecode header;
        // compilation retains its original string. No arbitrary host primary
        // can be installed through this clientData owner.
        if !obj::native_string_available(source) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native method source string",
            ));
        }
        let body = Owned::fresh(obj::duplicate(source));
        obj::change_type(body.as_ptr(), core::ptr::null(), 0);
        Ok(Self::new(
            params,
            body,
            NativeProcedureDefinition {
                location: original.location(),
                jim_parameters: None,
                source: original.source.clone(),
                body_line_base: original.body_line_base,
                native: None,
                statics: None,
            },
        ))
    }
}

fn create_declaration(
    params: Vec<Param>,
    body: Owned,
    definition: NativeProcedureDefinition,
) -> Rc<ProcDef> {
    let NativeProcedureDefinition {
        location,
        jim_parameters,
        source,
        body_line_base,
        native,
        statics,
    } = definition;
    let mut objects = Vec::new();
    let mut retain = |original: Owned| {
        let view = ProcedureObject::retain_lifetime(&original);
        objects.push(original);
        view
    };
    let body = retain(body);
    let params = params
        .into_iter()
        .map(|parameter| Param {
            name: parameter.name,
            default: parameter.default.map(&mut retain),
        })
        .collect();
    let jim_parameters = jim_parameters.map(&mut retain);
    Rc::new(ProcDef {
        params,
        body,
        compiler_header: Cell::new(tcl_dialect::NativeProcedureHeaderCompilation::Absent),
        location: RefCell::new(location),
        source,
        body_line_base,
        native,
        jim_parameters,
        native_local_names: RefCell::new(None),
        native_resources: NativeProcedureResources {
            roles: NativeProcedureRoleLedger::default(),
            objects: RefCell::new(Some(objects)),
            statics: RefCell::new(statics),
            compiled_names: RefCell::new(None),
        },
    })
}

impl NativeProcedureCommand {
    pub(crate) fn new(
        params: Vec<Param>,
        body: Owned,
        definition: NativeProcedureDefinition,
    ) -> Self {
        let declaration = create_declaration(params, body, definition);
        let binding =
            NativeProcedureBinding::new(&declaration).expect("new native procedure binding");
        Self {
            declaration,
            binding,
            identity: Rc::new(()),
        }
    }

    /// Current client data of this exact binding, without a new native role.
    #[must_use]
    pub fn declaration(&self) -> Rc<ProcDef> {
        self.binding
            .owner()
            .unwrap_or_else(|| Rc::clone(&self.declaration))
    }
    pub(crate) fn is_same_binding(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
    pub(crate) fn retire(&self) {
        self.binding.retire();
    }
    pub(crate) fn replace_declaration(&self, replacement: &Self) {
        self.binding
            .replace_from(&replacement.binding)
            .expect("new native procedure replacement");
    }

    /// Replace old-C shared client data at a reached compilation entry. The
    /// source object is duplicated, so its Bytecode primary is not transferred;
    /// each default remains the same object held by the new declaration.
    pub(super) fn replace_for_recompilation(
        &self,
    ) -> Result<Rc<ProcDef>, tcl_syntax::value::ValueError> {
        let original = self.declaration();
        original.check_native_liveness()?;
        let params = original
            .params
            .iter()
            .map(|parameter| {
                Ok(Param {
                    name: parameter.name.clone(),
                    default: parameter
                        .default
                        .as_ref()
                        .map(|value| value.checked_ptr().map(Owned::retain))
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, tcl_syntax::value::ValueError>>()?;
        let replacement = Self::new(
            params,
            Owned::fresh(obj::duplicate(original.body.checked_ptr()?)),
            NativeProcedureDefinition {
                location: original.location(),
                jim_parameters: None,
                source: original.source.clone(),
                body_line_base: original.body_line_base,
                native: original.native,
                statics: original.native_statics(),
            },
        );
        replacement
            .declaration()
            .compiler_header
            .set(original.compiler_header.get());
        self.replace_declaration(&replacement);
        Ok(self.declaration())
    }

    /// Create a separate declaration for a copied namespace command. Each body
    /// and default gains exactly the native reference held by that declaration.
    pub(crate) fn duplicate(
        &self,
        namespace: NsId,
        qualified_name: Vec<u8>,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let original = self.declaration();
        if original.native_resources.roles.is_retired() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired native procedure declaration",
            ));
        }
        let params = original
            .params
            .iter()
            .map(|parameter| {
                Ok(Param {
                    name: parameter.name.clone(),
                    default: parameter
                        .default
                        .as_ref()
                        .map(|value| value.checked_ptr().map(Owned::retain))
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, tcl_syntax::value::ValueError>>()?;
        let mut location = original.location();
        location.namespace = namespace;
        location.qualified_name = qualified_name;
        location.jim_namespace = location
            .jim_namespace
            .as_ref()
            .map(|value| Rc::new(Owned::retain(value.as_ptr())));
        let copied = Self::new(
            params,
            Owned::retain(original.body.checked_ptr()?),
            NativeProcedureDefinition {
                location,
                jim_parameters: original
                    .jim_parameters
                    .as_ref()
                    .map(|value| value.checked_ptr().map(Owned::retain))
                    .transpose()?,
                source: original.source.clone(),
                body_line_base: original.body_line_base,
                native: original.native,
                statics: original.native_statics(),
            },
        );
        copied
            .declaration
            .compiler_header
            .set(original.compiler_header.get());
        Ok(copied)
    }
}

pub(crate) struct NativeProcedureDefinition {
    pub(crate) location: ProcLocation,
    pub(crate) jim_parameters: Option<Owned>,
    pub(crate) source: Option<Rc<[u8]>>,
    pub(crate) body_line_base: u32,
    pub(crate) native: Option<NativeProcEntry>,
    pub(crate) statics: Option<Rc<crate::frame::StaticVariables>>,
}

impl ProcDef {
    pub(crate) fn check_native_liveness(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if self.native_resources.roles.is_retired() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired native procedure declaration",
            ));
        }
        Ok(())
    }

    pub(crate) fn native_statics(&self) -> Option<Rc<crate::frame::StaticVariables>> {
        self.native_resources.statics.borrow().clone()
    }
}

impl ProcDef {
    /// Retain only names issued by a successfully admitted procedure LVT.
    pub(crate) fn retain_native_compiled_names(
        &self,
        layout: &tcl_runtime_api::native_compilation::NativeCompiledLocalLayout,
    ) {
        *self.native_resources.compiled_names.borrow_mut() = Some(layout.names.clone());
    }

    pub(super) fn retain_native_compiled_frame_names(
        &self,
        names: Vec<Option<tcl_core_types::NameBytes>>,
    ) {
        *self.native_resources.compiled_names.borrow_mut() = Some(names);
    }

    /// C84's localVarName updater reads the retained declaration record.
    pub(crate) fn native_compiled_name(
        &self,
        index: usize,
    ) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
        self.check_native_liveness()?;
        let names = self.native_resources.compiled_names.borrow();
        let names =
            names
                .as_ref()
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "actual procedure compiled-name declaration",
                ))?;
        Ok(names
            .get(index)
            .and_then(Option::as_ref)
            .map_or_else(Vec::new, |name| name.as_bytes().to_vec()))
    }
}

#[cfg(test)]
mod callable_tests {
    use super::*;

    fn interp(version: &str) -> Interp {
        Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn commandless_roles_share_snapshots_but_duplicate_real_lambda_and_method_owners() {
        for version in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let interp = interp(version);
            let body = Owned::fresh(obj::new_string_bytes(b"set x $a; set x"));
            let default = Owned::fresh(obj::new_string_bytes(b"DEFAULT"));
            let original_body = body.as_ptr();
            let original_default = default.as_ptr();
            let owner = interp.create_native_callable_from_chosen(
                vec![Param {
                    name: b"a".to_vec(),
                    default: Some(default),
                }],
                body,
                GLOBAL,
                None,
                0,
            );
            let declaration = owner.declaration();
            let snapshot = owner.clone();
            assert_eq!(declaration.native_procedure_role_ledger().references(), 1);
            let lambda_copy = owner.duplicate_lambda_role();
            assert_eq!(declaration.native_procedure_role_ledger().references(), 2);
            assert_eq!(unsafe { (*original_body).ref_count }, 1);
            let method_copy = owner.duplicate_method().unwrap();
            let copied = method_copy.declaration();
            assert!(!Rc::ptr_eq(&declaration, &copied));
            assert_ne!(copied.body.checked_ptr().unwrap(), original_body);
            assert_eq!(
                copied.params[0]
                    .default
                    .as_ref()
                    .unwrap()
                    .checked_ptr()
                    .unwrap(),
                original_default
            );
            assert_eq!(unsafe { (*original_default).ref_count }, 2);
            drop(method_copy);
            assert!(copied.check_native_liveness().is_err());
            assert_eq!(unsafe { (*original_default).ref_count }, 1);
            drop(lambda_copy);
            drop(owner);
            assert_eq!(declaration.native_procedure_role_ledger().references(), 0);
            assert!(declaration.body.checked_ptr().is_err());
            assert!(declaration.native_procedure_role_ledger().is_retired());
            assert!(snapshot.enter_method().is_err());
            drop(snapshot);
        }
    }

    #[test]
    fn entered_method_client_data_and_proc_execution_retire_separately_from_queries() {
        use tcl_runtime_api::native_procedure_roles::NativeProcedureReference;
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let interp = interp(version);
            let owner = interp.create_native_callable_from_chosen(
                Vec::new(),
                Owned::fresh(obj::new_string_bytes(b"return OLD")),
                GLOBAL,
                None,
                0,
            );
            let query = owner.clone();
            let declaration = query.declaration();
            // ProcedureMethod's entered reference preserves its one Proc
            // clientData role; the proc core independently acquires execution.
            let method = owner.enter_method().unwrap();
            assert_eq!(declaration.native_procedure_role_ledger().references(), 1);
            let execution = NativeProcedureReference::acquire(&declaration).unwrap();
            drop(owner);
            assert_eq!(declaration.native_procedure_role_ledger().references(), 2);
            assert!(declaration.body.checked_ptr().is_ok());
            drop(execution);
            assert_eq!(declaration.native_procedure_role_ledger().references(), 1);
            drop(method);
            assert!(declaration.body.checked_ptr().is_err());
            assert!(query.enter_method().is_err());
            assert_eq!(declaration.native_procedure_role_ledger().references(), 0);
        }
    }

    #[test]
    fn original_lambda_primary_reuses_its_proc_and_compiled_body_without_a_command_binding() {
        for version in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interp(version);
            let lambda = Owned::fresh(obj::new_string_bytes(b"{a} {set x $a; set x}"));
            let head = Owned::fresh(obj::new_string_bytes(b"apply"));
            let value = Owned::fresh(obj::new_string_bytes(b"VALUE"));
            let argv = [head.as_ptr(), lambda.as_ptr(), value.as_ptr()];
            assert_eq!(interp.dispatch_invoke(&argv), Code::Ok);
            assert_eq!(interp.result_bytes(), b"VALUE");
            let (owner, _) = obj::native_lambda_expression::cached(
                lambda.as_ptr(),
                interp.native_callable_interpreter(),
            )
            .unwrap();
            let declaration = owner.declaration();
            assert_eq!(declaration.native_procedure_role_ledger().references(), 1);
            assert!(super::super::native_body_artifact::cache_snapshot(
                declaration.body.checked_ptr().unwrap()
            )
            .is_some());
            let copy = Owned::fresh(obj::duplicate(lambda.as_ptr()));
            assert_eq!(declaration.native_procedure_role_ledger().references(), 2);
            let copied_argv = [head.as_ptr(), copy.as_ptr(), value.as_ptr()];
            assert_eq!(interp.dispatch_invoke(&copied_argv), Code::Ok);
            let (copied_owner, _) = obj::native_lambda_expression::cached(
                copy.as_ptr(),
                interp.native_callable_interpreter(),
            )
            .unwrap();
            assert!(Rc::ptr_eq(&declaration, &copied_owner.declaration()));
            drop(copied_owner);
            drop(copy);
            drop(owner);
            drop(lambda);
            assert!(declaration.check_native_liveness().is_err());
        }
    }
}
