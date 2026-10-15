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

//! The global `env` array kept in step with the host environment, as C's
//! `tclEnv.c` keeps it in step with `environ`: element writes and unsets
//! reach the host, element reads and `array` commands refresh from it, and a
//! whole-array unset detaches the array without touching the host. Every
//! expectation here was measured on tclsh 8.6.18 and 9.0.4.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::Vm;

type Vars = Rc<RefCell<BTreeMap<String, String>>>;

/// A host whose environment is a map the test can inspect and change.
struct MapHost {
    clock: MapClock,
    stdio: MapStdIo,
    env: MapEnv,
}

struct MapClock;

impl tcl_platform::Clock for MapClock {
    fn now_secs(&self) -> i64 {
        0
    }

    fn now_millis(&self) -> i128 {
        0
    }
}

struct MapStdIo;

impl tcl_platform::StdIo for MapStdIo {
    fn write_stdout(&self, _bytes: &[u8]) {}

    fn write_stderr(&self, _bytes: &[u8]) {}
}

struct MapEnv(Vars);

impl tcl_platform::Env for MapEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) {
        self.0.borrow_mut().insert(key.to_owned(), value.to_owned());
    }

    fn unset(&self, key: &str) {
        self.0.borrow_mut().remove(key);
    }

    fn vars(&self) -> Vec<(String, String)> {
        self.0
            .borrow()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    fn cwd(&self) -> Result<String, tcl_platform::HostError> {
        Ok("/".to_owned())
    }

    fn chdir(&self, _path: &str) -> Result<(), tcl_platform::HostError> {
        Ok(())
    }
}

impl tcl_platform::Host for MapHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        tcl_platform::Capabilities::empty()
    }

    fn clock(&self) -> &dyn tcl_platform::Clock {
        &self.clock
    }

    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        &self.stdio
    }

    fn env(&self) -> &dyn tcl_platform::Env {
        &self.env
    }
}

fn vars(entries: &[(&str, &str)]) -> Vars {
    Rc::new(RefCell::new(
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    ))
}

/// A VM pinned to `version` whose host environment is `env`.
fn vm_on(version: TclVersion, env: &Vars) -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    vm.set_runtime_version(version);
    vm.set_host(Rc::new(MapHost {
        clock: MapClock,
        stdio: MapStdIo,
        env: MapEnv(Rc::clone(env)),
    }));
    vm
}

/// Evaluate `src`, returning its result or its error message.
fn eval(vm: &mut Vm, src: &str) -> Result<String, String> {
    let completion = vm.eval_source(src).map_err(|error| error.message)?;
    let text = completion.result.to_str().to_string();
    if completion.code.is_ok() {
        Ok(text)
    } else {
        Err(text)
    }
}

fn ok(vm: &mut Vm, src: &str) -> String {
    eval(vm, src).unwrap_or_else(|message| panic!("{src}: {message}"))
}

fn host(env: &Vars, key: &str) -> Option<String> {
    env.borrow().get(key).cloned()
}

const RELEASES: [TclVersion; 2] = [TclVersion::V8_6, TclVersion::V9_0];

#[test]
fn element_writes_reach_the_host_by_every_route() {
    for version in RELEASES {
        let env = vars(&[("SEEDED", "1")]);
        let mut vm = vm_on(version, &env);
        ok(
            &mut vm,
            "set env(SET) a\n\
             append env(APPEND) b c\n\
             lappend env(LAPPEND) x {y z}\n\
             incr env(SEEDED) 4\n\
             array set env {ARRAY1 1 ARRAY2 2}\n\
             lassign {l1 l2} env(LASSIGN1) env(LASSIGN2)\n\
             catch {error caught} env(CATCH)\n\
             foreach env(FOREACH) {first last} {}\n\
             dict set env(DICT) k v\n\
             regexp {(b+)} abbbc -> env(REGEXP)\n\
             scan 12 %d env(SCAN)\n\
             proc viaglobal {} {global env; set env(GLOBAL) g; incr env(SEEDED)}\n\
             viaglobal\n\
             proc viaarray {} {upvar #0 env e; set e(ALIAS) whole}\n\
             viaarray\n\
             set ::env(QUALIFIED) q",
        );
        for (key, value) in [
            ("SET", "a"),
            ("APPEND", "bc"),
            ("LAPPEND", "x {y z}"),
            ("SEEDED", "6"),
            ("ARRAY1", "1"),
            ("ARRAY2", "2"),
            ("LASSIGN1", "l1"),
            ("LASSIGN2", "l2"),
            ("CATCH", "caught"),
            ("FOREACH", "last"),
            ("DICT", "k v"),
            ("REGEXP", "bbb"),
            ("SCAN", "12"),
            ("GLOBAL", "g"),
            ("ALIAS", "whole"),
            ("QUALIFIED", "q"),
        ] {
            assert_eq!(host(&env, key).as_deref(), Some(value), "{version:?} {key}");
        }
    }
}

