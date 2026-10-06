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
        let empty = Owned::fresh(obj::new_string_bytes(b""));
        let context = Rc::new(Self {
            numeric_host: RefCell::new(None),
            live: std::cell::Cell::new(true),
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
        Ok(context)
    }

    pub(crate) fn select_numeric_host(&self, host: Rc<dyn tcl_platform::Host>) {
        *self.numeric_host.borrow_mut() = Some(host);
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

    pub(crate) fn is_live(&self) -> bool {
        self.live.get()
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
}

thread_local! {
    static OBJECT_CONTEXTS: RefCell<HashMap<usize, Weak<NativeJimObjectContext>>> = RefCell::new(HashMap::new());
}

pub(crate) fn bind_context(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<(), ValueError> {
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

pub(crate) fn copy_context(original: *mut TclObj, duplicate: *mut TclObj) {
    OBJECT_CONTEXTS.with(|contexts| {
        let previous = contexts.borrow().get(&(original as usize)).cloned();
        if let Some(previous) = previous {
            contexts.borrow_mut().insert(duplicate as usize, previous);
        }
    });
}

pub(crate) fn forget_context(value: *mut TclObj) {
    OBJECT_CONTEXTS.with(|contexts| contexts.borrow_mut().remove(&(value as usize)));
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
        .ok_or(ValueError::CommandProtocolUnavailable(
            "Jim original source context",
        ))
}

pub(crate) fn pin_source_info(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<NativeJimSourceInfo, ValueError> {
    if obj::obj_type_ptr(value) == &JIM_SOURCE_TYPE {
        // SAFETY: the exact primary descriptor owns this live boxed source.
        let source = unsafe { &*(obj::internal_rep(value) as usize as *const NativeJimSource) };
        if !Weak::ptr_eq(&source.context, &Rc::downgrade(context)) {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim original source interpreter",
            ));
        }
        return Ok(source.info.clone());
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
    if !obj::has_string_rep(value) || value == info.filename.as_ptr() || obj::is_shared(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim original Source storage",
        ));
    }
    bind_context(value, context)?;
    let backing = Box::new(NativeJimSource {
        info,
        context: Rc::downgrade(context),
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
