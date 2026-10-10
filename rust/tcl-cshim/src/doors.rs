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

//! What the C API does to the frame a command was called from: `Tcl_GetVar2Ex`,
//! `Tcl_ObjSetVar2`, `Tcl_UnsetVar2` and `Tcl_EvalObjEx`.
//!
//! Each acts on the variables and the evaluator of the engine the command is
//! running in, which the shim reaches only through the door the engine opened
//! for the call ([`InterpState::open_door`]). With no door open, which is a
//! command the engine invoked without one, each fails as a Tcl error and is
//! never an empty answer.
//!
//! **Names.** The door takes one name, spelt as `set` spells it, so an array
//! element given as two parts is composed (`a` and `k` are `a(k)`), a name
//! `TCL_GLOBAL_ONLY` asks for is rooted (`x` is `::x`), and a name Tcl reads as
//! an element and an index beside it is Tcl's own error (`variable isn't
//! array`). An array whose own name contains a parenthesis cannot be named that
//! way and is refused.
//!
//! **Flags.** Only what the header declares is honoured: `TCL_GLOBAL_ONLY` and
//! `TCL_LEAVE_ERR_MSG` for a variable, `TCL_EVAL_DIRECT` (a hint, which changes
//! nothing here) for an evaluation. Any other bit is refused, so a source that
//! passes one by number fails where it asked and does not run as if it had been
//! heard.
//!
//! **Errors.** A miss is the engine's own Tcl error, left in the interpreter's
//! result with its `-errorcode` when the call asked for it
//! (`TCL_LEAVE_ERR_MSG`; an evaluation always leaves it). A budget the engine
//! enforces and a crash are the errors C cannot swallow: the call fails as any
//! other, the error is kept ([`InterpState::set_fatal`]) and the command that is
//! running fails with it whatever it returns.
//!
//! **Objects.** A value read is handed to C as a new object kept until the
//! command that read it returns, so the pointer stays good through later calls
//! that change or unset the variable; a value passed in is held for the call
//! like a reference, so one with a count of zero is consumed as it is in Tcl: it
//! is freed when the call cannot store it, and when the command returns if it
//! can.

use std::ffi::c_int;

use tcl_engine_api::{CommandRegistrar, CompletionCode, EngineError};

use crate::ffi::{TCL_ERROR, TCL_EVAL_DIRECT, TCL_GLOBAL_ONLY, TCL_LEAVE_ERR_MSG, TCL_OK};
use crate::obj::{Obj, ObjRef, TclError};
use crate::state::InterpState;

/// The flags a variable call honours.
const VARIABLE_FLAGS: c_int = TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG;

/// The flags an evaluation honours.
const EVAL_FLAGS: c_int = TCL_EVAL_DIRECT;

/// What a variable call does, in the word C Tcl's message uses for it.
#[derive(Clone, Copy)]
enum Verb {
    Read,
    Set,
    Unset,
}

impl Verb {
    fn word(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Set => "set",
            Self::Unset => "unset",
        }
    }
}

fn failure(message: impl Into<String>) -> TclError {
    TclError {
        message: message.into().into_bytes(),
        code: None,
        getter: None,
        host: None,
    }
}

/// Refuse a flag bit the call does not honour.
fn check_flags(function: &str, flags: c_int, honoured: c_int) -> Result<(), TclError> {
    let refused = flags & !honoured;
    if refused == 0 {
        return Ok(());
    }
    Err(failure(format!(
        "{function}: the flags {refused:#x} are not implemented by the shim"
    )))
}

/// Preserve the Tcl9 shim's selected operands through the engine's combined
/// Unicode-name door. Unrepresentable counted inputs are host refusals.
fn bridge_name(
    part1: &str,
    part2: Option<&str>,
    verb: Verb,
    global_only: bool,
) -> Result<String, TclError> {
    use tcl_syntax::naming::{
        NativeNameProtocol, NativeVariableBridgeUnavailable, NativeVariableInputForm,
    };
    let input = match part2 {
        None => NativeVariableInputForm::Combined(part1.as_bytes()),
        Some(index) => NativeVariableInputForm::Separate {
            root: part1.as_bytes(),
            element: Some(index.as_bytes()),
        },
    };
    let bytes = NativeNameProtocol::C(tcl_dialect::TclVersion::V9_0)
        .variable_combined_bridge_input(input, global_only)
        .map_err(|error| match error {
            NativeVariableBridgeUnavailable::Part1IsElement => TclError::with_code(
                format!(
                    "can't {} \"{part1}({})\": variable isn't array",
                    verb.word(),
                    part2.expect("selected second operand"),
                ),
                "TCL VALUE VARNAME",
            ),
            NativeVariableBridgeUnavailable::Unrepresentable => TclError::host(
                EngineError::ExecutionRefusal(
                    "the engine's combined-name door cannot preserve the selected variable operands".into(),
                ),
            ),
        })?;
    String::from_utf8(bytes).map_err(|_| {
        TclError::host(EngineError::ExecutionRefusal(
            "the engine's variable door requires Unicode".into(),
        ))
    })
}

