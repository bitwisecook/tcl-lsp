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

//! The runtime as an engine of the extension interface: what the bytecode VM's
//! engine is held to (`rust/tcl-engine-tclvm`), held to this interpreter — the
//! units, the host commands and their door, the three budgets and why a body
//! cannot catch its way past one, the whitelist, the pinned release and the
//! confined stores. The bodies need `expr` and the loop commands, so the suite
//! needs the numeric tower.
#![cfg(all(feature = "engine", have_tommath))]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use core::ffi::{c_int, c_void};

use tcl_engine_api::{
    Budget, BudgetKind, CommandRegistrar, CompileUnit, CompletionCode, Engine, EngineError,
    HostCommand, HostOutcome, Value,
};
use tcl_runtime::engine::RuntimeEngine;
use tcl_runtime::interp::Interp;
use tcl_runtime::obj::TclObj;
use tcl_runtime_api::{FrameId, Frames, VarStore};

/// A host command that records the words it is called with.
#[derive(Default)]
struct Collector {
    emitted: RefCell<Vec<Vec<String>>>,
}

impl HostCommand for Collector {
    fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        self.emitted.borrow_mut().push(
            arguments
                .iter()
                .map(|argument| argument.as_str().unwrap_or_default().to_owned())
                .collect(),
        );
        Ok(Value::Empty.into())
    }
}

fn unit(body: &str) -> CompileUnit<'_> {
    CompileUnit {
        name: "test",
        parameters: &["words", "ctx"],
        body,
    }
}

fn no_arguments() -> [Value; 2] {
    [Value::list([]), Value::dict_of::<&str>([])]
}

/// Compile and invoke `body` on `engine` with empty `words` and `ctx`, and
/// answer its value's text: the engine answers strings.
fn run(engine: &mut RuntimeEngine, body: &str) -> Result<String, EngineError> {
    let handle = engine.compile(unit(body)).expect("compiles");
    engine
        .invoke(&handle, &no_arguments())
        .map(|value| value.as_str().expect("a string").to_owned())
}

#[test]
fn a_unit_compiles_once_and_invokes_with_structured_values() {
    let mut engine = RuntimeEngine::new();
    assert_eq!(engine.name(), "runtime");
    let handle = engine
        .compile(unit("return [llength $words]"))
        .expect("the body compiles");
    for count in 1..4_i64 {
        let words = Value::list((0..count).map(Value::Int));
        let ctx = Value::dict_of([("nwords", Value::Int(count))]);
        let result = engine
            .invoke(&handle, &[words, ctx])
            .expect("the body runs");
        assert_eq!(result.as_str(), Some(count.to_string().as_str()));
    }
    let error = engine
        .invoke(&handle, &[Value::list([])])
        .expect_err("a unit takes its parameters");
    assert!(
        matches!(&error, EngineError::Script { message, .. }
            if message == "wrong # args: unit takes 2 argument(s), got 1"),
        "{error:?}"
    );
}

#[test]
fn a_dict_crosses_as_a_dict() {
    let mut engine = RuntimeEngine::new();
    let handle = engine
        .compile(unit("return [dict get $ctx command]"))
        .expect("compiles");
    let ctx = Value::dict_of([
        ("command", Value::string("mylib::sort")),
        ("nwords", Value::Int(0)),
    ]);
    let result = engine
        .invoke(&handle, &[Value::list([]), ctx])
        .expect("runs");
    assert_eq!(result.as_str(), Some("mylib::sort"));
}

#[test]
fn a_host_command_receives_what_the_body_emits() {
    let mut engine = RuntimeEngine::new();
    let collector = Rc::new(Collector::default());
    engine
        .define_command("role", collector.clone())
        .expect("registers");
    run(&mut engine, "role 0 varwrite\nrole 1 body").expect("runs");
    assert_eq!(
        *collector.emitted.borrow(),
        vec![
            vec!["0".to_owned(), "varwrite".to_owned()],
            vec!["1".to_owned(), "body".to_owned()],
        ],
    );
    assert_eq!(engine.remove_command("role"), Ok(true));
    assert_eq!(engine.remove_command("role"), Ok(false), "it is gone");
    let error = run(&mut engine, "role 2 x").expect_err("removed");
    assert!(
        matches!(&error, EngineError::Script { message, .. }
            if message == "invalid command name \"role\""),
        "{error:?}"
    );
}

