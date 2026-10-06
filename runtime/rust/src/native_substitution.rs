// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim's original dictionary-substitution tuple and borrowed interpolation name.

use crate::{
    native_script::NativeJimScript,
    native_source::{self, NativeJimObjectContext},
    obj::{self, Owned, TclObj, TclObjType},
};
use std::rc::{Rc, Weak};
use tcl_syntax::{native_jim_substitution::dictionary_substitution_extents, value::ValueError};

/// An actual caller-owned token inventory. This weak projection acquires no
/// native name reference; conversion must pin the original before retirement.
#[derive(Clone)]
pub(crate) enum BorrowedInterpolationName {
    Script {
        backing: Weak<NativeJimScript>,
        index: usize,
    },
    #[cfg(test)]
    Inventory {
        backing: Weak<Vec<Owned>>,
        index: usize,
    },
}
impl BorrowedInterpolationName {
    fn retain_original(&self) -> Result<Owned, ValueError> {
        match self {
            Self::Script { backing, index } => {
                let backing = backing
                    .upgrade()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "retired borrowed Jim interpolation Script",
                    ))?;
                if backing.in_use.get() == 0 {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "retired borrowed Jim interpolation tokens",
                    ));
                }
                let (_, original) = backing.storage.interpolation_token(*index).ok_or(
                    ValueError::CommandProtocolUnavailable("Jim interpolation original token"),
                )?;
                Ok(original.clone())
            }
            #[cfg(test)]
            Self::Inventory { backing, index } => backing
                .upgrade()
                .and_then(|tokens| tokens.get(*index).cloned())
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "retired Jim caller token inventory",
                )),
        }
    }
}

struct DictionarySubstitution {
    name: Owned,
    index: Owned,
    context: Weak<NativeJimObjectContext>,
}
struct Interpolated {
    name: BorrowedInterpolationName,
    index: Owned,
    context: Weak<NativeJimObjectContext>,
}

pub(crate) fn install_interpolated(
    value: *mut TclObj,
    name: BorrowedInterpolationName,
    index: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<(), ValueError> {
    native_source::bind_context(value, context)?;
    native_source::bind_context(index, context)?;
    if !obj::has_string_rep(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim interpolated resident bytes",
        ));
    }
    let backing = Interpolated {
        name,
        index: Owned::retain(index),
        context: Rc::downgrade(context),
    };
    obj::change_type(
        value,
        &INTERPOLATED_TYPE,
        Box::into_raw(Box::new(backing)) as usize as u64,
    );
    Ok(())
}

pub(crate) fn ensure_dictionary_substitution(
    value: *mut TclObj,
    context: &Rc<NativeJimObjectContext>,
) -> Result<(), ValueError> {
    native_source::bind_context(value, context)?;
    if obj::obj_type_ptr(value) == &DICTIONARY_SUBSTITUTION_TYPE {
        // SAFETY: the exact descriptor owns this live boxed tuple.
        let stored =
            unsafe { &*(obj::internal_rep(value) as usize as *const DictionarySubstitution) };
        if !Weak::ptr_eq(&stored.context, &Rc::downgrade(context)) {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim dictionary substitution original interpreter",
            ));
        }
        return Ok(());
    }
    let (name, index) = if obj::obj_type_ptr(value) == &INTERPOLATED_TYPE {
        // SAFETY: the exact descriptor owns this live boxed optimization.
        let stored = unsafe { &*(obj::internal_rep(value) as usize as *const Interpolated) };
        if !Weak::ptr_eq(&stored.context, &Rc::downgrade(context)) {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim interpolation original interpreter",
            ));
        }
        // Native SetDictSubstFromAny pins BOTH children before freeing the old
        // optimization. The original name is borrowed until this exact point.
        (stored.name.retain_original()?, stored.index.clone())
    } else {
        let bytes = crate::dict::native_object_bytes(
            value,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )?;
        let extents = dictionary_substitution_extents(&bytes)?;
        let name = Owned::fresh(obj::new_string_bytes(&bytes[extents.name]));
        let index = Owned::fresh(obj::new_string_bytes(&bytes[extents.key]));
        native_source::bind_context(name.as_ptr(), context)?;
        native_source::bind_context(index.as_ptr(), context)?;
        (name, index)
    };
    let stored = DictionarySubstitution {
        name,
        index,
        context: Rc::downgrade(context),
    };
    obj::change_type(
        value,
        &DICTIONARY_SUBSTITUTION_TYPE,
        Box::into_raw(Box::new(stored)) as usize as u64,
    );
    Ok(())
}

