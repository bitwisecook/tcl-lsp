// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Jim Source owns an original filename object and its signed native line.
//! The interpreter context retains its actual empty and null-script objects.
//! Weak object-context associations grant no filename or object ownership.

use crate::obj::{self, Owned, TclObj, TclObjType};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

pub(crate) struct NativeJimObjectContext {
    numeric_host: RefCell<Option<Rc<dyn tcl_platform::Host>>>,
    live: std::cell::Cell<bool>,
    pool: Rc<obj::JimPool>,
    objects: [RefCell<Option<Owned>>; 10],
}

impl NativeJimObjectContext {
    pub(crate) fn new(
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Rc<Self>, tcl_syntax::value::ValueError> {
        if dialect.native_string_protocol() != Some(NativeStringProtocol::Jim084) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original source context",
            ));
        }
        let pool = Rc::new(obj::JimPool::new(true));
        let allocation = obj::JimAllocationScope::enter(Some(Rc::clone(&pool)), Rc::new(|| true));
        let empty = Owned::fresh(obj::new_string_bytes(b""));
        let context = Rc::new(Self {
            numeric_host: RefCell::new(None),
            live: std::cell::Cell::new(true),
            pool,
            objects: [
                Some(empty.clone()),
                Some(Owned::fresh(obj::new_wide_int_obj(1))),
                Some(Owned::fresh(obj::new_wide_int_obj(0))),
                None,
                None,
                Some(empty),
                Some(Owned::fresh(obj::new_string_bytes(b"unknown"))),
                Some(Owned::fresh(obj::new_string_bytes(b"jim::defer"))),
                Some(Owned::fresh(obj::new_string_bytes(b""))),
                Some(Owned::fresh(obj::new_string_bytes(b""))),
            ]
            .map(RefCell::new),
        });
        for object in &context.objects {
            let object = object.borrow();
            let Some(value) = object.as_ref() else {
                continue;
            };
            bind_context(value.as_ptr(), &context)?;
        }
        drop(allocation);
        Ok(context)
    }

    pub(crate) fn allocation_pool(&self) -> Rc<obj::JimPool> {
        Rc::clone(&self.pool)
    }

    pub(crate) fn select_numeric_host(&self, host: Rc<dyn tcl_platform::Host>) {
        *self.numeric_host.borrow_mut() = Some(host);
    }

    /// Actual retained host target fields, independent of source grammar.
    pub(crate) fn scalar_getter_target(
        &self,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterTarget, tcl_syntax::value::ValueError>
    {
        use tcl_syntax::value::ValueError;
        if !self.is_live() {
            return Err(ValueError::CommandProtocolUnavailable(
                "retired Jim interpreter",
            ));
        }
        let host = self
            .numeric_host
            .borrow()
            .clone()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let environment = host
            .numeric_environment()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        tcl_cmd_core::native_numeric::scalar_getter_target(environment)
    }

    pub(crate) fn fresh_numeric_conversion(
        &self,
        protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
        kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
        original: &[u8],
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterConversion, ValueError> {
        if !self.is_live() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired Jim interpreter",
            ));
        }
        let host = self
            .numeric_host
            .borrow()
            .clone()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let environment = host
            .numeric_environment()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        tcl_cmd_core::native_numeric::fresh_jim_conversion(protocol, kind, original, environment)
    }

    /// Forward this context's actual retained Host through the caller's
    /// original header/currency checker and single ABI observation cell.
    /// Pure target/fresh-conversion entries retain their independent route.
    pub(crate) fn with_numeric_environment_checked<T>(
        &self,
        check: &dyn Fn() -> Result<(), ValueError>,
        abi: &std::cell::Cell<Option<tcl_platform::NativeCIntegerAbi>>,
        operation: impl FnOnce(&dyn tcl_platform::NumericEnvironment) -> Result<T, ValueError>,
    ) -> Result<T, ValueError> {
        check()?;
        let host = self
            .numeric_host
            .borrow()
            .clone()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        check()?;
        let environment = host.numeric_environment();
        check()?;
        let environment = environment.ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let stage = || check().map_err(|_| tcl_platform::NumericEnvironmentUnavailable::Target);
        let checked = tcl_platform::CheckedNumericEnvironment::new(environment, &stage, abi);
        let result = operation(&checked);
        check()?;
        result
    }

    pub(crate) fn is_live(&self) -> bool {
        self.live.get()
    }
    /// Borrow the interpreter's original unknown header without another native reference.
    pub(crate) fn unknown_object(&self) -> Result<std::cell::Ref<'_, Owned>, ValueError> {
        let unavailable =
            || ValueError::CommandProtocolUnavailable("original Jim unknown object role");
        if !self.is_live() {
            return Err(unavailable());
        }
        let original = std::cell::Ref::filter_map(
            self.objects
                [tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Unknown as usize]
                .borrow(),
            Option::as_ref,
        )
        .map_err(|_| unavailable())?;
        obj::check_native_liveness(original.as_ptr())?;
        Ok(original)
    }
    pub(crate) fn defer_object(&self) -> std::cell::Ref<'_, Owned> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Defer)
    }
    fn object(
        &self,
        role: tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole,
    ) -> std::cell::Ref<'_, Owned> {
        std::cell::Ref::map(self.objects[role as usize].borrow(), |object| {
            object.as_ref().expect("live Jim interpreter object role")
        })
    }
    pub(crate) fn empty_object(&self) -> std::cell::Ref<'_, Owned> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Empty)
    }
    pub(crate) fn null_script_object(&self) -> std::cell::Ref<'_, Owned> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::NullScript)
    }
    /// Borrow the actual filename role for observation without a native retain.
    pub(crate) fn current_filename_object_ref(&self) -> std::cell::Ref<'_, Owned> {
        self.object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::CurrentFilename)
    }
    pub(crate) fn current_filename_object(&self) -> Owned {
        self.current_filename_object_ref().clone()
    }
    pub(crate) fn release_object(
        &self,
        role: tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole,
    ) {
        let retired = self.objects[role as usize].borrow_mut().take();
        drop(retired);
    }
    pub(crate) fn retire(&self) {
        self.live.set(false);
        self.pool.retire();
        let host = self.numeric_host.borrow_mut().take();
        drop(host);
    }
    pub(crate) fn replace_current_filename(
        self: &Rc<Self>,
        value: *mut TclObj,
    ) -> Result<(), ValueError> {
        bind_context(value, self)?;
        let retired = self.objects
            [tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::CurrentFilename as usize]
            .borrow_mut()
            .replace(Owned::retain(value));
        drop(retired);
        Ok(())
    }
}