/// A host command that creates another through the door it is given, and
/// provides a package there.
struct Factory(Rc<Collector>);

impl HostCommand for Factory {
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::Unsupported("a factory needs the door"))
    }

    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        _arguments: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        registrar.define_command("made", self.0.clone())?;
        registrar.provide_package("factory", "1.2")?;
        Ok(Value::string("built").into())
    }
}

#[test]
fn a_host_command_registers_another_and_provides_a_package_through_its_door() {
    let mut engine = RuntimeEngine::new();
    let collector = Rc::new(Collector::default());
    engine
        .define_command("factory", Rc::new(Factory(collector.clone())))
        .expect("registers");
    let answer = run(
        &mut engine,
        "set built [factory]\nmade $built 2\nreturn [package present factory]",
    )
    .expect("a command a host command registered runs at once");
    assert_eq!(answer, "1.2");
    assert_eq!(
        *collector.emitted.borrow(),
        vec![vec!["built".to_owned(), "2".to_owned()]],
    );
    let conflict = engine.provide_package("factory", "2.0");
    assert!(
        matches!(&conflict, Err(EngineError::Script { message, code })
            if message == "conflicting versions provided for package \"factory\": 1.2, then 2.0"
                && code.as_deref() == Some("TCL PACKAGE VERSIONCONFLICT")),
        "{conflict:?}"
    );
}

/// A host command's outcome is the completion it names: a break ends the
/// enclosing loop, a continue skips to its next iteration, a code of its own
/// is what `catch` reports, an error carries its `-errorcode`, and a return
/// with options takes effect as `return -options` does.
struct Answering;

impl HostCommand for Answering {
    fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        let value = Value::string(arguments[1].as_str().unwrap_or_default());
        match arguments[0].as_str() {
            Some("break") => Ok(HostOutcome::completing(CompletionCode::Break, value)),
            Some("continue") => Ok(HostOutcome::completing(CompletionCode::Continue, value)),
            Some("seven") => Ok(HostOutcome::completing(CompletionCode::Other(7), value)),
            Some("budget") => Err(EngineError::BudgetExceeded(BudgetKind::WallClock)),
            Some("error") => Err(EngineError::Script {
                message: "refused".to_owned(),
                code: Some("HOST REFUSED".to_owned()),
            }),
            Some("return") => Ok(HostOutcome::returning(
                value,
                Value::list([
                    Value::string("-code"),
                    Value::string("error"),
                    Value::string("-errorcode"),
                    Value::string("X Y"),
                ]),
            )),
            _ => Ok(value.into()),
        }
    }
}

#[test]
fn a_host_command_s_completion_is_the_one_it_names() {
    let mut engine = RuntimeEngine::new();
    engine
        .define_command("answer", Rc::new(Answering))
        .expect("registers");
    for (body, expected) in [
        ("set n 0\nwhile 1 {incr n; if {$n == 3} {answer break x}}\nreturn $n", "3"),
        (
            "set seen {}\nforeach i {1 2 3} {if {$i == 2} {answer continue x}; lappend seen $i}\nreturn $seen",
            "1 3",
        ),
        ("return [list [catch {answer seven v} r] $r]", "7 v"),
        (
            "return [list [catch {answer error v} r o] $r [dict get $o -errorcode]]",
            "1 refused {HOST REFUSED}",
        ),
        (
            "proc p {} {answer return msg; return after}\nreturn [list [catch p r o] $r [dict get $o -errorcode]]",
            "1 msg {X Y}",
        ),
        ("return [answer plain v]", "v"),
    ] {
        assert_eq!(
            run(&mut engine, body),
            Ok(expected.to_owned()),
            "{body}"
        );
    }
    assert_eq!(
        run(&mut engine, "catch {answer budget v}\nreturn after"),
        Err(EngineError::BudgetExceeded(BudgetKind::WallClock)),
        "a budget a host command's own work outran is the invocation's"
    );
    let error = run(&mut engine, "answer error v").expect_err("an uncaught error");
    assert_eq!(
        error,
        EngineError::Script {
            message: "refused".to_owned(),
            code: Some("HOST REFUSED".to_owned()),
        }
    );
}

