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

//! What the tclvm engine gives a host command beyond a value: the completion
//! code it answers, and the door it holds on the calling frame's variables,
//! on evaluation, on packages and on the libraries `info loaded` lists.
//!
//! The commands here are the ones a C extension's procedures are, written in
//! Rust against the interface: each is a [`HostCommand`], registered through
//! [`Engine::define_command`] and run from a script.

use std::rc::Rc;

use tcl_engine_api::{
    Budget, BudgetKind, CommandRegistrar, CompileUnit, CompletionCode, Engine, EngineError,
    HostCommand, HostOutcome, Value,
};
use tcl_engine_tclvm::TclVmEngine;

/// A command that answers `value` and completes with `code`, whatever it is
/// called with.
struct Completes(CompletionCode, &'static str);

impl HostCommand for Completes {
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Ok(HostOutcome::completing(self.0, Value::string(self.1)))
    }
}

/// What a door command does with the door it is given.
#[derive(Clone, Copy)]
enum Door {
    Set,
    Get,
    Unset,
    Eval,
    EvalReport,
    Provide,
    Loaded,
}

impl HostCommand for Door {
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::Unsupported("a door command needs the door"))
    }

    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        arguments: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        let word = |index: usize| arguments.get(index).and_then(Value::as_str).unwrap_or("");
        match self {
            Self::Set => {
                registrar.set_variable(word(0), Value::string(word(1)))?;
                Ok(Value::Empty.into())
            }
            Self::Get => Ok(registrar.variable(word(0))?.into()),
            Self::Unset => {
                registrar.unset_variable(word(0))?;
                Ok(Value::Empty.into())
            }
            Self::Eval => registrar.eval_in_invocation(word(0)),
            Self::EvalReport => Ok(Value::string(match registrar.eval_in_invocation(word(0)) {
                Ok(_) => "ok".to_owned(),
                Err(error) => format!("{error:?}"),
            })
            .into()),
            Self::Provide => {
                registrar.provide_package(word(0), word(1))?;
                Ok(Value::Empty.into())
            }
            Self::Loaded => {
                registrar.library_loaded(word(0), word(1))?;
                Ok(Value::Empty.into())
            }
        }
    }
}

fn engine() -> TclVmEngine {
    let mut engine = TclVmEngine::new();
    let commands: [(&str, Rc<dyn HostCommand>); 12] = [
        ("hbreak", Rc::new(Completes(CompletionCode::Break, ""))),
        (
            "hcontinue",
            Rc::new(Completes(CompletionCode::Continue, "")),
        ),
        (
            "hreturn",
            Rc::new(Completes(CompletionCode::Return, "from the command")),
        ),
        ("hok", Rc::new(Completes(CompletionCode::Ok, "ok"))),
        ("tick", Rc::new(Completes(CompletionCode::Ok, ""))),
        ("hset", Rc::new(Door::Set)),
        ("hget", Rc::new(Door::Get)),
        ("hunset", Rc::new(Door::Unset)),
        ("heval", Rc::new(Door::Eval)),
        ("hevalerr", Rc::new(Door::EvalReport)),
        ("hprovide", Rc::new(Door::Provide)),
        ("hloaded", Rc::new(Door::Loaded)),
    ];
    for (name, command) in commands {
        engine.define_command(name, command).expect("registers");
    }
    engine
}

/// Run `body` as a parameterless unit, and answer its result as text.
fn run(engine: &mut TclVmEngine, body: &str) -> Result<String, EngineError> {
    let handle = engine
        .compile(CompileUnit {
            name: "doors",
            parameters: &[],
            body,
        })
        .expect("compiles");
    engine
        .invoke(&handle, &[])
        .map(|value| value.as_str().unwrap_or_default().to_owned())
}

fn script_error(message: &str) -> EngineError {
    EngineError::Script {
        message: message.to_owned(),
        code: None,
    }
}

#[test]
fn a_host_command_completes_with_the_code_it_answers() {
    let mut engine = engine();
    assert_eq!(
        run(
            &mut engine,
            "set i 0; foreach x {a b c} { incr i; hbreak; incr i 100 }; set i"
        ),
        Ok("1".to_owned()),
        "a break ends the loop the command is in"
    );
    assert_eq!(
        run(
            &mut engine,
            "set i 0; foreach x {a b c} { incr i; hcontinue; incr i 100 }; set i"
        ),
        Ok("3".to_owned()),
        "a continue skips the rest of the iteration and not the loop"
    );
    assert_eq!(
        run(&mut engine, "hreturn; return nope"),
        Ok("from the command".to_owned()),
        "a return ends the procedure with the command's value"
    );
    assert_eq!(
        run(
            &mut engine,
            "set i 0; foreach x {a b c} { incr i; hok; incr i 100 }; set i"
        ),
        Ok("303".to_owned()),
        "a normal completion leaves the loop alone"
    );
}