/// Tcl 9 recovers the array behind an alias to one element, so the array's
/// trace fires through it; 8.x does not, and the write stays in the array.
#[test]
fn an_element_alias_reaches_the_host_from_9_0() {
    for (version, reaches) in [(TclVersion::V8_6, false), (TclVersion::V9_0, true)] {
        let env = vars(&[("HOME", "/root")]);
        let mut vm = vm_on(version, &env);
        let out = ok(
            &mut vm,
            "proc f {} {upvar 0 ::env(HOME) h; set h /alias; upvar #0 env(NEW) n; set n 1}\n\
             f\n\
             set ::env(HOME)",
        );
        let expected = if reaches { "/alias" } else { "/root" };
        assert_eq!(out, expected, "{version:?}");
        assert_eq!(host(&env, "HOME").as_deref(), Some(expected), "{version:?}");
        assert_eq!(host(&env, "NEW").is_some(), reaches, "{version:?}");
    }
}

#[test]
fn element_unsets_reach_the_host() {
    for version in RELEASES {
        let env = vars(&[("A", "1"), ("AB", "2"), ("B", "3"), ("GONE", "4")]);
        let mut vm = vm_on(version, &env);
        ok(&mut vm, "unset env(GONE); array unset env A*");
        assert_eq!(
            env.borrow().keys().cloned().collect::<Vec<_>>(),
            ["B"],
            "{version:?}"
        );
        assert_eq!(
            eval(&mut vm, "unset env(NOSUCH)"),
            Err("can't unset \"env(NOSUCH)\": no such element in array".to_owned())
        );
        assert_eq!(host(&env, "B").as_deref(), Some("3"));
    }
}

#[test]
fn element_reads_refresh_from_the_host() {
    for version in RELEASES {
        let env = vars(&[("CHANGED", "old"), ("DROPPED", "x")]);
        let mut vm = vm_on(version, &env);
        env.borrow_mut().insert("CHANGED".into(), "new".into());
        env.borrow_mut().insert("ADDED".into(), "added".into());
        env.borrow_mut().remove("DROPPED");
        assert_eq!(ok(&mut vm, "set env(CHANGED)"), "new");
        assert_eq!(ok(&mut vm, "info exists env(ADDED)"), "1");
        assert_eq!(
            ok(&mut vm, "proc r {} {global env; return $env(ADDED)}; r"),
            "added"
        );
        // A read of an element the host lost fails, while `info exists`
        // swallows that error and so still answers for the stale element.
        assert_eq!(
            ok(
                &mut vm,
                "list [catch {set env(DROPPED)} m o] $m [dict get $o -errorcode] \
                 [info exists env(DROPPED)] [info exists env(NEVER)]"
            ),
            "1 {can't read \"env(DROPPED)\": no such variable} {TCL READ VARNAME} 1 0"
        );
        assert_eq!(
            ok(&mut vm, "catch {set env(DROPPED)}; set ::errorInfo"),
            "no such variable\n    (read trace on \"env(DROPPED)\")\n    \
             invoked from within\n\"set env(DROPPED)\""
        );
    }
}