/// A host command that panics.
struct Panicking;

impl HostCommand for Panicking {
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        panic!("the host command broke");
    }
}

/// A panic in a host command reaches the embedder as a panic, as it would if
/// the command were called from Rust — where the hook host's containment
/// turns it into a crash record — and the engine keeps working afterwards.
#[test]
fn a_host_command_s_panic_reaches_the_embedder_and_the_engine_survives_it() {
    let mut engine = RuntimeEngine::new();
    engine
        .define_command("broken", Rc::new(Panicking))
        .expect("registers");
    let handle = engine
        .compile(unit("catch {broken}\nreturn caught"))
        .expect("compiles");
    let arguments = no_arguments();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine.invoke(&handle, &arguments)
    }));
    let payload = caught.expect_err("the panic is resumed past the body's catch");
    assert_eq!(
        payload.downcast_ref::<&str>().copied(),
        Some("the host command broke")
    );
    assert_eq!(
        run(&mut engine, "return [expr {6 * 7}]"),
        Ok("42".to_owned())
    );
}

#[test]
fn an_error_in_the_body_is_reported_with_its_code() {
    let mut engine = RuntimeEngine::new();
    assert_eq!(
        run(&mut engine, "error boom"),
        Err(EngineError::Script {
            message: "boom".to_owned(),
            code: Some("NONE".to_owned()),
        })
    );
    assert_eq!(
        run(&mut engine, "error boom {} {MY CODE}"),
        Err(EngineError::Script {
            message: "boom".to_owned(),
            code: Some("MY CODE".to_owned()),
        })
    );
}

#[test]
fn the_command_budget_is_enforced_and_distinguishable() {
    let mut engine = RuntimeEngine::new();
    engine
        .set_budget(Budget::of_commands(200).with_wall_clock(Duration::from_secs(5)))
        .expect("the runtime enforces both");
    let error = run(
        &mut engine,
        "set i 0\nwhile {$i < 100000} { set x [string length abc]\n incr i }",
    )
    .expect_err("the budget stops it");
    assert_eq!(error, EngineError::BudgetExceeded(BudgetKind::Commands));
    let spent = engine.commands_spent().expect("the runtime counts");
    assert!(spent > 200 && spent < 210, "{spent}");
    // The next invocation starts with a full budget.
    assert_eq!(run(&mut engine, "return ok"), Ok("ok".to_owned()));
}

/// A body that catches the error its budget raised does not run on: every
/// command after the limit fails the same way, so a host command after it is
/// never called, and the invocation reports the budget however the body ended.
#[test]
fn a_body_cannot_catch_its_way_past_a_budget() {
    let mut engine = RuntimeEngine::new();
    let collector = Rc::new(Collector::default());
    engine
        .define_command("role", collector.clone())
        .expect("registers");
    engine
        .set_budget(Budget::of_commands(100).with_max_value_bytes(64))
        .expect("the runtime enforces it");
    assert_eq!(
        run(
            &mut engine,
            "catch {string repeat abcdefgh 100}\nrole after"
        ),
        Err(EngineError::BudgetExceeded(BudgetKind::ValueSize))
    );
    assert!(
        collector.emitted.borrow().is_empty(),
        "nothing ran past the refused allocation"
    );
    for body in [
        "catch {while 1 {incr i}}\nset after 1\nreturn ran-on",
        "catch {while 1 {incr i}}",
        "try {while 1 {incr i}} on error {} {return handled}",
    ] {
        assert_eq!(
            run(&mut engine, body),
            Err(EngineError::BudgetExceeded(BudgetKind::Commands)),
            "{body}"
        );
    }
}