#[test]
fn a_break_outside_a_loop_is_what_tcl_says_of_it() {
    let mut engine = engine();
    for (command, message) in [
        ("hbreak", "invoked \"break\" outside of a loop"),
        ("hcontinue", "invoked \"continue\" outside of a loop"),
    ] {
        assert_eq!(
            run(&mut engine, command),
            Err(EngineError::Script {
                message: message.to_owned(),
                code: Some("TCL RESULT UNEXPECTED".to_owned()),
            })
        );
    }
}

#[test]
fn a_host_command_reads_writes_and_unsets_in_the_frame_that_called_it() {
    let mut engine = engine();
    assert_eq!(
        run(&mut engine, "hset x 7; set x"),
        Ok("7".to_owned()),
        "a write lands in the calling procedure's frame"
    );
    assert_eq!(
        run(&mut engine, "hset x 7; info exists ::x"),
        Ok("0".to_owned()),
        "and not in the global one"
    );
    assert_eq!(run(&mut engine, "set y 5; hget y"), Ok("5".to_owned()));
    assert_eq!(
        run(&mut engine, "hget nosuch"),
        Err(script_error("can't read \"nosuch\": no such variable")),
        "a variable that is not there is Tcl's own error"
    );
    assert_eq!(
        run(&mut engine, "set a(1) 1; hget a"),
        Err(script_error("can't read \"a\": variable is array")),
        "as is an array read as a scalar"
    );
    assert_eq!(
        run(&mut engine, "set a(1) 1; hget a(2)"),
        Err(script_error(
            "can't read \"a(2)\": no such element in array"
        )),
        "and an element that is not in its array"
    );
    assert_eq!(
        run(&mut engine, "hset arr(k) v; set arr(k)"),
        Ok("v".to_owned()),
        "an array element is named as a script names it"
    );
    assert_eq!(
        run(&mut engine, "set z 1; hunset z; info exists z"),
        Ok("0".to_owned()),
        "an unset removes the variable"
    );
    assert_eq!(
        run(&mut engine, "set arr(k) 1; hunset arr(k); array size arr"),
        Ok("0".to_owned()),
        "an element too"
    );
    assert_eq!(
        run(&mut engine, "hunset nosuch"),
        Err(EngineError::Script {
            message: "can't unset \"nosuch\": no such variable".to_owned(),
            code: Some("TCL UNSET VARNAME".to_owned()),
        }),
        "one that is not there is Tcl's own error, code included"
    );
    assert_eq!(
        run(&mut engine, "set a(1) 1; hset a 2"),
        Err(script_error("can't set \"a\": variable is array")),
        "a store the VM refuses is its error"
    );
}

#[test]
fn a_host_command_evaluates_a_script_in_the_calling_frame() {
    let mut engine = engine();
    assert_eq!(
        run(&mut engine, "set loc 3; heval {incr loc 10}; set loc"),
        Ok("13".to_owned()),
        "the script runs in the frame that called the command"
    );
    assert_eq!(
        run(&mut engine, "set loc 3; heval {expr {$loc * 2}}"),
        Ok("6".to_owned()),
        "and its result is the command's"
    );
    assert_eq!(
        run(
            &mut engine,
            "set i 0; foreach x {a b} { incr i; heval break; incr i 100 }; set i"
        ),
        Ok("1".to_owned()),
        "a code the script completes with is the one the command answers"
    );
    assert_eq!(
        run(
            &mut engine,
            "set i 0; foreach x {a b c} { incr i; heval continue; incr i 100 }; set i"
        ),
        Ok("3".to_owned()),
        "a continue the script completes with is the command's too"
    );
    assert_eq!(
        run(&mut engine, "heval {return fromeval}; return nope"),
        Ok("fromeval".to_owned()),
        "and so is a return"
    );
    assert_eq!(
        run(&mut engine, "heval {heval {set deep 1}}"),
        Ok("1".to_owned()),
        "a script can call a command that evaluates one"
    );
}

#[test]
fn an_error_in_an_evaluated_script_keeps_its_message_and_its_code() {
    let mut engine = engine();
    assert_eq!(
        run(&mut engine, "heval {error boom {} {MY CODE}}"),
        Err(EngineError::Script {
            message: "boom".to_owned(),
            code: Some("MY CODE".to_owned()),
        })
    );
    assert_eq!(
        run(&mut engine, "heval {error plain}"),
        Err(EngineError::Script {
            message: "plain".to_owned(),
            code: Some("NONE".to_owned()),
        })
    );
}

#[test]
fn a_script_that_completes_with_a_code_of_its_own_gives_the_command_that_code() {
    let mut engine = engine();
    assert_eq!(
        run(
            &mut engine,
            "list [catch {heval {return -level 0 -code 5 x}} m] $m"
        ),
        Ok("5 x".to_owned()),
        "the command completes with the code, and `catch` reports it"
    );
}

