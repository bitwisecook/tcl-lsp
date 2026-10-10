// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim's original-name JIM_UNSHARED lookup precedes member integer conversion.

use super::{Code, Interp};
use crate::{
    frame::JimVariableRead,
    native_substitution,
    obj::{self, Owned, TclObj},
};
use tcl_syntax::native_string::NativeStringProtocol;

impl Interp {
    pub(super) fn is_native_jim_dictionary_name(
        &self,
        original: *mut TclObj,
        bytes: &[u8],
    ) -> bool {
        native_substitution::with_dictionary_substitution(original, |_, _| ()).is_some()
            || (bytes.last() == Some(&b')')
                && bytes
                    .split(|byte| *byte == 0)
                    .next()
                    .unwrap_or_default()
                    .contains(&b'('))
    }

    pub(super) fn read_original_named_variable_for_update(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<*mut TclObj>, Code> {
        if let Some(cache) = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?
        {
            match cache.cell.read() {
                Some(JimVariableRead::Scalar(value)) => return Ok(Some(value)),
                Some(JimVariableRead::Link(link)) => {
                    if let Some(result) = self.read_original_jim_link_for_update(&link) {
                        return result;
                    }
                    return Err(self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original Jim update link target",
                        )
                        .into(),
                    ));
                }
                None => {}
            }
        }
        let bytes = self
            .native_object_string_bytes(original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if self.is_native_jim_dictionary_name(original, &bytes) {
            return self.read_native_jim_dictionary_sugar(original, true);
        }
        self.install_original_jim_variable(original, &bytes)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if let Some(cache) = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?
        {
            if let Some(JimVariableRead::Link(link)) = cache.cell.read() {
                if let Some(result) = self.read_original_jim_link_for_update(&link) {
                    return result;
                }
                return Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original Jim update link target",
                    )
                    .into(),
                ));
            }
        }
        // JIM_NONE lookup of a missing ordinary name does not create a guest
        // error or install a primary cache. No original value owner is added.
        Ok(self.var_get(&bytes))
    }

    pub(super) fn read_native_jim_dictionary_sugar(
        &mut self,
        original: *mut TclObj,
        unshared: bool,
    ) -> Result<Option<*mut TclObj>, Code> {
        let context = self
            .native_jim_object_context()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        native_substitution::ensure_dictionary_substitution(original, &context)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let (name, key) =
            native_substitution::with_dictionary_substitution(original, |name, key| (name, key))
                .expect("installed original tuple");
        let root = match self.read_original_named_variable(name) {
            Ok(root) => root,
            Err(_) if !self.host_refusal_pending() => return Ok(None),
            Err(code) => return Err(code),
        };
        if let Err(error) = crate::dict::ensure_dict_native(root, NativeStringProtocol::Jim084) {
            if error.native_access_refusal().is_some() {
                return Err(self.report_cmd_error(error.into()));
            }
            self.native_jim_dictionary_read_error(name, key, true);
            return Ok(None);
        }
        let bytes = crate::dict::native_object_bytes(key, NativeStringProtocol::Jim084)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let member = crate::dict::dict_get(root, &bytes).expect("converted original dictionary");
        let Some(member) = member else {
            self.native_jim_dictionary_read_error(name, key, false);
            return Ok(None);
        };
        if unshared && obj::is_shared(root) {
            // Decide sharing BEFORE the temporary duplicate owner. Duplication
            // retains every original member, then publishes the root before
            // Jim_GetWide/COW observes the selected original member.
            let duplicate = Owned::fresh(obj::duplicate(root));
            self.assign_original_named_variable(name, duplicate.as_ptr())?;
        }
        Ok(Some(member))
    }

    pub(super) fn assign_native_jim_dictionary_sugar(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        let context = self
            .native_jim_object_context()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        native_substitution::ensure_dictionary_substitution(original, &context)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let (name, key) =
            native_substitution::with_dictionary_substitution(original, |name, key| (name, key))
                .expect("installed original tuple");
        let existing = self.read_original_named_variable_for_update(name)?;
        let root = match existing {
            Some(root) if obj::is_shared(root) => {
                crate::value_ops::RuntimeAppendValue::retain(obj::duplicate(root))
            }
            Some(root) => crate::value_ops::RuntimeAppendValue::borrowed(root),
            None => {
                let root = Owned::fresh(
                    crate::dict::new_dict_obj_native(&[], None, NativeStringProtocol::Jim084)
                        .map_err(|error| self.report_cmd_error(error.into()))?,
                );
                // Native SetDictKeysVector publishes a genuinely new root
                // before converting or adding the key.
                self.assign_original_named_variable(name, root.as_ptr())?;
                let pointer = root.as_ptr();
                drop(root);
                crate::value_ops::RuntimeAppendValue::borrowed(pointer)
            }
        };
        if let Err(error) =
            crate::dict::ensure_dict_native(root.as_ptr(), NativeStringProtocol::Jim084)
        {
            if error.native_access_refusal().is_some() {
                return Err(self.report_cmd_error(error.into()));
            }
            let name = crate::dict::native_object_bytes(original, NativeStringProtocol::Jim084)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            return Err(crate::builtins::var_error(
                self,
                &name,
                crate::frame::VarError::IsScalar,
            ));
        }
        crate::dict::native_object_bytes(key, NativeStringProtocol::Jim084)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        crate::dict::dict_set(root.as_ptr(), key, value)
            .expect("converted original dictionary setter");
        self.assign_original_named_variable(name, root.as_ptr())?;
        Ok(())
    }

    pub(super) fn unset_native_jim_dictionary_sugar(
        &mut self,
        original: *mut TclObj,
    ) -> Result<bool, Code> {
        let context = self
            .native_jim_object_context()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        native_substitution::ensure_dictionary_substitution(original, &context)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let (name, key) =
            native_substitution::with_dictionary_substitution(original, |name, key| (name, key))
                .expect("installed original tuple");
        let existing = self.read_original_named_variable_for_update(name)?;
        if let Some(existing) = existing {
            let outcome = (|| -> Result<Option<Owned>, tcl_syntax::value::ValueError> {
                // Jim_SetDictKeysVector duplicates before dictionary conversion,
                // and publishes the working root only after removal succeeds.
                let mut root = crate::dict::PreparedNativeDictionary::prepare(
                    Some(existing),
                    NativeStringProtocol::Jim084,
                )?;
                if root.remove_member(key)? {
                    Ok(Some(root.into_value()))
                } else {
                    Ok(None)
                }
            })();
            match outcome {
                Ok(Some(root)) => {
                    self.assign_original_named_variable(name, root.as_ptr())?;
                    self.set_result_bytes(b"");
                    return Ok(true);
                }
                Err(error) if error.native_access_refusal().is_some() => {
                    return Err(self.report_cmd_error(error.into()));
                }
                Ok(None) | Err(_) => {}
            }
        }
        // Native retries the SAME retained parent with JIM_NONE after failure.
        let parent_exists = self
            .read_original_named_variable_for_update(name)?
            .is_some();
        use tcl_syntax::value::ValueOps;
        let bytes = self
            .native_string_bytes(&original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let recipe = self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .expect("selected Jim original unset");
        let message = recipe
            .dictionary_unset_error(&bytes, parent_exists)
            .map_err(|_| {
                self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim original unset diagnostic",
                    )
                    .into(),
                )
            })?;
        Err(self.set_error(&message))
    }

    pub(crate) fn increment_native_jim_original(
        &mut self,
        original: *mut TclObj,
        amount: tcl_cmd_core::native_increment::PreparedLegacyIncrementAmount,
    ) -> Code {
        let current = match self.read_original_named_variable_for_update(original) {
            Ok(current) => current,
            Err(code) => return code,
        };
        let objects = match crate::value_ops::RuntimeLegacyIncrementObjects::selected(self) {
            Ok(objects) => objects,
            Err(error) => return self.report_cmd_error(error),
        };
        let original_value = current.map(crate::value_ops::RuntimeAppendValue::borrowed);
        let sum = match tcl_cmd_core::native_increment::increment_legacy(
            &objects,
            original_value.as_ref(),
            amount,
        ) {
            Ok(sum) => sum,
            Err(error) => return self.report_cmd_error(error),
        };
        if let Err(code) = self.assign_original_named_variable(original, sum.as_ptr()) {
            return code;
        }
        self.set_result(sum.as_ptr());
        Code::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn original_dictionary_unset_matches_nine_native_jim_controls() {
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for row in
            include_str!("../../../../rust/tcl-syntax/tests/data/native_jim_unset/windows.tsv")
                .lines()
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect("jim"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(&decode(fields[1])).as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{}",
                fields[0]
            );
            assert_eq!(interp.result_bytes(), decode(fields[3]), "{}", fields[0]);
            assert!(!interp.host_refusal_pending(), "{}", fields[0]);
            compared += 1;
        }
        assert_eq!(compared, 9);
    }
    fn kind(value: *mut TclObj) -> String {
        let descriptor = obj::obj_type_ptr(value);
        if descriptor.is_null() {
            return "NULL".into();
        }
        // SAFETY: actual descriptor names are static while the header lives.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap()
            .into()
    }
    fn refs(value: *mut TclObj) -> obj::TclSize {
        // SAFETY: test only observes objects held by actual variable/alias/result owners.
        unsafe { (*value).ref_count }
    }
    #[test]
    fn original_dictionary_sugar_increment_matches_native_publication_windows() {
        let mut observed = String::new();
        for case in 0..7 {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let key = Owned::fresh(obj::new_string_bytes(b"k"));
            let member = Owned::fresh(obj::new_string_bytes(if case == 2 || case == 3 {
                b"BAD"
            } else {
                b"4"
            }));
            let member_ptr = member.as_ptr();
            let root = Owned::fresh(
                crate::dict::new_dict_obj_native(
                    &[(key.as_ptr(), member_ptr)],
                    None,
                    NativeStringProtocol::Jim084,
                )
                .unwrap(),
            );
            let root_ptr = root.as_ptr();
            interp.var_set(b"d", root_ptr).unwrap();
            let shared = matches!(case, 1 | 3 | 5 | 6);
            let original_root = if shared {
                Some(root)
            } else {
                drop(root);
                None
            };
            let original_member = if case == 4 {
                Some(member)
            } else {
                drop(member);
                None
            };
            drop(key);
            let argv = [
                Owned::fresh(obj::new_string_bytes(b"incr")),
                Owned::fresh(obj::new_string_bytes(if case == 5 {
                    b"d(missing)"
                } else {
                    b"d(k)"
                })),
                Owned::fresh(if case == 6 {
                    obj::new_string_bytes(b"BAD")
                } else {
                    obj::new_wide_int_obj(1)
                }),
            ];
            writeln!(
                observed,
                "BEFORE\t{case}\t{}\t{}\t{}\t{}\t{}\t{}",
                refs(root_ptr),
                kind(root_ptr),
                usize::from(obj::has_string_rep(root_ptr)),
                refs(member_ptr),
                kind(member_ptr),
                usize::from(obj::has_string_rep(member_ptr))
            )
            .unwrap();
            let code = interp.dispatch(&argv.iter().map(Owned::as_ptr).collect::<Vec<_>>());
            let current_root = interp.var_get(b"d").unwrap();
            let current =
                crate::dict::dict_get(current_root, if case == 5 { b"missing" } else { b"k" })
                    .unwrap()
                    .unwrap();
            writeln!(
                observed,
                "AFTER\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                code.as_int(),
                usize::from(root_ptr == current_root),
                kind(current_root),
                refs(current_root),
                usize::from(member_ptr == current),
                kind(current),
                refs(current),
                usize::from(obj::has_string_rep(current))
            )
            .unwrap();
            if shared {
                writeln!(
                    observed,
                    "OLD\t{case}\t{}\t{}\t{}",
                    refs(member_ptr),
                    kind(member_ptr),
                    usize::from(obj::has_string_rep(member_ptr))
                )
                .unwrap();
            }
            drop(original_root);
            drop(original_member);
        }
        let expected = include_str!(
            "../../../../rust/tcl-syntax/testdata/native_jim_sugar_increment/observations.tsv"
        );
        assert_eq!(expected.lines().count(), 18);
        assert_eq!(observed, expected);
    }
}