impl Drop for NativeJimObjectContext {
    fn drop(&mut self) {
        // Interpreter teardown already consumes these roles when installed.
        // Standalone selected contexts own the same ordered references.
        for object in &mut self.objects {
            drop(object.get_mut().take());
        }
    }
}

#[derive(Clone)]
pub(crate) struct NativeJimSourceInfo {
    pub(crate) filename: Owned,
    pub(crate) line: i32,
}

struct NativeJimSource {
    info: NativeJimSourceInfo,
    context: Weak<NativeJimObjectContext>,
    origin: Option<Rc<NativeJimSourceOrigin>>,
}

/// An actual source constructor birth. Copies retain source locations without
/// inheriting the original execution tree.
pub(crate) struct NativeJimSourceOrigin {
    root: Rc<()>,
    original: *mut TclObj,
    parent: Option<Rc<Self>>,
    kind: Option<tcl_syntax::jim_script_objects::JimScriptObjectKind>,
}

impl NativeJimSourceOrigin {
    pub(crate) fn same_root(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.root, &other.root)
    }

    pub(crate) fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    pub(crate) fn kind(&self) -> Option<tcl_syntax::jim_script_objects::JimScriptObjectKind> {
        self.kind
    }

    pub(crate) fn belongs_to(&self, original: *mut TclObj) -> bool {
        self.original == original
    }

    pub(crate) fn is_child_of(&self, parent: &Rc<Self>) -> bool {
        self.parent
            .as_ref()
            .is_some_and(|actual| Rc::ptr_eq(actual, parent))
    }
}