#[test]
fn a_command_whose_script_outruns_the_budget_is_told_it_was_the_budget() {
    let mut engine = engine();
    engine
        .set_budget(Budget::of_commands(5))
        .expect("sets the budget");
    assert_eq!(
        run(
            &mut engine,
            "hevalerr {foreach i {1 2 3 4 5 6 7 8 9 10 11 12} {tick}}"
        ),
        Ok("BudgetExceeded(Commands)".to_owned()),
        "the door reports a budget as one, not as a script's error"
    );
}

#[test]
fn a_script_a_host_command_evaluates_is_charged_to_the_invocations_budget() {
    let mut engine = engine();
    engine
        .set_budget(Budget::of_commands(5))
        .expect("sets the budget");
    assert_eq!(
        run(
            &mut engine,
            "heval {foreach i {1 2 3 4 5 6 7 8 9 10 11 12} {tick}}"
        ),
        Err(EngineError::BudgetExceeded(BudgetKind::Commands)),
        "a script that outruns the budget fails as the budget does"
    );
    assert_eq!(
        run(&mut engine, "heval {tick}; hok"),
        Ok("ok".to_owned()),
        "and one inside it does not"
    );
}

#[test]
fn a_host_command_provides_a_package_a_later_require_finds() {
    let mut engine = engine();
    assert_eq!(
        run(&mut engine, "package require pkgq"),
        Err(script_error("can't find package pkgq")),
        "nothing provides it yet"
    );
    assert_eq!(
        run(&mut engine, "hprovide pkgq 1.0; package require pkgq"),
        Ok("1.0".to_owned())
    );
    assert_eq!(
        run(&mut engine, "hprovide pkgq 1.0; package provide pkgq"),
        Ok("1.0".to_owned()),
        "the same version again is a no-op"
    );
    assert_eq!(
        run(&mut engine, "hprovide pkgq 2.0"),
        Err(EngineError::Script {
            message: "conflicting versions provided for package \"pkgq\": 1.0, then 2.0".to_owned(),
            code: Some("TCL PACKAGE VERSIONCONFLICT".to_owned()),
        }),
        "a different version is the error `package provide` gives"
    );
}

#[test]
fn a_host_command_names_a_loaded_library_info_loaded_lists() {
    let mut engine = engine();
    assert_eq!(run(&mut engine, "info loaded"), Ok(String::new()));
    assert_eq!(
        run(
            &mut engine,
            "hloaded {} Pkga; hloaded /opt/pkgb/libpkgb.so Pkgb; info loaded"
        ),
        Ok("{{} Pkga} {/opt/pkgb/libpkgb.so Pkgb}".to_owned())
    );
    assert_eq!(
        run(&mut engine, "info loaded {} Pkgb"),
        Ok("/opt/pkgb/libpkgb.so".to_owned())
    );
}

#[test]
fn an_embedder_reads_writes_and_evaluates_through_the_engine_itself() {
    let mut engine = engine();
    engine
        .set_variable("g", Value::Int(5))
        .expect("a global is set");
    assert_eq!(
        engine.variable("g").expect("reads").as_str(),
        Some("5"),
        "an integer comes back as the text it is"
    );
    assert_eq!(
        run(&mut engine, "return $::g"),
        Ok("5".to_owned()),
        "a unit sees the global the embedder set"
    );
    assert_eq!(
        engine.variable("nosuch").map(|_| ()),
        Err(script_error("can't read \"nosuch\": no such variable"))
    );
    assert_eq!(engine.unset_variable("g"), Ok(()));
    assert!(
        engine.unset_variable("g").is_err(),
        "a second unset has nothing to remove"
    );

    let outcome = engine.eval_in_invocation("set g2 9").expect("evaluates");
    assert_eq!(outcome.value.as_str(), Some("9"));
    assert_eq!(outcome.code, CompletionCode::Ok);
    assert_eq!(
        engine
            .eval_in_invocation("break")
            .expect("a break is an outcome")
            .code,
        CompletionCode::Break
    );
    assert!(
        matches!(
            engine.eval_in_invocation("error x"),
            Err(EngineError::Script { ref message, ref code })
                if message == "x" && code.as_deref() == Some("NONE")
        ),
        "an error is the error"
    );
}

#[test]
fn an_embedder_provides_a_package_and_a_library_through_the_engine_itself() {
    let mut engine = engine();
    engine.provide_package("e", "3.1").expect("provides");
    assert_eq!(
        engine
            .eval_in_invocation("package require e")
            .expect("evaluates")
            .value
            .as_str(),
        Some("3.1")
    );
    assert!(
        engine.provide_package("e", "4.0").is_err(),
        "a conflicting version is refused"
    );
    engine.library_loaded("", "Lib").expect("records");
    assert_eq!(
        engine
            .eval_in_invocation("info loaded")
            .expect("evaluates")
            .value
            .as_str(),
        Some("{{} Lib}")
    );
}