#[test]
fn array_commands_resync_from_the_host() {
    for version in RELEASES {
        let env = vars(&[("KEEP", "k"), ("DROP", "d")]);
        let mut vm = vm_on(version, &env);
        ok(&mut vm, "set env(TCLONLY) 1");
        env.borrow_mut().remove("DROP");
        env.borrow_mut().remove("TCLONLY");
        env.borrow_mut().insert("KEEP".into(), "k2".into());
        env.borrow_mut().insert("FRESH".into(), "f".into());
        assert_eq!(
            ok(&mut vm, "lsort [array names env]"),
            "FRESH KEEP",
            "{version:?}"
        );
        assert_eq!(ok(&mut vm, "array size env"), "2");
        assert_eq!(ok(&mut vm, "array get env KEEP"), "KEEP k2");
        env.borrow_mut().insert("LATE".into(), "l".into());
        assert_eq!(ok(&mut vm, "array exists env"), "1");
        assert_eq!(ok(&mut vm, "info exists env(LATE)"), "1");
        // The re-sync writes nothing back and fires no script trace.
        ok(
            &mut vm,
            "set log {}; trace add variable env {write unset} {apply {args {lappend ::log $args}}}",
        );
        env.borrow_mut().remove("LATE");
        assert_eq!(ok(&mut vm, "array size env; set log"), "");
        assert_eq!(ok(&mut vm, "info exists env(LATE)"), "0");
    }
}

#[test]
fn a_whole_array_unset_leaves_the_host_alone_and_detaches_the_array() {
    for script in ["unset env", "array unset env"] {
        let env = vars(&[("HOME", "/root"), ("PATH", "/bin")]);
        let mut vm = vm_on(TclVersion::V9_0, &env);
        ok(&mut vm, script);
        assert_eq!(env.borrow().len(), 2, "{script}");
        ok(
            &mut vm,
            "set env(LATER) 1; set env(HOME) /changed; set env(PATH) /x; unset env(PATH)",
        );
        assert_eq!(host(&env, "LATER"), None, "{script}");
        assert_eq!(host(&env, "HOME").as_deref(), Some("/root"), "{script}");
        assert_eq!(host(&env, "PATH").as_deref(), Some("/bin"), "{script}");
    }
}

#[test]
fn the_hook_is_invisible_and_runs_between_array_and_element_script_traces() {
    for version in RELEASES {
        let env = vars(&[]);
        let mut vm = vm_on(version, &env);
        assert_eq!(ok(&mut vm, "trace info variable env"), "");
        ok(
            &mut vm,
            "trace add variable env write {apply {args {lappend ::seen [interp eval c {info exists env(FOO)}]}}}\n\
             trace add variable env(FOO) write {apply {args {lappend ::seen [interp eval c {info exists env(FOO)}]}}}\n\
             interp create c\n\
             set env(FOO) 1",
        );
        // As the array's oldest trace it runs after the script traces on
        // `env` and before the element's own (tclsh 8.6 and 9.0: `0 1`).
        assert_eq!(ok(&mut vm, "set seen"), "0 1");
        assert_eq!(
            ok(&mut vm, "trace info variable env"),
            "{write {apply {args {lappend ::seen [interp eval c {info exists env(FOO)}]}}}}"
        );
        ok(
            &mut vm,
            "trace remove variable env write {apply {args {lappend ::seen [interp eval c {info exists env(FOO)}]}}}\n\
             trace remove variable env(FOO) write {apply {args {lappend ::seen [interp eval c {info exists env(FOO)}]}}}\n\
             trace remove variable env {read write unset array} EnvTraceProc",
        );
        ok(&mut vm, "set env(FOO) 2");
        assert_eq!(host(&env, "FOO").as_deref(), Some("2"));
        // A write trace that rewrites the element cannot reach the host: the
        // element's own traces are suppressed while it runs.
        ok(
            &mut vm,
            "trace add variable env(UP) write {apply {args {set ::env(UP) [string toupper $::env(UP)]}}}",
        );
        assert_eq!(ok(&mut vm, "set env(UP) hello"), "HELLO");
        assert_eq!(host(&env, "UP").as_deref(), Some("hello"));
        assert_eq!(ok(&mut vm, "set env(UP)"), "hello");
    }
}