/// Capture a real Source birth before its selected declaration is published.
pub(crate) fn retain_original_source_origin(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<Rc<NativeJimSourceOrigin>, ValueError> {
    let source = original_source_backing(value, context)?.ok_or(
        ValueError::CommandProtocolUnavailable("original Source constructor birth"),
    )?;
    // SAFETY: the exact live Source primary owns this backing. No reference to
    // its fields survives the sole constructor-birth mutation.
    let source = unsafe { &mut *source };
    if source.origin.is_some() {
        return Err(ValueError::CommandProtocolUnavailable(
            "original Source constructor already selected",
        ));
    }
    let origin = Rc::new(NativeJimSourceOrigin {
        root: Rc::new(()),
        original: value,
        parent: None,
        kind: None,
    });
    source.origin = Some(Rc::clone(&origin));
    Ok(origin)
}

pub(crate) fn original_source_origin(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<Option<Rc<NativeJimSourceOrigin>>, ValueError> {
    let source = original_source_backing(value, context)?;
    let Some(source) = source else {
        if let Some(origin) = crate::native_script::source_origin(value, context)? {
            return Ok(Some(origin));
        }
        #[cfg(have_tommath)]
        return crate::expr::native_jim_source_origin(value, context);
        #[cfg(not(have_tommath))]
        return Ok(None);
    };
    // SAFETY: original_source_backing authenticated this exact live primary.
    let source = unsafe { &*source };
    Ok(source
        .origin
        .as_ref()
        .filter(|origin| origin.belongs_to(value))
        .cloned())
}

thread_local! {
    static OBJECT_CONTEXTS: RefCell<HashMap<usize, Weak<NativeJimObjectContext>>> = RefCell::new(HashMap::new());
}

pub(crate) fn bind_context(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<(), ValueError> {
    obj::validate_jim_allocator(value, &context.pool)?;
    OBJECT_CONTEXTS.with(|contexts| {
        let selected = Rc::downgrade(context);
        let mut retained = contexts.borrow_mut();
        if retained
            .get(&(value as usize))
            .is_some_and(|previous| !Weak::ptr_eq(previous, &selected))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim original object interpreter",
            ));
        }
        retained.insert(value as usize, selected);
        Ok(())
    })
}

pub(crate) fn selected_string_protocol(value: *mut TclObj) -> Option<NativeStringProtocol> {
    OBJECT_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .contains_key(&(value as usize))
            .then_some(NativeStringProtocol::Jim084)
    })
}

/// Original weak context transport. It grants no native object reference.
#[derive(Clone)]
pub(crate) struct CapturedObjectContext(Option<Weak<NativeJimObjectContext>>);

pub(crate) fn capture_object_context(original: *mut TclObj) -> CapturedObjectContext {
    CapturedObjectContext(
        OBJECT_CONTEXTS.with(|contexts| contexts.borrow().get(&(original as usize)).cloned()),
    )
}

pub(crate) fn install_captured_object_context(
    target: *mut TclObj,
    original: &CapturedObjectContext,
) {
    OBJECT_CONTEXTS.with(|contexts| {
        let mut contexts = contexts.borrow_mut();
        if let Some(previous) = &original.0 {
            contexts.insert(target as usize, previous.clone());
        } else {
            contexts.remove(&(target as usize));
        }
    });
}

pub(crate) fn copy_context(original: *mut TclObj, duplicate: *mut TclObj) {
    let retained = capture_object_context(original);
    if retained.0.is_some() {
        install_captured_object_context(duplicate, &retained);
    }
}

pub(crate) fn forget_context(value: *mut TclObj) {
    // Retirement cannot recreate a destroyed weak-context map.
    let _ = OBJECT_CONTEXTS.try_with(|contexts| contexts.borrow_mut().remove(&(value as usize)));
}

pub(crate) fn context(value: *mut TclObj) -> Result<Rc<NativeJimObjectContext>, ValueError> {
    OBJECT_CONTEXTS
        .with(|contexts| {
            contexts
                .borrow()
                .get(&(value as usize))
                .and_then(Weak::upgrade)
                .filter(|context| context.is_live())
        })
        .ok_or_else(|| {
            #[cfg(test)]
            if std::env::var_os("TCL_RUNTIME_TRACE_MISSING_JIM_CONTEXT").is_some() {
                eprintln!(
                    "missing original Jim object context for {value:p}:\n{}",
                    std::backtrace::Backtrace::force_capture()
                );
            }
            ValueError::CommandProtocolUnavailable("Jim original source context")
        })
}

