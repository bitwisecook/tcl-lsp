// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim Subst/DictSubst execution over original prepared token objects.

use super::{Code, Interp};
use crate::{
    native_script::{self, NativeJimScript},
    native_source,
    native_substitution::{self, BorrowedInterpolationName},
    obj::{self, Owned, TclObj},
};
use std::rc::Rc;
use tcl_syntax::{
    jim_script_objects::JimScriptObjectKind, native_string::NativeStringProtocol, value::ValueError,
};

impl Interp {
    /// Jim_SubstObj prepares the original before acquiring the real parent
    /// reference. Its returned object is borrowed/refcount zero, not a result
    /// publication, and Subst never reinstalls a shimmered parent's cache.
    pub(crate) fn substitute_native_jim_original(
        &mut self,
        original: *mut TclObj,
        flags: u8,
    ) -> Result<*mut TclObj, Code> {
        let context = self
            .native_jim_object_context()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let backing =
            native_script::prepare_substitution(original, &context, self.lexer_config(), flags)
                .map_err(|error| self.report_cmd_error(error.into()))?;
        let _lease = native_script::activate_substitution(Owned::retain(original), &backing);
        let mut values = Vec::with_capacity(backing.storage.len());
        let mut end = 0;
        let mut skipped_first = false;
        while end < backing.storage.len() {
            let (kind, token) = backing
                .storage
                .interpolation_token(end)
                .expect("original substitution token");
            let value = match kind {
                Some(kind) => self.eval_native_jim_token(JimScriptObjectKind::Source(kind), token),
                // Actual zero-flag ordinary Script reuse reaches the native
                // LINE/WORD default guest-error branch without reparsing.
                None => Err(Code::Error),
            };
            let value = match value {
                Ok(value) => value,
                Err(Code::Return) => Owned::retain(self.result_obj()),
                Err(Code::Break) if flags & 128 != 0 => break,
                Err(Code::Continue) if flags & 128 != 0 => {
                    skipped_first |= end == 0;
                    end += 1;
                    continue;
                }
                Err(code @ (Code::Break | Code::Continue)) => {
                    return Err(self.error(if code == Code::Break {
                        b"invoked \"break\" outside of a loop"
                    } else {
                        b"invoked \"continue\" outside of a loop"
                    }));
                }
                Err(code) => return Err(code),
            };
            // Owned is the genuine intv reference, acquired BEFORE Jim_String.
            crate::dict::native_object_bytes(value.as_ptr(), NativeStringProtocol::Jim084)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            values.push(value);
            end += 1;
        }
        let value = if end == 1 && values.len() == 1 {
            values.pop().expect("native single-token return")
        } else {
            self.interpolate_native_jim_values_with_first(&backing, 0..end, values, !skipped_first)?
        };
        Ok(value.into_native_unowned())
    }

    pub(super) fn interpolate_native_jim_values(
        &mut self,
        backing: &Rc<NativeJimScript>,
        tokens: std::ops::Range<usize>,
        values: Vec<Owned>,
    ) -> Result<Owned, Code> {
        self.interpolate_native_jim_values_with_first(backing, tokens, values, true)
    }

    fn interpolate_native_jim_values_with_first(
        &mut self,
        backing: &Rc<NativeJimScript>,
        tokens: std::ops::Range<usize>,
        values: Vec<Owned>,
        first_present: bool,
    ) -> Result<Owned, Code> {
        let context = self
            .native_jim_object_context()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let mut bytes = Vec::new();
        for value in &values {
            bytes.extend_from_slice(
                &crate::dict::native_object_bytes(value.as_ptr(), NativeStringProtocol::Jim084)
                    .map_err(|error| self.report_cmd_error(error.into()))?,
            );
        }
        let result = Owned::fresh(obj::new_string_bytes(&bytes));
        native_source::bind_context(result.as_ptr(), &context)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let optimized = tokens.len() == 4
            && values.len() == 4
            && backing
                .storage
                .interpolation_token(tokens.start)
                .is_some_and(|(kind, _)| kind == Some(tcl_lexer::JimScriptTokenKind::Escaped))
            && backing
                .storage
                .interpolation_token(tokens.start + 1)
                .is_some_and(|(kind, _)| kind == Some(tcl_lexer::JimScriptTokenKind::Escaped))
            && backing
                .storage
                .interpolation_token(tokens.start + 2)
                .is_some_and(|(kind, _)| kind == Some(tcl_lexer::JimScriptTokenKind::Variable));
        if optimized {
            native_substitution::install_interpolated(
                result.as_ptr(),
                BorrowedInterpolationName::Script {
                    backing: Rc::downgrade(backing),
                    index: tokens.start,
                },
                values[2].as_ptr(),
                &context,
            )
            .map_err(|error| self.report_cmd_error(error.into()))?;
        } else if let Some(first) = values.first().filter(|first| {
            first_present && obj::obj_type_ptr(first.as_ptr()) == &native_source::JIM_SOURCE_TYPE
        }) {
            let info = native_source::pin_source_info(first.as_ptr(), &context)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            native_source::install_source(result.as_ptr(), info, &context)
                .map_err(|error| self.report_cmd_error(error.into()))?;
        }
        Ok(result)
    }

    pub(crate) fn expand_native_jim_dictionary_substitution(
        &mut self,
        original: *mut TclObj,
    ) -> Result<*mut TclObj, Code> {
        let context = self
            .native_jim_object_context()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        native_substitution::ensure_dictionary_substitution(original, &context)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let (name, key) =
            native_substitution::with_dictionary_substitution(original, |name, key| (name, key))
                .expect("prepared original dictionary substitution");
        // Parent's active Script/tuple owns children; no new name reference.
        let key = self.substitute_native_jim_original(key, 0)?;
        let key = Owned::retain(key); // actual JimExpandDictSugar key reference
        let root = self.read_original_named_variable(name)?;
        if let Err(error) = crate::dict::ensure_dict_native(root, NativeStringProtocol::Jim084) {
            if error.native_access_refusal().is_some() {
                return Err(self.report_cmd_error(error.into()));
            }
            return Err(self.native_jim_dictionary_read_error(name, key.as_ptr(), true));
        }
        let bytes = crate::dict::native_object_bytes(key.as_ptr(), NativeStringProtocol::Jim084)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        match crate::dict::dict_get(root, &bytes) {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(self.native_jim_dictionary_read_error(name, key.as_ptr(), false)),
            Err(_) => Err(self.native_jim_dictionary_read_error(name, key.as_ptr(), true)),
        }
    }

    pub(super) fn native_jim_dictionary_read_error(
        &mut self,
        name: *mut TclObj,
        key: *mut TclObj,
        malformed: bool,
    ) -> Code {
        let bytes = (|| -> Result<Vec<u8>, ValueError> {
            let mut bytes = b"can't read \"".to_vec();
            bytes.extend_from_slice(&crate::dict::native_object_bytes(
                name,
                NativeStringProtocol::Jim084,
            )?);
            bytes.push(b'(');
            bytes.extend_from_slice(&crate::dict::native_object_bytes(
                key,
                NativeStringProtocol::Jim084,
            )?);
            bytes.extend_from_slice(if malformed {
                b")\": variable isn't array"
            } else {
                b")\": no such element in array"
            });
            Ok(bytes)
        })();
        match bytes {
            Ok(bytes) => self.error(&bytes),
            Err(error) => self.report_cmd_error(error.into()),
        }
    }
}
