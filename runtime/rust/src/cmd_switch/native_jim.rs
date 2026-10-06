// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim switch refetches the same case-list object after actual match callbacks.
use crate::{
    interp::{Code, Interp},
    obj::{self, TclObj},
};
use tcl_cmd_core::{
    native_jim_switch::{self, Failure, Immediate, NativeJimSwitchObjects, Selection},
    CmdError,
};
use tcl_syntax::{native_string::NativeStringProtocol, raw_string::RawString, value::ValueError};
static IMMEDIATES: &[&str] = &["default", "-"];
impl NativeJimSwitchObjects for Interp {
    type Value = *mut TclObj;
    type Callback = Code;
    fn switch_bytes(&mut self, value: &*mut TclObj) -> Result<Vec<u8>, CmdError> {
        crate::dict::native_object_bytes(*value, NativeStringProtocol::Jim084).map_err(Into::into)
    }
    fn switch_borrow(&self, value: &*mut TclObj) -> *mut TclObj {
        *value
    }
    fn switch_list_length(&mut self, value: &*mut TclObj) -> Result<usize, CmdError> {
        Ok(crate::list::list_elements_native_checked(*value, NativeStringProtocol::Jim084)?.len())
    }
    fn switch_list_member(
        &mut self,
        value: &*mut TclObj,
        index: usize,
    ) -> Result<*mut TclObj, CmdError> {
        crate::list::list_elements_native_checked(*value, NativeStringProtocol::Jim084)?
            .get(index)
            .copied()
            .ok_or_else(|| {
                ValueError::CommandProtocolUnavailable("Jim switch current List member").into()
            })
    }
    fn switch_immediate(
        &mut self,
        value: &*mut TclObj,
        immediate: Immediate,
    ) -> Result<bool, CmdError> {
        let table = tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
            IMMEDIATES,
        );
        self.native_jim_compare_immediate(
            *value,
            &table,
            match immediate {
                Immediate::Default => 0,
                Immediate::Dash => 1,
            },
        )
        .map_err(Into::into)
    }
    fn switch_equal(
        &mut self,
        subject: &*mut TclObj,
        pattern: &*mut TclObj,
    ) -> Result<bool, CmdError> {
        if *subject == *pattern {
            return Ok(true);
        }
        Ok(self.switch_bytes(subject)? == self.switch_bytes(pattern)?)
    }
    fn switch_glob(
        &mut self,
        pattern: &*mut TclObj,
        subject: &*mut TclObj,
    ) -> Result<bool, CmdError> {
        let pattern = RawString::from_bytes(self.switch_bytes(pattern)?);
        let subject = RawString::from_bytes(self.switch_bytes(subject)?);
        subject.jim084_matches(&pattern, false).map_err(Into::into)
    }
    fn switch_callback_code(&self, callback: &Code) -> i32 {
        callback.as_int() as i32
    }
    fn switch_negative_match(&mut self, code: i32) -> Code {
        Code::from_int(code)
    }
    fn switch_command(
        &mut self,
        command: Option<&*mut TclObj>,
        pattern: &*mut TclObj,
        subject: &*mut TclObj,
        option_end: bool,
    ) -> Result<i64, Code> {
        let head;
        let pointer;
        let original = match command {
            Some(command) => command,
            None => {
                head = obj::Owned::fresh(obj::new_string_bytes(b"regexp")).into_native_unowned();
                pointer = head;
                &pointer
            }
        };
        crate::cmd_regex::invoke_jim_match_command(
            self, original, pattern, subject, false, option_end,
        )
    }
}
pub(super) fn invoke(
    interp: &mut Interp,
    args: &[*mut TclObj],
    protocol: tcl_registry::native_jim_switch::NativeJimSwitchProtocol,
) -> Code {
    let selection = match native_jim_switch::select(interp, protocol, args) {
        Ok(selection) => selection,
        Err(Failure::Command(error)) => return interp.report_cmd_error(error),
        Err(Failure::Callback(code)) => return code,
    };
    let context = match interp.native_jim_object_context() {
        Ok(context) => context,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    interp.set_result(context.empty_object().as_ptr());
    match selection {
        Selection::Empty => Code::Ok,
        Selection::Body(body) => {
            let frame = interp.unlocated_frame();
            interp.eval_original_body_framed(
                tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
                body,
                frame,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    thread_local! {static STATE: RefCell<(usize,usize,*mut TclObj)> = const {RefCell::new((0,0,core::ptr::null_mut()))};}
    fn hex(text: &str) -> Vec<u8> {
        assert!(text.len().is_multiple_of(2));
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn alternate(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
        STATE.with(|state| state.borrow_mut().1 += 1);
        interp.set_result(obj::new_wide_int_obj(1));
        Code::Ok
    }
    fn matcher(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
        let (index, calls, root) = STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.1 += 1;
            *state
        });
        if index == 30 {
            if let Err(error) = obj::native_character_count(root,NativeStringProtocol::Jim084,tcl_registry::native_string_length::NativeStringLengthRepresentation::JimCachedString) {return interp.report_cmd_error(error.into());}
        }
        if index == 31 && calls == 1 {
            interp.register_builtin(b"matcher", alternate);
            interp.set_result(obj::new_wide_int_obj(0));
            return Code::Ok;
        }
        if index == 27 {
            interp.set_result(obj::new_wide_int_obj(0));
            return Code::Ok;
        }
        if index == 28 {
            interp.set_result_bytes(b"NONNUMERIC");
            return Code::Ok;
        }
        if index == 29 {
            interp.set_result_bytes(b"CALLBACK_ERROR");
            return Code::Error;
        }
        interp.set_result(obj::new_wide_int_obj(1));
        Code::Ok
    }
    #[test]
    fn switch_matches_all_192_original_native_option_and_callback_results() {
        let cases =
            include_str!("../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/cases.tsv");
        let mut compared = 0;
        for (profile, rows) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/8.4.20.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/8.5.19.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/9.1.0.tsv"
                ),
            ),
            (
                "jim",
                include_str!(
                    "../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/jim0.84.tsv"
                ),
            ),
        ] {
            for (index, row) in rows
                .lines()
                .filter(|line| line.starts_with("RESULT\t"))
                .enumerate()
            {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields[1].parse::<usize>().unwrap(), index);
                let case: Vec<_> = cases.lines().nth(index).unwrap().split('\t').collect();
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect(profile),
                    tcl_registry::special_vars::NativeBootstrapInputs {
                        package_path: Vec::new(),
                        default_library: None,
                    },
                )
                .unwrap();
                interp.register_builtin(b"matcher", matcher);
                let originals: Vec<_> = case[1..]
                    .iter()
                    .map(|word| obj::Owned::fresh(obj::new_string_bytes(&hex(word))))
                    .collect();
                STATE.with(|state| {
                    *state.borrow_mut() = (index, 0, originals.last().unwrap().as_ptr())
                });
                let argv: Vec<_> = originals.iter().map(obj::Owned::as_ptr).collect();
                let code = interp.eval_original_object_vector(&argv);
                assert_eq!(
                    code.as_int(),
                    fields[3].parse::<i64>().unwrap(),
                    "{profile} {}",
                    case[0]
                );
                assert_eq!(
                    STATE.with(|state| state.borrow().1),
                    fields[4].parse::<usize>().unwrap(),
                    "{profile} {}: callback count",
                    case[0]
                );
                assert_eq!(
                    interp.result_bytes(),
                    hex(fields[5]),
                    "{profile} {}",
                    case[0]
                );
                if profile == "jim" {
                    for object in rows
                        .lines()
                        .filter(|line| line.starts_with(&format!("OBJECT\t{index}\t")))
                    {
                        let object: Vec<_> = object.split('\t').collect();
                        let value = originals[object[2].parse::<usize>().unwrap()].as_ptr();
                        let ty = obj::obj_type_ptr(value);
                        // SAFETY: originals retain every header until after inspection.
                        let actual = if ty.is_null() {
                            "NULL"
                        } else {
                            unsafe { std::ffi::CStr::from_ptr((*ty).name) }
                                .to_str()
                                .unwrap()
                        };
                        assert_eq!(
                            actual, object[3],
                            "{profile} {} original {}",
                            case[0], object[2]
                        );
                        // SAFETY: inspection borrows this externally retained original.
                        assert_eq!(
                            unsafe { (*value).ref_count },
                            object[4].parse::<obj::TclSize>().unwrap(),
                            "{profile} {} original {} references",
                            case[0],
                            object[2]
                        );
                        assert_eq!(
                            obj::has_string_rep(value),
                            object[5] == "1",
                            "{profile} {} original {} resident",
                            case[0],
                            object[2]
                        );
                    }
                }
                compared += 1;
                STATE.with(|state| state.borrow_mut().2 = core::ptr::null_mut());
            }
        }
        assert_eq!(compared, 192);
    }
    thread_local! {static REFETCH: RefCell<(usize,*mut TclObj,Option<obj::Owned>,Vec<String>)> = const {RefCell::new((0,core::ptr::null_mut(),None,Vec::new()))};}
    fn kind(value: *mut TclObj) -> String {
        let ty = obj::obj_type_ptr(value);
        if ty.is_null() {
            "NULL".to_owned()
        } else {
            // SAFETY: the caller observes a live externally held or borrowed header.
            unsafe { std::ffi::CStr::from_ptr((*ty).name) }
                .to_str()
                .unwrap()
                .to_owned()
        }
    }
    fn refs(value: *mut TclObj) -> obj::TclSize {
        // SAFETY: the original header is live during every recorded window.
        unsafe { (*value).ref_count }
    }
    fn refetch_match(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        REFETCH.with(|state|{
            let mut state=state.borrow_mut();let (outcome,root)=(state.0,state.1);
            let members=crate::list::list_elements_native_checked(root,NativeStringProtocol::Jim084).unwrap();
            let body=obj::Owned::retain(members[1]);let pattern=argv[argv.len()-2];
            state.3.push(format!("BEFORE\t{outcome}\t{}\t{}\t{}\t{}\t{}\t{}",refs(root),kind(body.as_ptr()),refs(body.as_ptr()),kind(pattern),refs(pattern),usize::from(pattern==members[0])));
            state.2=Some(body);
            obj::native_character_count(root,NativeStringProtocol::Jim084,tcl_registry::native_string_length::NativeStringLengthRepresentation::JimCachedString).unwrap();
            let body=state.2.as_ref().unwrap().as_ptr();
            state.3.push(format!("AFTER_SHIMMER\t{outcome}\t{}\t{}\t{}\t{}\t{}",kind(root),kind(body),refs(body),kind(pattern),refs(pattern)));
            if outcome==1 {interp.set_result_bytes(b"FAILED");return Code::Error;}
            if outcome==2 {interp.set_result_bytes(b"NONNUMERIC");return Code::Ok;}
            interp.set_result(obj::new_wide_int_obj(1));Code::Ok
        })
    }
    #[test]
    fn same_case_list_is_refetched_in_all_12_native_ownership_windows() {
        let mut windows = Vec::new();
        for outcome in 0..3 {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jim"),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            interp.register_builtin(b"matcher", refetch_match);
            let originals: Vec<_> = [
                b"switch".as_slice(),
                b"-command",
                b"matcher",
                b"X",
                b"X {list MATCH}",
            ]
            .into_iter()
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
            .collect();
            let root = originals.last().unwrap().as_ptr();
            REFETCH.with(|state| *state.borrow_mut() = (outcome, root, None, Vec::new()));
            let argv: Vec<_> = originals.iter().map(obj::Owned::as_ptr).collect();
            let code = interp.eval_original_object_vector(&argv);
            REFETCH.with(|state| {
                let mut state = state.borrow_mut();
                windows.append(&mut state.3);
                let members =
                    crate::list::list_elements_native_checked(root, NativeStringProtocol::Jim084)
                        .unwrap();
                let child = members[1];
                let old = state.2.as_ref().unwrap().as_ptr();
                windows.push(format!(
                    "AFTER_SWITCH\t{outcome}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    code.as_int(),
                    kind(root),
                    refs(root),
                    kind(old),
                    refs(old),
                    kind(child),
                    refs(child),
                    usize::from(child == old)
                ));
                windows.push(format!(
                    "RESULT\t{outcome}\t{}",
                    std::str::from_utf8(&interp.result_bytes()).unwrap()
                ));
                *state = (0, core::ptr::null_mut(), None, Vec::new());
            });
        }
        assert_eq!(
            windows.join("\n"),
            include_str!("../../../../rust/tcl-cmd-core/tests/data/native_jim_switch/refetch.tsv")
                .strip_suffix('\n')
                .expect("native fixture ends with one record newline")
        );
        assert_eq!(windows.len(), 12);
    }
}
