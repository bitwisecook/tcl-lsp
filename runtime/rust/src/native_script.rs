// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original Jim Script tokens and their filename and active backing ownership.

use crate::{
    native_source::{self, NativeJimObjectContext, NativeJimSourceInfo},
    obj::{self, Owned, TclObj, TclObjType},
};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
use tcl_syntax::{
    jim_script_objects::{JimScriptObjectConstruction, JimScriptObjects},
    native_jim_substitution::{JimOrdinaryScript, JimScriptStorage, JimSubstitutionObjects},
    value::ValueError,
};

pub(crate) struct NativeJimScript {
    pub(crate) storage: JimScriptStorage<Owned>,
    pub(crate) in_use: Cell<usize>,
    context: Weak<NativeJimObjectContext>,
}

impl NativeJimScript {
    pub(crate) fn ordinary(&self) -> Result<&JimOrdinaryScript<Owned>, ValueError> {
        self.storage.ordinary()
    }
}

struct ScriptHeader(Rc<NativeJimScript>);
impl Drop for ScriptHeader {
    fn drop(&mut self) {
        self.0.in_use.set(self.0.in_use.get() - 1);
    }
}

pub(crate) struct NativeJimScriptLease {
    original: Owned,
    header: Option<ScriptHeader>,
    restore_primary: bool,
}
impl Drop for NativeJimScriptLease {
    fn drop(&mut self) {
        if let Some(header) = self.header.take().filter(|_| self.restore_primary) {
            install_header(self.original.as_ptr(), header);
        }
    }
}

pub(crate) fn activate(original: Owned, backing: &Rc<NativeJimScript>) -> NativeJimScriptLease {
    backing.in_use.set(
        backing
            .in_use
            .get()
            .checked_add(1)
            .expect("native Script use count"),
    );
    NativeJimScriptLease {
        original,
        header: Some(ScriptHeader(Rc::clone(backing))),
        restore_primary: true,
    }
}

pub(crate) fn source_info(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<Option<NativeJimSourceInfo>, ValueError> {
    if obj::obj_type_ptr(value) != &JIM_SCRIPT_TYPE {
        return Ok(None);
    }
    // SAFETY: the exact Script descriptor owns this live header.
    let header = unsafe { &*(obj::internal_rep(value) as usize as *const ScriptHeader) };
    if !Weak::ptr_eq(&header.0.context, &Rc::downgrade(context)) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim Script original interpreter",
        ));
    }
    let ordinary = header.0.ordinary()?;
    Ok(Some(NativeJimSourceInfo {
        filename: ordinary.filename.clone(),
        line: ordinary.first_line,
    }))
}

pub(crate) fn prepare(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
    config: tcl_lexer::LexerConfig,
) -> Result<Rc<NativeJimScript>, ValueError> {
    native_source::bind_context(value, context)?;
    let original = if value == context.empty_object().as_ptr() {
        context.null_script_object().as_ptr()
    } else {
        value
    };
    if obj::obj_type_ptr(original) == &JIM_SCRIPT_TYPE {
        // SAFETY: the exact Script descriptor owns this live header.
        let header = unsafe { &*(obj::internal_rep(original) as usize as *const ScriptHeader) };
        if !Weak::ptr_eq(&header.0.context, &Rc::downgrade(context)) {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim Script original interpreter",
            ));
        }
        if header.0.storage.flags() != 0 {
            return Err(ValueError::NativeFatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::JimSubstitutionScriptReentry,
            ));
        }
        return Ok(Rc::clone(&header.0));
    }
    let bytes = crate::dict::native_object_bytes(
        original,
        tcl_syntax::native_string::NativeStringProtocol::Jim084,
    )?;
    let mut info = native_source::pin_source_info(original, context)?;
    let roster =
        tcl_lexer::jim_script_tokens(&tcl_lexer::SourceImage::native(bytes.as_slice()), config)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim Script original token roster")
            })?;
    let objects = JimScriptObjects::prepare(&roster, |construction| match construction {
        JimScriptObjectConstruction::Line { argc, line_delta } => {
            let value = Owned::fresh(obj::new_string_bytes(b""));
            native_source::bind_context(value.as_ptr(), context)?;
            let line = info
                .line
                .wrapping_add(i32::from_ne_bytes(line_delta.to_ne_bytes()));
            let payload = (u64::from(u32::from_ne_bytes(argc.to_ne_bytes())) << 32)
                | u64::from(u32::from_ne_bytes(line.to_ne_bytes()));
            obj::change_type(value.as_ptr(), &JIM_SCRIPT_LINE_TYPE, payload);
            Ok(value)
        }
        JimScriptObjectConstruction::Word(count) => {
            let value = Owned::fresh(obj::new_wide_int_obj(i64::from(count)));
            native_source::bind_context(value.as_ptr(), context)?;
            Ok(value)
        }
        JimScriptObjectConstruction::Source {
            bytes, line_delta, ..
        } => {
            let value = Owned::fresh(obj::new_string_bytes(bytes));
            native_source::install_source(
                value.as_ptr(),
                NativeJimSourceInfo {
                    filename: info.filename.clone(),
                    line: info
                        .line
                        .wrapping_add(i32::from_ne_bytes(line_delta.to_ne_bytes())),
                },
                context,
            )?;
            Ok(value)
        }
    })?;
    let linenr = match objects.completeness_line {
        tcl_lexer::JimScriptLine::Original(delta) => info
            .line
            .wrapping_add(i32::from_ne_bytes(delta.to_ne_bytes())),
        tcl_lexer::JimScriptLine::Zero => 0,
    };
    let baseline = info.line;
    info.line = info
        .line
        .wrapping_add(i32::from_ne_bytes(objects.first_line_delta.to_ne_bytes()));
    let backing = Rc::new(NativeJimScript {
        storage: JimScriptStorage::Ordinary(JimOrdinaryScript {
            objects,
            filename: info.filename,
            first_line: info.line,
            baseline,
            linenr: Cell::new(linenr),
        }),
        in_use: Cell::new(1),
        context: Rc::downgrade(context),
    });
    install_header(original, ScriptHeader(Rc::clone(&backing)));
    Ok(backing)
}