/// An error's message with the name the shim rooted given back as the C code
/// spelt it, since C Tcl reports a `TCL_GLOBAL_ONLY` name as it was given.
fn unrooted(mut error: TclError, rooted: &str, asked: &str) -> TclError {
    if rooted != asked {
        let (rooted, asked) = (format!("\"{rooted}\""), format!("\"{asked}\""));
        if let Some(at) = error
            .message
            .windows(rooted.len())
            .position(|window| window == rooted.as_bytes())
        {
            error.message.splice(at..at + rooted.len(), asked.bytes());
        }
    }
    error
}

fn no_door() -> TclError {
    failure(
        "no engine door is open: a C command reaches variables and the evaluator only while \
         the engine runs it with one",
    )
}

/// A call after a fatal error is refused as that error.
fn refuse_after_fatal(state: &InterpState) -> Result<(), TclError> {
    match state.fatal() {
        Some(error) => Err(failure(error.to_string())),
        None => Ok(()),
    }
}

/// What an engine's error is to C, keeping the ones C cannot swallow.
fn engine_failure(state: &InterpState, error: EngineError) -> TclError {
    match error {
        EngineError::Script { message, code } => TclError {
            message: message.into_bytes(),
            code: code.map(String::into_bytes),
            getter: None,
            host: None,
        },
        EngineError::ScriptBytes { message, code, .. } => TclError {
            message,
            code,
            getter: None,
            host: None,
        },
        EngineError::ExecutionRefusal(_) => TclError::host(error),
        EngineError::Compile(message) => failure(message),
        EngineError::Unsupported(_) => failure(error.to_string()),
        EngineError::BudgetExceeded(_) | EngineError::Crashed(_) => {
            let reported = failure(error.to_string());
            state.set_fatal(error);
            reported
        }
    }
}

/// One variable call through the door, every failure reported on the
/// interpreter as C Tcl reports it and answered as `None`.
fn variable_call<T>(
    state: &InterpState,
    function: &str,
    verb: Verb,
    (part1, part2): (&str, Option<&str>),
    flags: c_int,
    call: impl FnOnce(&mut dyn CommandRegistrar, &str) -> Result<T, EngineError>,
) -> Option<T> {
    let outcome = check_flags(function, flags, VARIABLE_FLAGS)
        .and_then(|()| refuse_after_fatal(state))
        .and_then(|()| bridge_name(part1, part2, verb, false))
        .and_then(|asked| {
            let name = bridge_name(part1, part2, verb, flags & TCL_GLOBAL_ONLY != 0)?;
            match state.with_door(|door| call(door, &name)) {
                Some(Ok(answer)) => Ok(answer),
                Some(Err(error)) => Err(unrooted(engine_failure(state, error), &name, &asked)),
                None => Err(no_door()),
            }
        });
    match outcome {
        Ok(answer) => Some(answer),
        Err(error) => {
            if let Some(refusal) = &error.host {
                state.refuse_host((**refusal).clone());
            }
            if flags & TCL_LEAVE_ERR_MSG != 0 {
                state.set_error(&error);
            }
            None
        }
    }
}

/// `Tcl_GetVar2Ex`: the variable's value as a new object kept until the command
/// returns, or NULL.
pub(crate) fn get_variable(
    state: &InterpState,
    part1: &str,
    part2: Option<&str>,
    flags: c_int,
) -> *mut Obj {
    variable_call(
        state,
        "Tcl_GetVar2Ex",
        Verb::Read,
        (part1, part2),
        flags,
        |door, name| door.variable(name),
    )
    .and_then(|value| match Obj::from_value(&value) {
        Ok(object) => Some(state.retain(ObjRef::new(object))),
        Err(error) => {
            state.set_error(&TclError::host(error));
            None
        }
    })
    .unwrap_or(std::ptr::null_mut())
}