/// Borrow exact tuple members. The callback must not invoke guest code or
/// replace the parent's primary; take lifetime/owned pins before doing either.
pub(crate) fn with_dictionary_substitution<R>(
    value: *mut TclObj,
    with: impl FnOnce(*mut TclObj, *mut TclObj) -> R,
) -> Option<R> {
    if obj::obj_type_ptr(value) != &DICTIONARY_SUBSTITUTION_TYPE {
        return None;
    }
    // SAFETY: descriptor identity establishes the owned live tuple.
    let stored = unsafe { &*(obj::internal_rep(value) as usize as *const DictionarySubstitution) };
    Some(with(stored.name.as_ptr(), stored.index.as_ptr()))
}

pub(crate) fn cache_snapshot(
    value: *mut TclObj,
) -> Option<tcl_syntax::native_object::NativeObjectCacheSnapshot> {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    match obj::obj_type_ptr(value) {
        kind if kind == &DICTIONARY_SUBSTITUTION_TYPE => Some(Cache::JimDictionarySubstitution),
        kind if kind == &INTERPOLATED_TYPE => Some(Cache::JimInterpolated),
        _ => None,
    }
}

static DICTIONARY_SUBSTITUTION_TYPE: TclObjType = TclObjType {
    name: c"dict-substitution".as_ptr(),
    free_int_rep_proc: Some(dictionary_free),
    dup_int_rep_proc: Some(dictionary_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};
static INTERPOLATED_TYPE: TclObjType = TclObjType {
    name: c"interpolated".as_ptr(),
    free_int_rep_proc: Some(interpolated_free),
    dup_int_rep_proc: Some(interpolated_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};
extern "C" fn dictionary_free(value: *mut TclObj) {
    // SAFETY: the exact type owns this boxed tuple once.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut DictionarySubstitution
        ))
    };
}
extern "C" fn dictionary_dup(original: *mut TclObj, duplicate: *mut TclObj) {
    // SAFETY: the exact source descriptor owns the live tuple.
    let stored =
        unsafe { &*(obj::internal_rep(original) as usize as *const DictionarySubstitution) };
    let cloned = DictionarySubstitution {
        name: stored.name.clone(),
        index: stored.index.clone(),
        context: stored.context.clone(),
    };
    obj::change_type(
        duplicate,
        &DICTIONARY_SUBSTITUTION_TYPE,
        Box::into_raw(Box::new(cloned)) as usize as u64,
    );
}
extern "C" fn interpolated_free(value: *mut TclObj) {
    // SAFETY: the exact type owns this boxed optimization once.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut Interpolated
        ))
    };
}
extern "C" fn interpolated_dup(original: *mut TclObj, duplicate: *mut TclObj) {
    // SAFETY: the exact source descriptor owns the live optimization.
    let stored = unsafe { &*(obj::internal_rep(original) as usize as *const Interpolated) };
    let cloned = Interpolated {
        name: stored.name.clone(),
        index: stored.index.clone(),
        context: stored.context.clone(),
    };
    obj::change_type(
        duplicate,
        &INTERPOLATED_TYPE,
        Box::into_raw(Box::new(cloned)) as usize as u64,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        interp::{Code, Interp},
        native_source::NativeJimSourceInfo,
    };
    use std::fmt::Write;
    use tcl_syntax::native_jim_substitution::JimScriptStorage;

    fn refs(value: *mut TclObj) -> obj::TclSize {
        // SAFETY: every observation borrows a genuine owned native object.
        unsafe { (*value).ref_count }
    }
    fn native_type(value: *mut TclObj) -> &'static str {
        let descriptor = obj::obj_type_ptr(value);
        if descriptor.is_null() {
            return "NULL";
        }
        // SAFETY: native descriptors are static and names NUL terminated.
        unsafe {
            std::ffi::CStr::from_ptr((*descriptor).name)
                .to_str()
                .unwrap()
        }
    }
    fn hex(value: *mut TclObj) -> String {
        obj::bytes_of(value)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
    fn kind(kind: tcl_lexer::JimScriptTokenKind) -> i32 {
        match kind {
            tcl_lexer::JimScriptTokenKind::String => 1,
            tcl_lexer::JimScriptTokenKind::Escaped => 2,
            tcl_lexer::JimScriptTokenKind::Variable => 3,
            tcl_lexer::JimScriptTokenKind::IndexedVariable => 4,
            tcl_lexer::JimScriptTokenKind::Command => 5,
            tcl_lexer::JimScriptTokenKind::Expression => 6,
            _ => panic!("native Subst real token purpose"),
        }
    }
    fn children(
        observed: &mut String,
        stage: &str,
        value: *mut TclObj,
        filename: *mut TclObj,
        original_name: *mut TclObj,
        original_index: *mut TclObj,
        interpolated: bool,
    ) {
        let (name, index) = if interpolated {
            // SAFETY: exact Interpolated descriptor installed by this producer;
            // the actual caller token inventory and variable still own children.
            let stored = unsafe { &*(obj::internal_rep(value) as usize as *const Interpolated) };
            let name = match &stored.name {
                BorrowedInterpolationName::Inventory { backing, index } => {
                    backing.upgrade().unwrap()[*index].as_ptr()
                }
                _ => panic!("direct caller original token inventory"),
            };
            (name, stored.index.as_ptr())
        } else {
            with_dictionary_substitution(value, |name, index| (name, index)).unwrap()
        };
        writeln!(
            observed,
            "CHILDREN\t{stage}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            native_type(value),
            usize::from(name == original_name),
            usize::from(index == original_index),
            refs(name),
            refs(index),
            refs(filename),
            native_type(name),
            native_type(index),
            hex(name),
            hex(index)
        )
        .unwrap();
    }
    #[test]
    fn original_substitution_storage_matches_205_native_windows() {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        let context = interp.native_jim_object_context().unwrap();
        let config = interp.lexer_config();
        let sources: [&[u8]; 4] = [
            b"a\n$k[set x X]\\n{q};z",
            b"A\0B$k[set x X]",
            b"${x\ny}P\nQ",
            b"d($k)",
        ];
        let mut observed = String::new();
        for (case, source) in sources.into_iter().enumerate() {
            for flags in 0..8 {
                let filename = Owned::fresh(obj::new_string_bytes(b"FILE"));
                let original = Owned::fresh(obj::new_string_bytes(source));
                native_source::install_source(
                    original.as_ptr(),
                    NativeJimSourceInfo {
                        filename: filename.clone(),
                        line: 7,
                    },
                    &context,
                )
                .unwrap();
                writeln!(
                    observed,
                    "BEFORE\t{case}\t{flags}\t{}\t{}\t{}",
                    native_type(original.as_ptr()),
                    refs(original.as_ptr()),
                    refs(filename.as_ptr())
                )
                .unwrap();
                let backing = crate::native_script::prepare_substitution(
                    original.as_ptr(),
                    &context,
                    config,
                    flags,
                )
                .unwrap();
                let JimScriptStorage::Substitution {
                    objects,
                    filename: script_file,
                } = &backing.storage
                else {
                    panic!("fresh Subst storage")
                };
                writeln!(
                    observed,
                    "SUBST\t{case}\t{flags}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    native_type(original.as_ptr()),
                    objects.tokens().len(),
                    objects.flags,
                    backing.in_use.get(),
                    usize::from(script_file.as_ptr() == context.empty_object().as_ptr()),
                    refs(filename.as_ptr()),
                    refs(original.as_ptr())
                )
                .unwrap();
                for (index, token) in objects.tokens().iter().enumerate() {
                    let bytes = obj::bytes_of(token.value.as_ptr());
                    writeln!(
                        observed,
                        "TOKEN\t{case}\t{flags}\t{index}\t{}\t{}\t{}\t{}\t{}",
                        kind(token.kind),
                        native_type(token.value.as_ptr()),
                        refs(token.value.as_ptr()),
                        bytes.len(),
                        hex(token.value.as_ptr())
                    )
                    .unwrap();
                }
                let duplicate = Owned::fresh(obj::duplicate(original.as_ptr()));
                writeln!(
                    observed,
                    "DUP\t{case}\t{flags}\t{}\t{}\t{}",
                    native_type(duplicate.as_ptr()),
                    usize::from(obj::has_string_rep(duplicate.as_ptr())),
                    refs(filename.as_ptr())
                )
                .unwrap();
                drop(duplicate);
                drop(original);
                drop(backing);
                writeln!(
                    observed,
                    "RETIRE\t{case}\t{flags}\t{}",
                    refs(filename.as_ptr())
                )
                .unwrap();
            }
        }
        let setup = Owned::fresh(obj::new_string_bytes(
            b"set k KEY; set d [dict create KEY VALUE]",
        ));
        assert_eq!(interp.eval_body_obj(setup.as_ptr()), Code::Ok);
        drop(setup);
        let filename = Owned::fresh(obj::new_string_bytes(b"FILE"));
        let tokens: Rc<Vec<Owned>> = Rc::new(
            [b"d".as_slice(), b"(", b"k", b")"]
                .into_iter()
                .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)))
                .collect(),
        );
        native_source::install_source(
            tokens[0].as_ptr(),
            NativeJimSourceInfo {
                filename: filename.clone(),
                line: 7,
            },
            &context,
        )
        .unwrap();
        let index = interp
            .read_original_named_variable(tokens[2].as_ptr())
            .unwrap();
        writeln!(
            observed,
            "OPT_BEFORE\t{}\t{}\t{}",
            refs(tokens[0].as_ptr()),
            refs(index),
            refs(filename.as_ptr())
        )
        .unwrap();
        let value = Owned::fresh(obj::new_string_bytes(b"d(KEY)"));
        install_interpolated(
            value.as_ptr(),
            BorrowedInterpolationName::Inventory {
                backing: Rc::downgrade(&tokens),
                index: 0,
            },
            index,
            &context,
        )
        .unwrap();
        children(
            &mut observed,
            "interpolated",
            value.as_ptr(),
            filename.as_ptr(),
            tokens[0].as_ptr(),
            index,
            true,
        );
        let duplicate = Owned::fresh(obj::duplicate(value.as_ptr()));
        children(
            &mut observed,
            "interpolated-duplicate",
            duplicate.as_ptr(),
            filename.as_ptr(),
            tokens[0].as_ptr(),
            index,
            true,
        );
        drop(duplicate);
        ensure_dictionary_substitution(value.as_ptr(), &context).unwrap();
        children(
            &mut observed,
            "dict-converted",
            value.as_ptr(),
            filename.as_ptr(),
            tokens[0].as_ptr(),
            index,
            false,
        );
        let result = interp
            .expand_native_jim_dictionary_substitution(value.as_ptr())
            .unwrap();
        writeln!(
            observed,
            "EXPAND\t1\t{}\t{}\t{}",
            native_type(value.as_ptr()),
            refs(filename.as_ptr()),
            hex(result)
        )
        .unwrap();
        children(
            &mut observed,
            "expanded",
            value.as_ptr(),
            filename.as_ptr(),
            tokens[0].as_ptr(),
            index,
            false,
        );
        drop(value);
        drop(tokens);
        writeln!(observed, "OPT_RETIRE\t{}", refs(filename.as_ptr())).unwrap();
        let original = Owned::fresh(obj::new_string_bytes(b"set x 1"));
        let ordinary = crate::native_script::prepare(original.as_ptr(), &context, config).unwrap();
        let reused =
            crate::native_script::prepare_substitution(original.as_ptr(), &context, config, 0)
                .unwrap();
        writeln!(
            observed,
            "REUSE\t{}\t{}\t{}",
            usize::from(Rc::ptr_eq(&ordinary, &reused)),
            reused.storage.flags(),
            reused.storage.len()
        )
        .unwrap();
        let reparsed =
            crate::native_script::prepare_substitution(original.as_ptr(), &context, config, 4)
                .unwrap();
        writeln!(
            observed,
            "REPARSE\t{}\t{}\t{}",
            usize::from(Rc::ptr_eq(&ordinary, &reparsed)),
            reparsed.storage.flags(),
            reparsed.storage.len()
        )
        .unwrap();
        assert_eq!(observed.lines().count(), 205);
        assert_eq!(
            observed,
            include_str!(
                "../../../rust/tcl-syntax/testdata/native_jim_indexed_substitution/normal.tsv"
            )
        );
    }
}