pub(crate) fn prepare_substitution(
    original: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
    config: tcl_lexer::LexerConfig,
    flags: u8,
) -> Result<Rc<NativeJimScript>, ValueError> {
    native_source::bind_context(original, context)?;
    if obj::obj_type_ptr(original) == &JIM_SCRIPT_TYPE {
        // SAFETY: this exact Script descriptor owns the live header.
        let header = unsafe { &*(obj::internal_rep(original) as usize as *const ScriptHeader) };
        if !Weak::ptr_eq(&header.0.context, &Rc::downgrade(context)) {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim substitution original interpreter",
            ));
        }
        if header.0.storage.flags() == flags {
            return Ok(Rc::clone(&header.0));
        }
    }
    let bytes = crate::dict::native_object_bytes(
        original,
        tcl_syntax::native_string::NativeStringProtocol::Jim084,
    )?;
    let roster = tcl_lexer::jim_subst_tokens(
        &tcl_lexer::SourceImage::native(bytes.as_slice()),
        config,
        flags,
    )
    .map_err(|_| {
        ValueError::CommandProtocolUnavailable("Jim substitution original token roster")
    })?;
    let objects = JimSubstitutionObjects::prepare(&roster, |bytes| {
        let value = Owned::fresh(obj::new_string_bytes(bytes));
        native_source::bind_context(value.as_ptr(), context)?;
        Ok(value)
    })?;
    let backing = Rc::new(NativeJimScript {
        storage: JimScriptStorage::Substitution {
            objects,
            filename: context.empty_object().clone(),
        },
        in_use: Cell::new(1),
        context: Rc::downgrade(context),
    });
    install_header(original, ScriptHeader(Rc::clone(&backing)));
    Ok(backing)
}

pub(crate) fn activate_substitution(
    original: Owned,
    backing: &Rc<NativeJimScript>,
) -> NativeJimScriptLease {
    let mut lease = activate(original, backing);
    lease.restore_primary = false;
    lease
}

fn install_header(value: *mut TclObj, header: ScriptHeader) {
    obj::change_type(
        value,
        &JIM_SCRIPT_TYPE,
        Box::into_raw(Box::new(header)) as usize as u64,
    );
}

pub(crate) static JIM_SCRIPT_TYPE: TclObjType = TclObjType {
    name: c"script".as_ptr(),
    free_int_rep_proc: Some(script_free),
    dup_int_rep_proc: Some(script_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};
pub(crate) static JIM_SCRIPT_LINE_TYPE: TclObjType = TclObjType {
    name: c"scriptline".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
extern "C" fn script_free(value: *mut TclObj) {
    // SAFETY: this exact Script descriptor owns the boxed header.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut ScriptHeader
        ))
    };
}
extern "C" fn script_dup(_: *mut TclObj, _: *mut TclObj) {
    // Jim duplicates Script to an untyped string; the resident bytes already copied.
}

pub(crate) fn cache_snapshot(
    value: *mut TclObj,
) -> Option<tcl_syntax::native_object::NativeObjectCacheSnapshot> {
    if obj::obj_type_ptr(value) != &JIM_SCRIPT_TYPE {
        return None;
    }
    // SAFETY: exact descriptor owns the live header.
    let header = unsafe { &*(obj::internal_rep(value) as usize as *const ScriptHeader) };
    Some(
        tcl_syntax::native_object::NativeObjectCacheSnapshot::JimScript {
            flags: header.0.storage.flags(),
            tokens: header.0.storage.len(),
        },
    )
}

pub(crate) fn script_line(value: *mut TclObj) -> Option<(i32, i32)> {
    if obj::obj_type_ptr(value) != &JIM_SCRIPT_LINE_TYPE {
        return None;
    }
    let payload = obj::internal_rep(value);
    Some(((payload >> 32) as u32 as i32, payload as u32 as i32))
}

#[cfg(test)]
#[path = "native_script_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "native_script_execution_tests.rs"]
mod execution_tests;
