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

//! The native trace that keeps the global `env` array in step with the host
//! environment — C's `EnvTraceProc` (`tclEnv.c`), which `TclSetupEnv` puts on
//! `env` in every interpreter it creates, children included, as one
//! array-level trace for reads, writes, unsets and `array` commands.
//!
//! - an element write sets the host variable to the element's new value;
//! - an element unset removes the host variable;
//! - an element read first refreshes the element from the host, and fails
//!   with `no such variable` when the host has none;
//! - an `array` command first re-syncs the whole array from the host;
//! - unsetting the whole array leaves the host alone, and the trace dies with
//!   the variable.
//!
//! The trace hangs off one binding of `::env` ([`TraceTable::env`]), fires in
//! the whole-array group of the ordinary trace walk as the oldest trace there,
//! and is invisible to `trace info` and `trace remove`. Its own stores into
//! the array are raw: they fire no traces and never write back.
//!
//! Not modelled: 9.x's `encoding error` for a value the system encoding cannot
//! represent, the element write traces C 8.6+ fires as a re-sync stores each
//! host variable, 8.4/8.5 deleting and re-creating the variable on every
//! re-sync, and a name containing `=` splitting at the first `=` when
//! re-synced.
//!
//! [`TraceTable::env`]: crate::cmd_trace::TraceTable::env

use std::collections::HashSet;

use tcl_runtime_api::guard::GuardDomain;

use crate::interp::{drop_fresh, new_string, obj_bytes, Interp};
use crate::namespace::GLOBAL;
use crate::vars::TraceHome;

/// The traced array's simple name in the global namespace.
const ENV: &[u8] = b"env";
/// The same array, spelled so it resolves from any frame.
const GLOBAL_ENV: &[u8] = b"::env";

/// Whether a variable spelling's simple tail is `env` — the only spellings
/// that can reach the global `env` array without going through a link.
fn tail_is_env(name: &[u8]) -> bool {
    let base = tcl_syntax::naming::split_element_ref_bytes(name).map_or(name, |(base, _)| base);
    let tail = base
        .iter()
        .rposition(|&byte| byte == b':')
        .map_or(base, |colon| &base[colon + 1..]);
    tail == ENV
}

/// Remove the intrinsic registration belonging to this destroyed root only.
pub(crate) fn detach_destroyed_env_trace(
    traces: &mut crate::cmd_trace::TraceTable,
    home: &TraceHome,
    element: Option<&[u8]>,
) {
    if element.is_none()
        && home.link_elem.is_none()
        && home.selected_member.is_none()
        && home.ns == Some(GLOBAL)
        && home.level.is_none()
        && home.base == ENV
        && traces.env.is_some()
        && traces.env == home.binding_id
    {
        traces.env = None;
    }
}

/// A Tcl string as the host environment takes it: the host sees a C string,
/// so a value stops at its first NUL, as `environ` does in C.
fn host_text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

impl Interp {
    /// Put the native trace on the global `env` array as it is bound now.
    pub(crate) fn arm_env_trace(&self) {
        let binding = self.namespaces().var_table(GLOBAL).binding_id(ENV);
        self.traces.borrow_mut().env = binding;
        // The per-slot trace answers cached by compiled code read the trace
        // set, which now includes this one.
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
    }

    /// Take the native trace off the global `env` array.
    pub(crate) fn disarm_env_trace(&self) {
        let armed = self.traces.borrow_mut().env.take().is_some();
        if armed {
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
        }
    }

    /// Whether the access resolved to `home` reaches the traced `env` array.
    pub(crate) fn env_trace_reaches(&self, home: &TraceHome) -> bool {
        let Some(binding) = self.traces.borrow().env else {
            return false;
        };
        home.ns == Some(GLOBAL)
            && home.level.is_none()
            && home.base == ENV
            && self.namespaces().var_table(GLOBAL).binding_id(ENV) == Some(binding)
    }