/// `Tcl_ObjSetVar2`: the object given, now the variable's value, or NULL.
pub(crate) fn set_variable(
    state: &InterpState,
    part1: &str,
    part2: Option<&str>,
    new_value: ObjRef,
    flags: c_int,
) -> *mut Obj {
    let stored = new_value.get().to_value();
    variable_call(
        state,
        "Tcl_ObjSetVar2",
        Verb::Set,
        (part1, part2),
        flags,
        |door, name| door.set_variable(name, stored),
    )
    .map_or(std::ptr::null_mut(), |()| state.retain(new_value))
}

/// `Tcl_UnsetVar2`.
pub(crate) fn unset_variable(
    state: &InterpState,
    part1: &str,
    part2: Option<&str>,
    flags: c_int,
) -> c_int {
    variable_call(
        state,
        "Tcl_UnsetVar2",
        Verb::Unset,
        (part1, part2),
        flags,
        |door, name| door.unset_variable(name),
    )
    .map_or(TCL_ERROR, |()| TCL_OK)
}

/// `Tcl_PkgProvideEx`: the package reaches the engine's package database
/// through the open door, as `package provide` puts it there, so a version in
/// conflict with the one already provided is the engine's own Tcl error and
/// fails the call before the shim records it. An engine with no such door, or no
/// door open, leaves the package to the shim's own record, which is what
/// [`crate::Interp::provided_packages`] lists. The error is always left in the
/// interpreter's result, as `Tcl_PkgProvideEx` leaves it.
pub(crate) fn provide_package(state: &InterpState, name: &str, version: &str) -> c_int {
    let attempt = refuse_after_fatal(state)
        .and_then(|()| tell_the_engine(state, name, version))
        .and_then(|()| state.provide(name.as_bytes(), version.as_bytes()));
    match attempt {
        Ok(()) => TCL_OK,
        Err(error) => {
            state.set_error(&error);
            TCL_ERROR
        }
    }
}

pub(crate) fn provide_package_bytes(state: &InterpState, name: &[u8], version: &[u8]) -> c_int {
    let attempt = refuse_after_fatal(state).and_then(|()| {
        match state.with_door(|door| {
            let name = std::str::from_utf8(name)
                .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?;
            let version = std::str::from_utf8(version)
                .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))?;
            door.provide_package(name, version)
        }) {
            None | Some(Ok(()) | Err(EngineError::Unsupported(_))) => {}
            Some(Err(error)) => return Err(engine_failure(state, error)),
        }
        state.provide(name, version)
    });
    match attempt {
        Ok(()) => TCL_OK,
        Err(error) => {
            state.set_error(&error);
            TCL_ERROR
        }
    }
}

/// Say a package to the engine's package database, when a door is open and the
/// engine has one.
fn tell_the_engine(state: &InterpState, name: &str, version: &str) -> Result<(), TclError> {
    match state.with_door(|door| door.provide_package(name, version)) {
        None | Some(Ok(()) | Err(EngineError::Unsupported(_))) => Ok(()),
        Some(Err(error)) => Err(engine_failure(state, error)),
    }
}