/// One `string repeat` spends a single command and no measurable time while
/// asking for as much memory as it likes: the value-size budget refuses it
/// before anything is allocated, and the same call under the cap still works.
#[test]
fn the_value_size_budget_stops_an_allocation_the_other_two_cannot_see() {
    let mut engine = RuntimeEngine::new();
    engine
        .set_budget(
            Budget::of_commands(1_000_000)
                .with_wall_clock(Duration::from_secs(5))
                .with_max_value_bytes(1024),
        )
        .expect("the runtime enforces all three");
    assert_eq!(
        run(&mut engine, "string repeat aaaaaaaa 1000000"),
        Err(EngineError::BudgetExceeded(BudgetKind::ValueSize))
    );
    assert_eq!(
        run(&mut engine, "string repeat ab 8"),
        Ok("abababababababab".to_owned())
    );
}

/// A loop that dispatches nothing never meets the command budget: the wall
/// clock is what stops it, promptly.
#[test]
fn the_wall_clock_budget_stops_a_loop_the_command_budget_cannot_see() {
    let mut engine = RuntimeEngine::new();
    engine
        .set_budget(Budget::of_commands(1_000_000).with_wall_clock(Duration::from_millis(50)))
        .expect("the runtime enforces both");
    let started = std::time::Instant::now();
    assert_eq!(
        run(&mut engine, "while {1} {}"),
        Err(EngineError::BudgetExceeded(BudgetKind::WallClock))
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the cap fires promptly: {:?}",
        started.elapsed()
    );
}

#[test]
fn a_restricted_engine_has_only_the_whitelist_and_the_host_commands() {
    let mut engine = RuntimeEngine::new();
    let collector = Rc::new(Collector::default());
    engine
        .define_command("fold", collector.clone())
        .expect("registers");
    let before = engine
        .compile(unit("fold [string length [lindex $words 0]]"))
        .expect("compiles");
    engine
        .restrict_commands(&["set", "expr", "if", "return", "string", "lindex", "llength"])
        .expect("restricts");
    engine
        .invoke(
            &before,
            &[
                Value::list([Value::string("abcde")]),
                Value::dict_of::<&str>([]),
            ],
        )
        .expect("a unit compiled before the whitelist still runs");
    assert_eq!(*collector.emitted.borrow(), vec![vec!["5".to_owned()]]);
    for body in [
        "open /etc/passwd",
        "exec ls",
        "source /dev/null",
        "proc p {} {}",
        "unknown x",
    ] {
        let error = run(&mut engine, body).expect_err("a command off the whitelist");
        assert!(
            matches!(&error, EngineError::Script { message, .. }
                if message.contains("invalid command name")),
            "{body}: {error:?}"
        );
    }
}

/// A whitelist that names an ensemble keeps the commands its subcommands are,
/// spelt either way; one it does not name keeps none of its own.
#[test]
fn a_whitelisted_ensembles_subcommands_run_wherever_they_are_called() {
    let mut engine = RuntimeEngine::new();
    engine
        .restrict_commands(&["set", "return", "string", "dict"])
        .expect("restricts");
    for (body, expected) in [
        ("set r [string cat a - b]\nreturn $r", "a-b"),
        ("return [string toupper [string trim { x }]]", "X"),
        ("set d [dict create k v]\nreturn [dict get $d k]", "v"),
        ("return [::tcl::string::insert ab 1 x]", "axb"),
    ] {
        assert_eq!(run(&mut engine, body), Ok(expected.to_owned()), "{body}");
    }
    let mut dicts_only = RuntimeEngine::new();
    dicts_only
        .restrict_commands(&["set", "return", "dict"])
        .expect("restricts");
    for body in [
        "return [::tcl::string::insert ab 1 x]",
        "return [string trim { x }]",
    ] {
        assert!(run(&mut dicts_only, body).is_err(), "{body}");
    }
}