    /// Whether any variable trace could fire for an access spelt `name`: a
    /// script trace anywhere, or the `env` trace for a spelling that may
    /// resolve to the global `env` array. A `false` lets the access skip trace
    /// resolution altogether, as it does in an interpreter with no traces.
    pub(crate) fn traces_may_observe(&self, name: &[u8]) -> bool {
        let traces = self.traces.borrow();
        !traces.traces.is_empty()
            || (traces.env.is_some() && (traces.env_aliased || tail_is_env(name)))
    }

    /// Record a link from `local` to a variable named `target`. Once a link
    /// names `env` under another local name, any spelling may reach the global
    /// `env` array through a chain of links, so [`Self::traces_may_observe`]
    /// stops filtering on the spelling.
    pub(crate) fn note_variable_link(&self, local: &[u8], target: &[u8]) {
        if target != ENV || tail_is_env(local) || self.traces.borrow().env_aliased {
            return;
        }
        self.traces.borrow_mut().env_aliased = true;
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
    }

    /// Run the `env` trace for `op` on element `elem` (`None` for the array
    /// itself). Returns `true` when a read finds no such host variable, which
    /// fails the read as the trace's error.
    pub(crate) fn run_env_trace(&mut self, elem: Option<&[u8]>, op: &[u8]) -> bool {
        if op == b"array" {
            self.sync_env_array();
            return false;
        }
        // Unsetting the whole array leaves the host environment alone.
        let Some(elem) = elem else {
            return false;
        };
        let host = self.host();
        let name = host_text(elem);
        match op {
            b"write" => {
                if let Some(value) = self.env_element(elem) {
                    host.env().set(&name, &host_text(&value));
                }
            }
            b"unset" => host.env().unset(&name),
            _ => match host.env().get(&name) {
                Some(value) => self.store_env_element(elem, value.as_bytes()),
                None => return true,
            },
        }
        false
    }

    /// Re-sync the whole array from the host: every host variable stored, and
    /// every element the host lacks removed. Running inside the array's own
    /// trace, a removal fires the element's unset traces but not the
    /// array's, as C's `Tcl_UnsetVar2` from `TclSetupEnv` does.
    fn sync_env_array(&mut self) {
        let mut vars = self.host().env().vars();
        vars.sort_unstable();
        let live: HashSet<&[u8]> = vars.iter().map(|(name, _)| name.as_bytes()).collect();
        let stale: Vec<Vec<u8>> = self
            .array_names(GLOBAL_ENV)
            .unwrap_or_default()
            .into_iter()
            .filter(|elem| !live.contains(elem.as_slice()))
            .collect();
        for (name, value) in &vars {
            self.store_env_element(name.as_bytes(), value.as_bytes());
        }
        for elem in stale {
            self.var_unset_elem_reported(GLOBAL_ENV, Some(b"env"), &elem);
        }
    }

    /// The current value of element `elem`, read without traces.
    fn env_element(&self, elem: &[u8]) -> Option<Vec<u8>> {
        self.var_get_elem_at(GLOBAL_ENV, elem, 0).map(obj_bytes)
    }