/// `Tcl_EvalObjEx`: the script's code, and its result or its error in the
/// interpreter's result.
pub(crate) fn evaluate(state: &InterpState, script: &Obj, flags: c_int) -> c_int {
    let attempt = check_flags("Tcl_EvalObjEx", flags, EVAL_FLAGS)
        .and_then(|()| refuse_after_fatal(state))
        .and_then(|()| {
            let text = script.text().map_err(|error| {
                TclError::host(EngineError::ExecutionRefusal(error.to_string()))
            })?;
            state
                .with_door(|door| {
                    if let Some(shared) = state.shared() {
                        crate::ShimCommand::publish(&shared, door)?;
                    }
                    door.eval_in_invocation(&text)
                })
                .ok_or_else(no_door)?
                .map_err(|error| engine_failure(state, error))
        });
    match attempt {
        Ok(outcome) => {
            let object = match Obj::from_value(&outcome.value) {
                Ok(object) => object,
                Err(error) => {
                    state.set_error(&TclError::host(error));
                    return TCL_ERROR;
                }
            };
            state.set_result(ObjRef::new(object));
            state.set_return_options(
                (outcome.code == CompletionCode::Return).then_some(outcome.options),
            );
            outcome.code.as_int()
        }
        Err(error) => {
            state.set_return_options(None);
            state.set_error(&error);
            TCL_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{c_int, c_void};
    use std::rc::Rc;

    use tcl_engine_api::{
        BudgetKind, CommandRegistrar, CompletionCode, EngineError, HostCommand, HostOutcome, Value,
    };

    use super::{
        TCL_ERROR, TCL_EVAL_DIRECT, TCL_GLOBAL_ONLY, TCL_LEAVE_ERR_MSG, TCL_OK, Verb, compose,
        evaluate, get_variable, provide_package, root, set_variable, unset_variable,
    };
    use crate::obj::{Obj, ObjRef, TclError};
    use crate::state::{DoorRef, InterpState};

    /// A door that answers what it is told and records what it is asked.
    struct Fake {
        log: Vec<String>,
        read: Result<Value, EngineError>,
        write: Result<(), EngineError>,
        eval: Result<HostOutcome, EngineError>,
        provide: Result<(), EngineError>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                log: Vec::new(),
                read: Ok(Value::Empty),
                write: Ok(()),
                eval: Ok(HostOutcome::ok(Value::Empty)),
                provide: Ok(()),
            }
        }
    }

    impl CommandRegistrar for Fake {
        fn define_command(
            &mut self,
            name: &str,
            _command: Rc<dyn HostCommand>,
        ) -> Result<(), EngineError> {
            self.log.push(format!("define {name}"));
            Ok(())
        }

        fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
            self.log.push(format!("remove {name}"));
            Ok(true)
        }

        fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
            self.log.push(format!("provide {name} {version}"));
            self.provide.clone()
        }

        fn variable(&mut self, name: &str) -> Result<Value, EngineError> {
            self.log.push(format!("read {name}"));
            self.read.clone()
        }

        fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
            self.log.push(format!("write {name} {value:?}"));
            self.write.clone()
        }

        fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
            self.log.push(format!("unset {name}"));
            self.write.clone()
        }

        fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
            self.log.push(format!("eval {script}"));
            self.eval.clone()
        }
    }

    fn miss() -> EngineError {
        EngineError::Script {
            message: "can't read \"x\": no such variable".to_owned(),
            code: Some("TCL LOOKUP VARNAME x".to_owned()),
        }
    }

    fn text(object: *mut Obj) -> String {
        // SAFETY: the tests pass the pointers the calls under test answered, while
        // the scope that keeps them is open.
        unsafe { &*object }.text().expect("Unicode test object")
    }

    #[test]
    fn names_are_composed_as_set_spells_them() {
        let global = TCL_GLOBAL_ONLY;
        let cases = [
            ("a", None, 0, "a"),
            ("a", Some("k"), 0, "a(k)"),
            ("a", Some(""), 0, "a()"),
            ("a(k)", None, 0, "a(k)"),
            ("x", None, global, "::x"),
            ("::x", None, global, "::x"),
            ("ns::v", None, global, "::ns::v"),
            ("a", Some("k"), global, "::a(k)"),
            ("a(k)", None, global, "::a(k)"),
            ("::a", Some("k"), 0, "::a(k)"),
        ];
        for (part1, part2, flags, expected) in cases {
            assert_eq!(
                bridge_name(part1, part2, Verb::Read, flags & TCL_GLOBAL_ONLY != 0),
                Ok(expected.to_owned()),
                "{part1} {part2:?} {flags}"
            );
        }
    }

    #[test]
    fn an_unrepresentable_counted_global_name_retains_host_refusal_without_guest_reporting() {
        let state = InterpState::new_shared();
        state.set_result_text("BEFORE");
        let mut fake = Fake::new();
        let mut door = DoorRef::new(&mut fake);
        let _scope = state.open_door(&mut door);
        assert!(get_variable(&state, "x\0tail", None, TCL_GLOBAL_ONLY).is_null());
        assert!(matches!(
            state.take_host_refusal(),
            Some(EngineError::ExecutionRefusal(_))
        ));
        assert_eq!(state.result().get().bytes(), b"BEFORE");
        drop(_scope);
        assert!(fake.log.is_empty());
    }

    #[test]
    fn an_element_given_an_index_is_tcl_s_error_in_the_verb_of_the_call() {
        for (verb, word) in [
            (Verb::Read, "read"),
            (Verb::Set, "set"),
            (Verb::Unset, "unset"),
        ] {
            assert_eq!(
                bridge_name("a(k)", Some("j"), verb, false),
                Err(TclError::with_code(
                    format!("can't {word} \"a(k)(j)\": variable isn't array"),
                    "TCL VALUE VARNAME"
                ))
            );
        }
        assert!(
            bridge_name("a(", Some("j"), Verb::Read, false).is_err(),
            "a parenthesis that opens no element is no array name"
        );
        assert_eq!(
            bridge_name("a)", Some("j"), Verb::Read, false),
            Ok("a)(j)".to_owned()),
            "and a closing one with no opening is part of the name"
        );
    }

    #[test]
    fn an_array_name_with_a_parenthesis_is_refused() {
        let error = compose("a(b", Some("k"), Verb::Read).expect_err("refused");
        assert!(
            error
                .message
                .windows(b"cannot name the array".len())
                .any(|word| word == b"cannot name the array"),
            "{error:?}"
        );
        assert_eq!(error.code, None);
    }

    #[test]
    fn a_flag_the_call_does_not_honour_is_refused_before_the_door_is_asked() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        let mut door = DoorRef::new(&mut fake);
        let open = state.open_door(&mut door);
        let flags = TCL_LEAVE_ERR_MSG | 0x4;
        assert!(get_variable(&state, "x", None, flags).is_null());
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "Tcl_GetVar2Ex: the flags 0x4 are not implemented by the shim"
        );
        assert_eq!(unset_variable(&state, "x", None, flags), TCL_ERROR);
        let value = ObjRef::new(Obj::from_text("v"));
        assert!(set_variable(&state, "x", None, value, flags).is_null());
        let script = Obj::from_text("set x 1");
        assert_eq!(evaluate(&state, &script, 0x2_0000), TCL_ERROR);
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "Tcl_EvalObjEx: the flags 0x20000 are not implemented by the shim",
            "TCL_EVAL_GLOBAL is not one the shim does"
        );
        drop(open);
        assert!(fake.log.is_empty(), "{:?}", fake.log);
    }

    #[test]
    fn a_miss_is_left_in_the_result_only_when_the_call_asks() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        fake.read = Err(miss());
        let mut door = DoorRef::new(&mut fake);
        let _open = state.open_door(&mut door);

        state.set_result_text("untouched");
        assert!(get_variable(&state, "x", None, 0).is_null());
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "untouched"
        );
        assert_eq!(state.error_code_text(), None);

        assert!(get_variable(&state, "x", None, TCL_LEAVE_ERR_MSG).is_null());
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "can't read \"x\": no such variable"
        );
        assert_eq!(
            state.error_code_text().as_deref(),
            Some("TCL LOOKUP VARNAME x")
        );
    }

    #[test]
    fn a_name_the_call_rooted_is_reported_as_the_c_code_gave_it() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        fake.read = Err(EngineError::Script {
            message: "can't read \"::ns::a(k)\": no such variable".to_owned(),
            code: None,
        });
        let mut door = DoorRef::new(&mut fake);
        let _open = state.open_door(&mut door);

        let flags = TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG;
        assert!(get_variable(&state, "ns::a", Some("k"), flags).is_null());
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "can't read \"ns::a(k)\": no such variable"
        );

        assert!(get_variable(&state, "::ns::a(k)", None, flags).is_null());
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "can't read \"::ns::a(k)\": no such variable",
            "a name given rooted was not rooted by the call, so the engine's message stands"
        );
    }

    #[test]
    fn a_package_reaches_the_engine_before_the_shim_records_it() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        {
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            assert_eq!(provide_package(&state, "pkga", "1.0"), TCL_OK);
        }
        assert_eq!(fake.log, ["provide pkga 1.0"]);
        assert_eq!(
            state.provided_packages(),
            [(tcl_core_types::NameBytes::from("pkga"), b"1.0".to_vec())]
        );
    }

    #[test]
    fn a_package_the_engine_refuses_is_its_error_and_is_not_recorded() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        fake.provide = Err(EngineError::Script {
            message: "conflicting versions provided for package \"pkga\": 2.0, then 1.0".to_owned(),
            code: Some("TCL PACKAGE VERSIONCONFLICT".to_owned()),
        });
        {
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            assert_eq!(provide_package(&state, "pkga", "1.0"), TCL_ERROR);
        }
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "conflicting versions provided for package \"pkga\": 2.0, then 1.0",
            "left in the result without being asked for, as Tcl_PkgProvideEx leaves it"
        );
        assert_eq!(
            state.error_code_text().as_deref(),
            Some("TCL PACKAGE VERSIONCONFLICT")
        );
        assert!(state.provided_packages().is_empty(), "nothing was provided");
    }

    #[test]
    fn a_package_with_no_door_to_say_it_to_is_the_shims_own_record() {
        for unsupported in [true, false] {
            let state = InterpState::new();
            let mut fake = Fake::new();
            fake.provide = Err(EngineError::Unsupported("providing a package"));
            if unsupported {
                let mut door = DoorRef::new(&mut fake);
                let _open = state.open_door(&mut door);
                assert_eq!(provide_package(&state, "pkga", "1.0"), TCL_OK);
                assert_eq!(state.fatal(), None);
            } else {
                assert_eq!(
                    provide_package(&state, "pkga", "1.0"),
                    TCL_OK,
                    "and so is a call outside any invocation"
                );
            }
            assert_eq!(
                state.provided_packages(),
                [(tcl_core_types::NameBytes::from("pkga"), b"1.0".to_vec())]
            );
        }
    }

    #[test]
    fn a_package_at_another_version_is_the_shims_conflict_when_the_engine_has_none() {
        let state = InterpState::new();
        assert_eq!(provide_package(&state, "pkga", "1.0"), TCL_OK);
        assert_eq!(
            provide_package(&state, "pkga", "1.0"),
            TCL_OK,
            "again is a no-op"
        );
        assert_eq!(provide_package(&state, "pkga", "2.0"), TCL_ERROR);
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "conflicting versions provided for package \"pkga\": 1.0, then 2.0"
        );
    }

    #[test]
    fn a_budget_a_package_call_meets_is_fatal() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        fake.provide = Err(EngineError::BudgetExceeded(BudgetKind::Commands));
        let mut door = DoorRef::new(&mut fake);
        let _open = state.open_door(&mut door);
        assert_eq!(provide_package(&state, "pkga", "1.0"), TCL_ERROR);
        assert_eq!(
            state.fatal(),
            Some(EngineError::BudgetExceeded(BudgetKind::Commands))
        );
        assert_eq!(provide_package(&state, "pkgb", "1.0"), TCL_ERROR);
        assert!(state.provided_packages().is_empty());
    }

    #[test]
    fn a_package_call_after_a_fatal_error_is_refused_without_reaching_the_engine() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        fake.eval = Err(EngineError::BudgetExceeded(BudgetKind::Commands));
        {
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            let script = Obj::from_text("loop");
            assert_eq!(evaluate(&state, &script, 0), TCL_ERROR);
            assert_eq!(provide_package(&state, "pkga", "1.0"), TCL_ERROR);
        }
        assert_eq!(
            fake.log,
            ["eval loop"],
            "the package was not said to the engine"
        );
        assert!(state.provided_packages().is_empty());
    }

    #[test]
    fn a_value_read_is_a_new_object_that_outlives_later_calls() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        fake.read = Ok(Value::Int(7));
        let kept;
        {
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            let object = get_variable(&state, "x", None, 0);
            // SAFETY: `object` is the live object the call answered.
            kept = unsafe { ObjRef::adopt(object) };
            assert_eq!(
                kept.get().refcount(),
                2,
                "the scope holds one, this the other"
            );
            assert_eq!(unset_variable(&state, "x", None, 0), TCL_OK);
            assert_eq!(
                text(object),
                "7",
                "the variable's going does not take the value"
            );
            assert!(
                matches!(kept.get().to_value(), Value::Int(7)),
                "typed, not text"
            );
        }
        assert_eq!(
            kept.get().refcount(),
            1,
            "the scope let go of its reference"
        );
        assert_eq!(fake.log, ["read x", "unset x"]);
    }

    #[test]
    fn a_value_passed_in_is_stored_typed_and_held_for_the_command() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        let value = ObjRef::new(Obj::int(5));
        let mine = value.clone();
        {
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            let stored = set_variable(&state, "x", None, value, 0);
            assert_eq!(
                stored,
                mine.as_ptr(),
                "the object the variable now holds is the one given"
            );
            assert_eq!(mine.get().refcount(), 2, "held for the command");
        }
        assert_eq!(mine.get().refcount(), 1);
        assert_eq!(fake.log, ["write x Int(5)"]);
    }

    #[test]
    fn a_value_a_failed_store_cannot_keep_is_let_go_at_once() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        fake.write = Err(miss());
        let value = ObjRef::new(Obj::int(5));
        let mine = value.clone();
        let mut door = DoorRef::new(&mut fake);
        let _open = state.open_door(&mut door);
        assert!(set_variable(&state, "x", None, value, TCL_LEAVE_ERR_MSG).is_null());
        assert_eq!(
            mine.get().refcount(),
            1,
            "only the caller's reference is left"
        );
    }

    #[test]
    fn with_no_door_open_each_call_is_an_error_and_never_an_empty_answer() {
        let state = InterpState::new();
        assert!(get_variable(&state, "x", None, TCL_LEAVE_ERR_MSG).is_null());
        assert!(
            state
                .result()
                .get()
                .text()
                .starts_with("no engine door is open")
        );
        state.reset_result();
        assert_eq!(
            unset_variable(&state, "x", None, TCL_LEAVE_ERR_MSG),
            TCL_ERROR
        );
        assert!(
            state
                .result()
                .get()
                .text()
                .starts_with("no engine door is open")
        );
        state.reset_result();
        let script = Obj::from_text("set x 1");
        assert_eq!(evaluate(&state, &script, 0), TCL_ERROR);
        assert!(
            state
                .result()
                .get()
                .text()
                .starts_with("no engine door is open")
        );
    }

    #[test]
    fn a_budget_or_a_crash_is_fatal_and_later_calls_fail_without_reaching_the_door() {
        for fatal in [
            EngineError::BudgetExceeded(BudgetKind::Commands),
            EngineError::Crashed("boom".to_owned()),
        ] {
            let state = InterpState::new();
            let mut fake = Fake::new();
            fake.eval = Err(fatal.clone());
            {
                let mut door = DoorRef::new(&mut fake);
                let _open = state.open_door(&mut door);
                let script = Obj::from_text("loop");
                assert_eq!(evaluate(&state, &script, 0), TCL_ERROR);
                assert_eq!(state.fatal(), Some(fatal.clone()));
                assert_eq!(evaluate(&state, &script, 0), TCL_ERROR);
                assert_eq!(
                    state.result().get().text().expect("Unicode test object"),
                    fatal.to_string()
                );
                assert!(get_variable(&state, "x", None, 0).is_null());
                assert_eq!(unset_variable(&state, "x", None, 0), TCL_ERROR);
            }
            assert_eq!(
                fake.log,
                ["eval loop"],
                "{fatal}: nothing after the first reached it"
            );
            assert_eq!(state.take_fatal(), Some(fatal));
            assert_eq!(state.take_fatal(), None);
        }
    }

    #[test]
    fn the_first_fatal_error_is_the_one_kept() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        fake.eval = Err(EngineError::BudgetExceeded(BudgetKind::WallClock));
        let mut door = DoorRef::new(&mut fake);
        let _open = state.open_door(&mut door);
        let script = Obj::from_text("loop");
        assert_eq!(evaluate(&state, &script, 0), TCL_ERROR);
        state.set_fatal(EngineError::Crashed("later".to_owned()));
        assert_eq!(
            state.take_fatal(),
            Some(EngineError::BudgetExceeded(BudgetKind::WallClock))
        );
    }

    #[test]
    fn a_script_error_or_an_unsupported_door_is_not_fatal() {
        for (error, message) in [
            (miss(), "can't read \"x\": no such variable"),
            (
                EngineError::Unsupported("reading a variable"),
                "unsupported by this engine: reading a variable",
            ),
        ] {
            let state = InterpState::new();
            let mut fake = Fake::new();
            fake.read = Err(error);
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            assert!(get_variable(&state, "x", None, TCL_LEAVE_ERR_MSG).is_null());
            assert_eq!(
                state.result().get().text().expect("Unicode test object"),
                message
            );
            assert_eq!(state.fatal(), None);
        }
    }

    #[test]
    fn an_evaluation_answers_its_code_and_leaves_its_value_or_its_error_as_the_result() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        let script = Obj::from_text("body");
        for (answer, code, result) in [
            (Ok(HostOutcome::ok(Value::string("v"))), TCL_OK, "v"),
            (
                Ok(HostOutcome::completing(CompletionCode::Break, Value::Empty)),
                3,
                "",
            ),
            (
                Ok(HostOutcome::completing(
                    CompletionCode::Return,
                    Value::string("r"),
                )),
                2,
                "r",
            ),
            (
                Ok(HostOutcome::completing(
                    CompletionCode::Other(9),
                    Value::Int(1),
                )),
                9,
                "1",
            ),
        ] {
            fake.eval = answer;
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            assert_eq!(evaluate(&state, &script, TCL_EVAL_DIRECT), code);
            assert_eq!(
                state.result().get().text().expect("Unicode test object"),
                result
            );
        }
        fake.eval = Err(EngineError::Script {
            message: "boom".to_owned(),
            code: Some("A B".to_owned()),
        });
        let mut door = DoorRef::new(&mut fake);
        let _open = state.open_door(&mut door);
        assert_eq!(evaluate(&state, &script, 0), TCL_ERROR);
        assert_eq!(
            state.result().get().text().expect("Unicode test object"),
            "boom"
        );
        assert_eq!(state.error_code_text().as_deref(), Some("A B"));
    }

    #[test]
    fn an_evaluation_keeps_the_options_of_the_return_it_ended_in_until_the_next_one() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        let script = Obj::from_text("body");
        let options = Value::string("-code 1 -level 1 -errorcode {X Y}");
        fake.eval = Ok(HostOutcome::returning(
            Value::string("msg"),
            options.clone(),
        ));
        let mut door = DoorRef::new(&mut fake);
        let _open = state.open_door(&mut door);

        assert_eq!(evaluate(&state, &script, 0), 2);
        assert_eq!(
            state.take_return_options().as_ref().and_then(Value::as_str),
            options.as_str(),
            "a return's options are the interpreter's until taken"
        );
        assert!(state.take_return_options().is_none(), "and taken once");

        assert_eq!(evaluate(&state, &script, 0), 2);
        state.reset_result();
        assert!(
            state.take_return_options().is_none(),
            "`Tcl_ResetResult` resets them, as it resets the return level and code"
        );

        assert_eq!(evaluate(&state, &script, 0), 2);
        state.set_result_text("a result set by hand");
        assert!(
            state.take_return_options().is_some(),
            "a result set does not"
        );
    }

    #[test]
    fn an_evaluation_that_is_not_a_return_or_that_fails_clears_the_options() {
        let script = Obj::from_text("body");
        for (answer, code) in [
            (Ok(HostOutcome::ok(Value::string("v"))), TCL_OK),
            (
                Ok(HostOutcome::completing(CompletionCode::Break, Value::Empty)),
                3,
            ),
            (
                Err(EngineError::Script {
                    message: "boom".to_owned(),
                    code: None,
                }),
                TCL_ERROR,
            ),
        ] {
            let state = InterpState::new();
            state.set_return_options(Some(Value::string("-code 1 -level 1")));
            let mut fake = Fake::new();
            fake.eval = answer;
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            assert_eq!(evaluate(&state, &script, 0), code);
            assert!(
                state.take_return_options().is_none(),
                "{code}: the later evaluation replaced what the first left"
            );
        }
    }

    unsafe extern "C" fn nothing(
        _client_data: *mut c_void,
        _interp: *mut InterpState,
        _word_count: c_int,
        _words: *const *mut Obj,
    ) -> c_int {
        TCL_OK
    }

    #[test]
    fn a_command_the_c_code_created_is_published_before_the_script_that_calls_it() {
        let state = InterpState::new_shared();
        state.create_command("late", nothing, std::ptr::null_mut(), None);
        let mut fake = Fake::new();
        {
            let mut door = DoorRef::new(&mut fake);
            let _open = state.open_door(&mut door);
            let script = Obj::from_text("late");
            assert_eq!(evaluate(&state, &script, 0), TCL_OK);
        }
        assert_eq!(fake.log, ["define late", "eval late"]);
        assert!(state.take_pending().is_empty(), "and it is published once");
    }

    #[test]
    fn a_door_opened_inside_another_is_closed_to_the_outer_until_it_ends() {
        let state = InterpState::new();
        let (mut outer, mut inner) = (Fake::new(), Fake::new());
        {
            let mut outer_door = DoorRef::new(&mut outer);
            let _outer_open = state.open_door(&mut outer_door);
            let first = get_variable(&state, "a", None, 0);
            assert!(!first.is_null());
            // SAFETY: `first` is live while the outer scope is open.
            let held = unsafe { ObjRef::adopt(first) };
            {
                let mut inner_door = DoorRef::new(&mut inner);
                let _inner_open = state.open_door(&mut inner_door);
                assert!(!get_variable(&state, "b", None, 0).is_null());
            }
            assert_eq!(
                held.get().refcount(),
                2,
                "what the outer command was handed outlives the inner command"
            );
            assert!(!get_variable(&state, "c", None, 0).is_null());
        }
        assert!(get_variable(&state, "d", None, 0).is_null());
        assert_eq!(outer.log, ["read a", "read c"]);
        assert_eq!(inner.log, ["read b"]);
    }

    #[test]
    fn the_door_is_closed_while_a_call_through_it_runs() {
        let state = InterpState::new();
        let mut fake = Fake::new();
        let mut door = DoorRef::new(&mut fake);
        let _open = state.open_door(&mut door);
        let nested = state
            .with_door(|_| state.with_door(|_| ()))
            .expect("the door was open");
        assert_eq!(
            nested, None,
            "nothing reaches the outer door while it is in use"
        );
        assert!(
            state.with_door(|_| ()).is_some(),
            "and it is open again after"
        );
    }
}