#[test]
fn return_is_an_ordinary_early_exit() {
    let mut engine = RuntimeEngine::new();
    assert_eq!(
        run(
            &mut engine,
            "if {[llength $words] == 0} { return }\nreturn folded"
        ),
        Ok(String::new())
    );
}

/// `set_release` pins the interpreter's release: a leading zero is octal up
/// to 8.6 and under iRules, and decimal from 9.0; every operation leaves the
/// thread's own numeral grammar as it found it; and a name that is no release
/// the runtime runs is refused.
#[test]
fn set_release_pins_the_numeral_grammar() {
    for (profile, want, thread) in [
        ("tcl8.6", "8", tcl_syntax::number::NumberSyntax::Tcl90),
        ("tcl9.0", "10", tcl_syntax::number::NumberSyntax::Tcl84),
        ("f5-irules", "8", tcl_syntax::number::NumberSyntax::Tcl90),
    ] {
        // The thread reads numerals by a release other than the engine's.
        tcl_syntax::number::set_runtime_syntax(thread);
        let before = tcl_syntax::number::runtime_syntax();
        let mut engine = RuntimeEngine::new();
        engine
            .set_release(profile)
            .expect("a catalogue profile pins");
        assert_eq!(engine.release(), Some(profile));
        assert_eq!(
            tcl_syntax::number::runtime_syntax(),
            before,
            "{profile}: building and pinning leave the thread's grammar"
        );
        assert_eq!(
            run(&mut engine, "return [expr {010 + 0}]"),
            Ok(want.to_owned()),
            "{profile}"
        );
        assert_eq!(
            tcl_syntax::number::runtime_syntax(),
            before,
            "{profile}: the thread keeps its own grammar"
        );
        assert_eq!(
            engine.set_release(profile),
            Ok(()),
            "{profile}: the same pin again"
        );
        let other = if profile == "tcl9.0" {
            "tcl8.6"
        } else {
            "tcl9.0"
        };
        assert_eq!(
            engine.set_release(other),
            Err(EngineError::Unsupported(
                "pinning a release after a unit was compiled"
            )),
            "{profile}"
        );
    }
    for name in ["no-such-dialect", "", "tcl", "tk", "f5-bigip"] {
        let mut engine = RuntimeEngine::new();
        assert_eq!(
            engine.set_release(name),
            Err(EngineError::Unsupported("pinning a release")),
            "{name:?}"
        );
        assert_eq!(engine.release(), None);
    }
}