#[test]
fn child_interps_share_the_host_and_safe_interps_have_none() {
    let env = vars(&[]);
    let mut vm = vm_on(TclVersion::V9_0, &env);
    ok(
        &mut vm,
        "set env(PARENT) p\n\
         interp create c\n\
         c eval {set env(CHILD) c}\n\
         interp create -safe s\n\
         s eval {set env(SAFE) s}",
    );
    assert_eq!(ok(&mut vm, "c eval {set env(PARENT)}"), "p");
    assert_eq!(ok(&mut vm, "set env(CHILD)"), "c");
    assert_eq!(host(&env, "CHILD").as_deref(), Some("c"));
    assert_eq!(host(&env, "SAFE"), None);
    assert_eq!(ok(&mut vm, "s eval {array names env}"), "SAFE");
    ok(&mut vm, "unset env(PARENT)");
    assert_eq!(
        ok(
            &mut vm,
            "c eval {list [info exists env(PARENT)] [catch {set env(PARENT)}]}"
        ),
        "1 1"
    );
    assert_eq!(
        ok(
            &mut vm,
            "c eval {list [array names env PARENT] [info exists env(PARENT)]}"
        ),
        "{} 0"
    );
    // A child's whole-array unset detaches only the child's array.
    ok(&mut vm, "c eval {unset env; set env(DETACHED) 1}");
    assert_eq!(host(&env, "DETACHED"), None);
    ok(&mut vm, "set env(STILL) 1");
    assert_eq!(host(&env, "STILL").as_deref(), Some("1"));
}

#[test]
fn a_rebound_host_is_hooked_and_the_old_one_is_left_alone() {
    let first = vars(&[("FIRST", "1")]);
    let second = vars(&[("SECOND", "2")]);
    let mut vm = vm_on(TclVersion::V9_0, &first);
    ok(&mut vm, "set env(WRITTEN) 1");
    vm.set_host(Rc::new(MapHost {
        clock: MapClock,
        stdio: MapStdIo,
        env: MapEnv(Rc::clone(&second)),
    }));
    assert_eq!(ok(&mut vm, "lsort [array names env]"), "SECOND");
    ok(&mut vm, "set env(AFTER) 1; unset env(SECOND)");
    assert_eq!(
        first.borrow().keys().cloned().collect::<Vec<_>>(),
        ["FIRST", "WRITTEN"]
    );
    assert_eq!(
        second.borrow().keys().cloned().collect::<Vec<_>>(),
        ["AFTER"]
    );
}

#[test]
fn the_home_directory_follows_env_home() {
    let env = vars(&[("HOME", "/root")]);
    let mut vm = vm_on(TclVersion::V9_0, &env);
    assert_eq!(
        ok(
            &mut vm,
            "set env(HOME) /h; list [file home] [file tildeexpand ~/x]"
        ),
        "/h /h/x"
    );
    ok(&mut vm, "unset env(HOME)");
    assert_eq!(
        eval(&mut vm, "file home"),
        Err("couldn't find HOME environment variable to expand path".to_owned())
    );

    let env = vars(&[("HOME", "/root")]);
    let mut vm = vm_on(TclVersion::V8_6, &env);
    assert_eq!(ok(&mut vm, "set env(HOME) /h; file nativename ~/x"), "/h/x");
}

#[test]
fn subprocesses_see_env_writes() {
    if !std::path::Path::new("/bin/sh").exists() {
        return;
    }
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(
        eval(
            &mut vm,
            "set env(TCLVM_ENV_PROBE) bar\n\
             set seen [exec /bin/sh -c {echo ${TCLVM_ENV_PROBE-unset}}]\n\
             unset env(TCLVM_ENV_PROBE)\n\
             lappend seen [exec /bin/sh -c {echo ${TCLVM_ENV_PROBE-unset}}]"
        ),
        Ok("bar unset".to_owned())
    );
}