/// Inspect the actual Source primary without falling back to an unlocated script.
pub(crate) fn original_source_info(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<Option<NativeJimSourceInfo>, ValueError> {
    // SAFETY: original_source_backing authenticates the exact live primary.
    Ok(original_source_backing(value, context)?.map(|source| unsafe { &*source }.info.clone()))
}

fn original_source_backing(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<Option<*mut NativeJimSource>, ValueError> {
    obj::check_native_liveness(value)?;
    if obj::obj_type_ptr(value) == &JIM_SOURCE_TYPE {
        // SAFETY: the exact primary descriptor owns this live boxed source.
        let source = unsafe { &*(obj::internal_rep(value) as usize as *const NativeJimSource) };
        if !Weak::ptr_eq(&source.context, &Rc::downgrade(context)) {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim original source interpreter",
            ));
        }
        return Ok(Some(
            obj::internal_rep(value) as usize as *mut NativeJimSource
        ));
    }
    Ok(None)
}

pub(crate) fn pin_source_info(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<NativeJimSourceInfo, ValueError> {
    if let Some(info) = original_source_info(value, context)? {
        return Ok(info);
    }
    if let Some(info) = crate::native_script::source_info(value, context)? {
        return Ok(info);
    }
    Ok(NativeJimSourceInfo {
        filename: context.empty_object().clone(),
        line: 1,
    })
}

pub(crate) fn install_source(
    value: *mut TclObj,
    info: NativeJimSourceInfo,
    context: &Rc<NativeJimObjectContext>,
) -> Result<(), ValueError> {
    install_source_from(value, info, context, None)
}

pub(crate) fn install_source_from(
    value: *mut TclObj,
    info: NativeJimSourceInfo,
    context: &Rc<NativeJimObjectContext>,
    parent: Option<(
        &Rc<NativeJimSourceOrigin>,
        tcl_syntax::jim_script_objects::JimScriptObjectKind,
    )>,
) -> Result<(), ValueError> {
    if !obj::has_string_rep(value) || value == info.filename.as_ptr() || obj::is_shared(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim original Source storage",
        ));
    }
    bind_context(value, context)?;
    let backing = Box::new(NativeJimSource {
        info,
        context: Rc::downgrade(context),
        origin: parent.map(|(parent, kind)| {
            Rc::new(NativeJimSourceOrigin {
                root: Rc::clone(&parent.root),
                original: value,
                parent: Some(Rc::clone(parent)),
                kind: Some(kind),
            })
        }),
    });
    obj::change_type(
        value,
        &JIM_SOURCE_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
    Ok(())
}

pub(crate) static JIM_SOURCE_TYPE: TclObjType = TclObjType {
    name: c"source".as_ptr(),
    free_int_rep_proc: Some(source_free),
    dup_int_rep_proc: Some(source_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};

extern "C" fn source_free(value: *mut TclObj) {
    // SAFETY: the Source primary owns exactly this allocation.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut NativeJimSource
        ))
    };
}

extern "C" fn source_dup(original: *mut TclObj, duplicate: *mut TclObj) {
    // SAFETY: the original Source primary owns this live backing.
    let original = unsafe { &*(obj::internal_rep(original) as usize as *const NativeJimSource) };
    let backing = Box::new(NativeJimSource {
        info: original.info.clone(),
        context: original.context.clone(),
        origin: None,
    });
    if let Some(context) = original.context.upgrade() {
        bind_context(duplicate, &context).expect("fresh Source duplicate retains original context");
    }
    obj::change_type(
        duplicate,
        &JIM_SOURCE_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
}

#[cfg(test)]
#[path = "native_source_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) fn context_map_available_for_retirement_test() -> bool {
    OBJECT_CONTEXTS.try_with(|_| ()).is_ok()
}