/// `confine_stores` keeps every write in the invocation's own frame: each body
/// below writes somewhere else and is refused, as a Tcl error raised before
/// anything is written. A caught error publishes neither `::errorInfo` nor
/// `::errorCode`, the same writes to locals succeed, a global still reads,
/// and the random generator, whose seed every invocation shares, refuses.
#[test]
fn confine_stores_refuses_every_store_outside_the_activation() {
    for (body, written) in [
        ("set ::g 1", "::g"),
        ("incr ::counter", "::counter"),
        ("lappend ::l x", "::l"),
        ("append ::ap x", "::ap"),
        ("set ::a(k) 1", "::a"),
        ("foreach ::x {1 2} {}", "::x"),
        ("lassign {1 2} ::p q", "::p"),
        ("regexp {(a)} a ::m", "::m"),
        ("regsub a abc b ::rs", "::rs"),
        ("scan 5 %d ::n", "::n"),
        ("dict set ::d k v", "::d"),
        ("dict unset ::du k", "::du"),
        ("dict lappend ::dl k v", "::dl"),
        ("dict incr ::di k", "::di"),
        ("dict append ::da k v", "::da"),
        ("binary scan A a ::b", "::b"),
        ("namespace eval ::ns {variable v 1}", "::ns::v"),
        ("global g2; set g2 1", "::g2"),
        ("upvar 0 ::g3 alias; set alias 1", "::g3"),
        ("uplevel 1 {set up 1}", "::up"),
    ] {
        let mut engine = RuntimeEngine::new();
        engine
            .confine_stores()
            .expect("the runtime confines its stores");
        let answer = run(&mut engine, body);
        // The runtime's `regexp` reports a store it could not make in words of
        // its own, where C Tcl leaves the variable's error; the store is still
        // refused, which is what this asserts for it.
        let expected = if body.starts_with("regexp") {
            "couldn't set match variable"
        } else {
            "stores are confined to the activation"
        };
        assert!(
            matches!(&answer, Err(EngineError::Script { message, .. })
                if message.contains(expected)),
            "{body}: {answer:?}"
        );
        assert_eq!(
            run(&mut engine, &format!("return [info exists {written}]")),
            Ok("0".to_owned()),
            "{body}: nothing was written"
        );
    }

    let mut engine = RuntimeEngine::new();
    engine.confine_stores().expect("confines");
    let published = "return [list [info exists ::errorInfo] [info exists ::errorCode]]";
    for body in [
        format!("catch {{lindex {{}} y}}; {published}"),
        published.to_owned(),
    ] {
        assert_eq!(run(&mut engine, &body), Ok("0 0".to_owned()), "{body}");
    }

    let mut engine = RuntimeEngine::new();
    run(&mut engine, "set ::seen yes; set ::grown {a}").expect("an open engine writes globals");
    engine.confine_stores().expect("confines");
    assert_eq!(
        run(
            &mut engine,
            "set acc {}; foreach x {a b} {lappend acc $x}; incr n; dict set d k v\n\
             set arr(k) 1; lassign {1 2} p q; regexp {(a)} a whole m\n\
             regsub a abc b rs; dict lappend dl k v; dict incr di k\n\
             return [list $acc $n $d $arr(k) $p $m $rs $dl $di $::seen]",
        ),
        Ok("{a b} 1 {k v} 1 1 a bbc {k v} {k 1} yes".to_owned())
    );
    // A global the host left is read, but appending to it in place is a store.
    assert!(run(&mut engine, "lappend ::grown b").is_err());
    assert_eq!(run(&mut engine, "return $::grown"), Ok("a".to_owned()));

    for body in ["expr {srand(7)}", "expr {rand()}"] {
        let mut engine = RuntimeEngine::new();
        engine.confine_stores().expect("confines");
        let answer = run(&mut engine, body);
        assert!(
            matches!(&answer, Err(EngineError::Script { message, .. })
                if message.contains("stores are confined to the activation")),
            "{body}: {answer:?}"
        );
    }
}

/// A confined engine reads no host environment: the globals the host seeded
/// (`::env`, `::tcl_platform`, `::tcl_library`) are gone, so a body cannot
/// fold the analysing machine into an answer about a program that runs
/// elsewhere.
#[test]
fn a_confined_engine_reads_no_host_environment() {
    let seeded = "return [list [info exists ::env] [info exists ::tcl_platform] \
                  [info exists ::tcl_library]]";
    let mut open = RuntimeEngine::new();
    assert_eq!(run(&mut open, seeded), Ok("1 1 1".to_owned()));
    let mut confined = RuntimeEngine::new();
    confined.confine_stores().expect("confines");
    assert_eq!(run(&mut confined, seeded), Ok("0 0 0".to_owned()));
    for body in [
        "return $::tcl_platform(byteOrder)",
        "return $::tcl_library",
        "return $::env(PATH)",
    ] {
        assert!(
            matches!(run(&mut confined, body), Err(EngineError::Script { .. })),
            "{body}"
        );
    }
}