    /// Store `value` into element `elem` without traces, leaving an element
    /// that already holds it untouched.
    fn store_env_element(&mut self, elem: &[u8], value: &[u8]) {
        if self.env_element(elem).as_deref() == Some(value) {
            return;
        }
        let object = new_string(value);
        if self.var_set_elem_at(GLOBAL_ENV, elem, object, 0).is_err() {
            drop_fresh(object);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use tcl_dialect::TclVersion;
    use tcl_platform::Env as _;

    use super::tail_is_env;
    use crate::counters;
    use crate::interp::{Code, Interp};

    #[derive(Default)]
    struct MapEnv(RefCell<BTreeMap<String, String>>);

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

    struct StoppedClock;

    impl tcl_platform::Clock for StoppedClock {
        fn now_secs(&self) -> i64 {
            0
        }

        fn now_millis(&self) -> i128 {
            0
        }
    }

    struct Silent;

    impl tcl_platform::StdIo for Silent {
        fn write_stdout(&self, _bytes: &[u8]) {}

        fn write_stderr(&self, _bytes: &[u8]) {}
    }

    /// A host whose environment is a map the test can inspect and change.
    struct MapHost {
        env: MapEnv,
        clock: StoppedClock,
        stdio: Silent,
    }

    impl MapHost {
        fn new(entries: &[(&str, &str)]) -> Rc<Self> {
            let env = MapEnv::default();
            for (key, value) in entries {
                env.set(key, value);
            }
            Rc::new(Self {
                env,
                clock: StoppedClock,
                stdio: Silent,
            })
        }

        fn var(&self, key: &str) -> Option<String> {
            self.env.get(key)
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

        fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
            static ENVIRONMENT: tcl_host_c_abi::NativeNumericEnvironment =
                tcl_host_c_abi::NativeNumericEnvironment;
            tcl_host_c_abi::NativeNumericEnvironment::supported()
                .then_some(&ENVIRONMENT as &dyn tcl_platform::NumericEnvironment)
        }
    }

    /// Run `body` against an interpreter over a fresh [`MapHost`] holding
    /// `HOME=/root` and `KEPT=k`, then require that nothing leaked.
    fn with_host(release: TclVersion, body: impl FnOnce(&mut Interp, &Rc<MapHost>)) {
        counters::reset();
        {
            let host = MapHost::new(&[("HOME", "/root"), ("KEPT", "k")]);
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(release.dialect_name()),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("the selected C release has an original native birth plan");
            body(&mut interp, &host);
        }
        assert_eq!(counters::finalize(), 0, "leaked objects");
        assert_eq!(counters::double_free_count(), 0);
    }

    fn ok(interp: &mut Interp, script: &str) -> String {
        let code = interp.eval_str(script.as_bytes());
        let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
        assert_eq!(code, Code::Ok, "{script} -> {result}");
        result
    }

    /// A name or value reaches the host cut at its first NUL, as C's
    /// `environ` keeps it.
    #[test]
    fn a_nul_cuts_the_name_and_value_the_host_keeps() {
        with_host(TclVersion::V9_0, |interp, host| {
            ok(interp, "set env(NUL\\0NAME) a\\0b");
            assert_eq!(host.var("NUL").as_deref(), Some("a"));
        });
    }

    /// tclsh 8.6.18 and 9.0.4: a re-sync that drops `env(GONE)` runs its
    /// own unset trace (not the array's) and leaves no trace behind.
    #[test]
    fn a_resync_fires_and_drops_a_removed_elements_unset_traces() {
        for release in [TclVersion::V8_6, TclVersion::V9_0] {
            with_host(release, |interp, host| {
                ok(
                    interp,
                    "set env(GONE) 1; set ::log {}\n\
                     trace add variable env unset {apply {args {lappend ::log arr}}}\n\
                     trace add variable env(GONE) unset {apply {args {lappend ::log elem:$args}}}",
                );
                host.env.unset("GONE");
                ok(interp, "array size env");
                assert_eq!(ok(interp, "set ::log"), "{elem:env GONE unset}");
                assert_eq!(ok(interp, "trace info variable env(GONE)"), "");
            });
        }
    }

    #[test]
    fn only_a_spelling_whose_tail_is_env_can_name_the_array_directly() {
        for name in [
            &b"env"[..],
            b"::env",
            b"env(HOME)",
            b"::env(a(b))",
            b"::x::env",
        ] {
            assert!(tail_is_env(name), "{}", String::from_utf8_lossy(name));
        }
        for name in [
            &b"e"[..],
            b"envy",
            b"env:",
            b"::env::",
            b"x(env)",
            b"environment(x)",
        ] {
            assert!(!tail_is_env(name), "{}", String::from_utf8_lossy(name));
        }
    }

    #[test]
    fn every_kind_of_element_write_and_unset_reaches_the_host() {
        with_host(TclVersion::V9_0, |interp, host| {
            ok(
                interp,
                "set env(SET) a; append env(SET) b; lappend env(LIST) x y; incr env(COUNT)\n\
                 array set env {PAIR1 1 PAIR2 2}; lassign {assigned} env(LASSIGN)\n\
                 catch {set unused caught} env(CAUGHT); set ::env(QUALIFIED) q\n\
                 dict set env(DICT) k v; set env(EMPTY) {}\n\
                 proc writer {} {\n\
                     global env; set env(GLOBAL) g\n\
                     upvar #0 env alias; set alias(ALIAS) u\n\
                     namespace eval ::inner {set ::env(NS) n}\n\
                 }\n\
                 writer",
            );
            for (key, value) in [
                ("SET", "ab"),
                ("LIST", "x y"),
                ("COUNT", "1"),
                ("PAIR1", "1"),
                ("PAIR2", "2"),
                ("LASSIGN", "assigned"),
                ("CAUGHT", "caught"),
                ("QUALIFIED", "q"),
                ("DICT", "k v"),
                ("EMPTY", ""),
                ("GLOBAL", "g"),
                ("ALIAS", "u"),
                ("NS", "n"),
            ] {
                assert_eq!(host.var(key).as_deref(), Some(value), "{key}");
            }
            ok(
                interp,
                "unset env(SET); array unset env PAIR*\n\
                 proc unsetter {} {upvar #0 env e; unset e(ALIAS)}; unsetter",
            );
            for key in ["SET", "PAIR1", "PAIR2", "ALIAS"] {
                assert_eq!(host.var(key), None, "{key}");
            }
            assert_eq!(host.var("LIST").as_deref(), Some("x y"));
            // Unsetting a missing element is still an error.
            assert_eq!(ok(interp, "catch {unset env(NOSUCH)}"), "1");
            // A value reaches the host as a C string, as `environ` takes it.
            assert_eq!(
                ok(interp, "set env(NUL) a\\0b; string length $env(NUL)"),
                "1"
            );
            assert_eq!(host.var("NUL").as_deref(), Some("a"));
        });
    }

    #[test]
    fn reads_refresh_from_the_host_and_fail_when_it_has_no_such_variable() {
        with_host(TclVersion::V9_0, |interp, host| {
            host.env.set("LATER", "1");
            assert_eq!(ok(interp, "info exists env(LATER)"), "1");
            assert_eq!(ok(interp, "set env(LATER)"), "1");
            host.env.set("LATER", "2");
            assert_eq!(ok(interp, "set env(LATER)"), "2");
            assert_eq!(
                ok(interp, "proc reader {} {return $::env(LATER)}; reader"),
                "2"
            );
            host.env.unset("LATER");
            assert_eq!(
                ok(
                    interp,
                    "list [catch {set env(LATER)} m o] $m [dict get $o -errorcode]"
                ),
                "1 {can't read \"env(LATER)\": no such variable} {TCL READ VARNAME}"
            );
            assert_eq!(
                ok(interp, "set ::errorInfo"),
                "no such variable\n    (read trace on \"env(LATER)\")\n    \
                 invoked from within\n\"set env(LATER)\""
            );
            // A cached element the host has lost still exists, as in C,
            // until an `array` command re-syncs the array.
            assert_eq!(ok(interp, "info exists env(LATER)"), "1");
            assert_eq!(ok(interp, "array names env LATER"), "");
            assert_eq!(ok(interp, "info exists env(LATER)"), "0");
            assert_eq!(ok(interp, "info exists env(NEVER)"), "0");
            host.env.set("ADDED", "a");
            host.env.unset("KEPT");
            assert_eq!(ok(interp, "lsort [array names env]"), "ADDED HOME");
            assert_eq!(ok(interp, "array get env ADDED"), "ADDED a");
            assert_eq!(ok(interp, "array size env"), "2");
            // `incr` and `append` read through the trace and treat a missing
            // host variable as unset.
            host.env.unset("ADDED");
            assert_eq!(ok(interp, "incr env(ADDED)"), "1");
            assert_eq!(host.var("ADDED").as_deref(), Some("1"));
        });
    }

    #[test]
    fn an_element_alias_reaches_the_host_only_from_9_0() {
        for (release, home) in [(TclVersion::V8_6, "/root"), (TclVersion::V9_0, "/alias")] {
            with_host(release, |interp, host| {
                ok(interp, "upvar 0 ::env(HOME) h; set h /alias");
                assert_eq!(host.var("HOME").as_deref(), Some(home), "{release:?}");
                assert_eq!(ok(interp, "set env(HOME)"), home, "{release:?}");
            });
        }
    }

    #[test]
    fn unsetting_the_whole_array_leaves_the_host_alone_and_detaches_it() {
        for detach in ["unset env", "array unset env", "upvar #0 env e; unset e"] {
            with_host(TclVersion::V9_0, |interp, host| {
                ok(interp, detach);
                assert_eq!(host.var("HOME").as_deref(), Some("/root"), "{detach}");
                assert_eq!(host.var("KEPT").as_deref(), Some("k"), "{detach}");
                ok(
                    interp,
                    "set env(AFTER) 1; set env(HOME) /after; unset env(HOME)",
                );
                assert_eq!(host.var("AFTER"), None, "{detach}");
                assert_eq!(host.var("HOME").as_deref(), Some("/root"), "{detach}");
                assert_eq!(ok(interp, "array names env"), "AFTER", "{detach}");
            });
        }
    }

    #[test]
    fn the_trace_is_invisible_and_runs_after_the_arrays_script_traces() {
        with_host(TclVersion::V9_0, |interp, host| {
            ok(
                interp,
                "interp create observer\n\
                 proc seen {kind args} {\n\
                     lappend ::seen $kind [observer eval {info exists env(FOO)}]\n\
                 }\n\
                 trace add variable env write {seen array}\n\
                 trace add variable env(FOO) write {seen element}\n\
                 set env(FOO) x",
            );
            assert_eq!(ok(interp, "set seen"), "array 0 element 1");
            assert_eq!(ok(interp, "llength [trace info variable env]"), "1");
            ok(
                interp,
                "trace remove variable env write {seen array}\n\
                 trace remove variable env(FOO) write {seen element}",
            );
            assert_eq!(ok(interp, "trace info variable env"), "");
            ok(interp, "set env(FOO) y");
            assert_eq!(host.var("FOO").as_deref(), Some("y"));
            // A read trace on the array runs before the host is consulted, so
            // it sees the failing read, as in C.
            ok(interp, "trace add variable env read {lappend ::reads}");
            assert_eq!(ok(interp, "catch {set env(MISSING)}"), "1");
            assert_eq!(ok(interp, "set reads"), "env MISSING read");
            // A script write trace's own rewrite is suppressed by the running
            // trace, so it never reaches the host, as in C.
            ok(
                interp,
                "trace add variable env(UP) write {apply {{a b op} {set ::env(UP) REWRITTEN}}}\n\
                 set env(UP) value",
            );
            assert_eq!(host.var("UP").as_deref(), Some("value"));
            assert_eq!(ok(interp, "set env(UP)"), "value");
        });
    }

    #[test]
    fn child_interpreters_share_the_host_and_safe_ones_have_no_env() {
        with_host(TclVersion::V9_0, |interp, host| {
            ok(
                interp,
                "interp create child; child eval {set env(FROM_CHILD) c}",
            );
            assert_eq!(host.var("FROM_CHILD").as_deref(), Some("c"));
            assert_eq!(ok(interp, "set env(FROM_CHILD)"), "c");
            ok(interp, "set env(FROM_PARENT) p");
            assert_eq!(ok(interp, "child eval {set env(FROM_PARENT)}"), "p");
            ok(interp, "unset env(FROM_PARENT)");
            assert_eq!(ok(interp, "child eval {info exists env(FROM_PARENT)}"), "1");
            assert_eq!(
                ok(
                    interp,
                    "child eval {list [catch {set env(FROM_PARENT)} m] $m}"
                ),
                "1 {can't read \"env(FROM_PARENT)\": no such variable}"
            );
            ok(interp, "interp create -safe sandbox");
            assert_eq!(ok(interp, "sandbox eval {info exists env}"), "0");
            ok(
                interp,
                "sandbox eval {set env(SAFE) s; set env(HOME) /safe}",
            );
            assert_eq!(host.var("SAFE"), None);
            assert_eq!(host.var("HOME").as_deref(), Some("/root"));
            ok(interp, "sandbox eval {unset env(HOME)}");
            assert_eq!(host.var("HOME").as_deref(), Some("/root"));
        });
    }

    #[test]
    fn a_confined_interpreter_neither_reads_nor_writes_the_host() {
        with_host(TclVersion::V9_0, |interp, host| {
            interp.confine_stores();
            assert_eq!(ok(interp, "info exists ::env(HOME)"), "0");
            assert_eq!(ok(interp, "catch {set ::env(HOME)}"), "1");
            assert_eq!(ok(interp, "info exists ::env"), "0");
            assert_eq!(host.var("HOME").as_deref(), Some("/root"));
            assert!(!interp.traces_may_observe(b"::env(HOME)"));
        });
    }

    #[test]
    fn rebinding_the_host_moves_the_trace_to_the_new_host() {
        with_host(TclVersion::V9_0, |interp, first| {
            let second = MapHost::new(&[("SECOND", "2")]);
            interp.set_host(second.clone());
            ok(interp, "set env(WRITTEN) w; unset env(SECOND)");
            assert_eq!(second.var("WRITTEN").as_deref(), Some("w"));
            assert_eq!(second.var("SECOND"), None);
            assert_eq!(first.var("WRITTEN"), None);
            assert_eq!(first.var("HOME").as_deref(), Some("/root"));
            // Rebinding after `unset env` traces the fresh array again.
            ok(interp, "unset env");
            interp.set_host(first.clone());
            ok(interp, "set env(BACK) b");
            assert_eq!(first.var("BACK").as_deref(), Some("b"));
        });
    }

    #[test]
    fn the_home_directory_follows_env_home() {
        with_host(TclVersion::V9_0, |interp, _| {
            ok(interp, "set env(HOME) /h");
            assert_eq!(ok(interp, "file home"), "/h");
            assert_eq!(ok(interp, "file tildeexpand ~/x"), "/h/x");
            ok(interp, "unset env(HOME)");
            assert_eq!(
                ok(interp, "list [catch {file home} m] $m"),
                "1 {couldn't find HOME environment variable to expand path}"
            );
        });
        with_host(TclVersion::V8_6, |interp, _| {
            ok(interp, "set env(HOME) /h");
            assert_eq!(ok(interp, "file nativename ~"), "/h");
            ok(interp, "unset env(HOME)");
            assert_eq!(
                ok(interp, "list [catch {file nativename ~} m] $m"),
                "1 {couldn't find HOME environment variable to expand path}"
            );
        });
    }

    #[test]
    fn info_exists_runs_script_read_traces_and_swallows_their_errors() {
        with_host(TclVersion::V9_0, |interp, _| {
            ok(
                interp,
                "set x 1; trace add variable x read {lappend ::log}\n\
                 trace add variable y read {apply {{a b op} {error boom}}}",
            );
            assert_eq!(ok(interp, "info exists x; set log"), "x {} read");
            assert_eq!(ok(interp, "info exists y"), "0");
            assert_eq!(ok(interp, "info exists ::errorInfo"), "0");
        });
    }

    #[test]
    fn variables_that_cannot_reach_env_keep_the_untraced_fast_path() {
        with_host(TclVersion::V9_0, |interp, _| {
            assert!(!interp.traces_may_observe(b"x"));
            assert!(!interp.traces_may_observe(b"a(env)"));
            assert!(interp.traces_may_observe(b"::env(HOME)"));
            assert!(interp.var_is_traced(b"env"));
            assert!(!interp.var_is_traced(b"x"));
            // A local named `env` is a different variable.
            assert_eq!(
                ok(
                    interp,
                    "proc local {} {set env(L) 1; info exists ::env(L)}; local"
                ),
                "0"
            );
            ok(interp, "upvar #0 env e");
            assert!(interp.traces_may_observe(b"x"));
            assert!(interp.var_is_traced(b"e"));
            assert!(!interp.var_is_traced(b"x"));
        });
    }
}