/// An engine restricted to a whitelist that allows `expr` keeps its math
/// functions, but not `rand` and `srand`.
#[test]
fn a_restricted_engine_keeps_the_math_functions_but_the_generator() {
    for release in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let mut engine = RuntimeEngine::new();
        engine.set_release(release).expect("pins");
        engine
            .restrict_commands(&["expr", "return"])
            .expect("restricts");
        engine.confine_stores().expect("confines");
        assert_eq!(
            run(
                &mut engine,
                "return [expr {abs(-1) + int(2.5) + double(1)}]"
            ),
            Ok("4.0".to_owned()),
            "{release}"
        );
        for body in ["return [expr {rand()}]", "return [expr {srand(7)}]"] {
            assert!(run(&mut engine, body).is_err(), "{release}: {body}");
        }
    }
    // The whitelist alone removes the generator, without the confinement.
    let mut engine = RuntimeEngine::new();
    engine
        .restrict_commands(&["expr", "return"])
        .expect("restricts");
    for body in ["return [expr {rand()}]", "return [expr {srand(7)}]"] {
        assert!(run(&mut engine, body).is_err(), "{body}");
    }
    assert_eq!(
        run(&mut engine, "return [expr {abs(-2)}]"),
        Ok("2".to_owned())
    );
}

#[test]
fn each_invocation_gets_its_own_locals() {
    let mut engine = RuntimeEngine::new();
    let handle = engine
        .compile(unit(
            "if {[info exists leak]} { return leaked }\nset leak 1\nreturn fresh",
        ))
        .expect("compiles");
    for _ in 0..3 {
        let answer = engine.invoke(&handle, &no_arguments()).expect("runs");
        assert_eq!(answer.as_str(), Some("fresh"));
    }
}

/// A command that stores `after` into the variable its first word names in the
/// frame its second word names, through the frame-addressed store the shared
/// command core and compiled code use (`VarStore::set` at another frame), and
/// not through a script command.
unsafe extern "C" fn store_at(
    _client_data: *mut c_void,
    interp: *mut Interp,
    objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: dispatch passes its live interpreter and `objc` live words.
    let (interp, words) = unsafe {
        (
            &mut *interp,
            std::slice::from_raw_parts(objv, usize::try_from(objc).unwrap_or(0)),
        )
    };
    let text = |word: *mut TclObj| {
        // SAFETY: each word is live for the call.
        unsafe { core::ffi::CStr::from_ptr(tcl_runtime::capi::Tcl_GetString(word)) }
            .to_string_lossy()
            .into_owned()
    };
    let (name, up) = (text(words[1]), text(words[2]));
    let level = Frames::current(interp).0 - up.parse::<usize>().expect("a level");
    // SAFETY: a fresh object the store takes its own hold on.
    let value = unsafe { tcl_runtime::capi::Tcl_NewStringObj(c"after".as_ptr(), -1) };
    VarStore::set(interp, FrameId(level), &name, value);
    0
}

/// The frame-addressed stores are confined too: a store into the caller's
/// frame, or the global one, made through `VarStore::set` by a command a
/// procedure calls is refused as a store from a script is, where an open
/// engine makes it.
#[test]
fn a_frame_addressed_store_outside_the_activation_is_refused() {
    for (confined, expected) in [(false, "after after"), (true, "before before")] {
        let mut engine = RuntimeEngine::new();
        // SAFETY: the interpreter is live and the procedure is this file's.
        unsafe {
            tcl_runtime::capi::Tcl_CreateObjCommand(
                engine.interp_mut(),
                c"store_at".as_ptr(),
                Some(store_at),
                core::ptr::null_mut(),
                None,
            );
        }
        run(
            &mut engine,
            "set ::g before\n\
             proc outer {} { set x before; inner; return $x }\n\
             proc inner {} { store_at x 1 }\n\
             proc global {} { store_at ::g 1 }",
        )
        .expect("an open engine sets up");
        if confined {
            engine.confine_stores().expect("confines");
        }
        assert_eq!(
            run(&mut engine, "global\nreturn [list [outer] $::g]"),
            Ok(expected.to_owned()),
            "confined: {confined}"
        );
    }
}
