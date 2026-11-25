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

//! What a fresh interpreter installs: math-function commands, Jim core roles, root cells, the platform schema and host globals.

use super::*;

struct SyntheticHost {
    clock: SyntheticClock,
    stdio: SyntheticStdIo,
    env: SyntheticEnv,
}

impl SyntheticHost {
    pub(super) fn new(entries: &[(&str, &str)]) -> Self {
        Self {
            clock: SyntheticClock,
            stdio: SyntheticStdIo,
            env: SyntheticEnv(RefCell::new(
                entries
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                    .collect(),
            )),
        }
    }
}

struct SyntheticClock;

impl tcl_platform::Clock for SyntheticClock {
    fn now_secs(&self) -> i64 {
        0
    }

    fn now_millis(&self) -> i128 {
        0
    }
}

struct SyntheticStdIo;

impl tcl_platform::StdIo for SyntheticStdIo {
    fn write_stdout(&self, _bytes: &[u8]) {}

    fn write_stderr(&self, _bytes: &[u8]) {}
}

struct SyntheticEnv(RefCell<std::collections::BTreeMap<String, String>>);

impl tcl_platform::Env for SyntheticEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) {
        self.0
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
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
        Ok("/synthetic".to_string())
    }

    fn chdir(&self, _path: &str) -> Result<(), tcl_platform::HostError> {
        Ok(())
    }
}

impl tcl_platform::Host for SyntheticHost {
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

#[cfg(have_tommath)]
#[test]
fn native_math_function_commands_follow_constructor_surface() {
    for (environment, expected) in [
        ("jim", false),
        ("tcl8.4", false),
        ("tcl8.5", true),
        ("tcl8.6", true),
        ("tcl9.0", true),
        ("tcl9.1", true),
    ] {
        let interpreter = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(environment),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert!(
            interpreter.native_execution_refusal().is_none(),
            "{environment}"
        );
        assert_eq!(
            interpreter
                .find_command_id(GLOBAL, b"::tcl::mathfunc::abs")
                .is_some(),
            expected,
            "{environment}",
        );
    }
    let generic = Interp::new();
    assert!(generic
        .find_command_id(GLOBAL, b"::tcl::mathfunc::abs")
        .is_some());
}

#[test]
fn native_jim_core_keeps_only_constructor_empty_roles() {
    let interp = Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let context = interp.native_jim_object_context().unwrap();
    let empty = context.empty_object();
    assert_eq!(interp.result_obj(), empty.as_ptr());
    // These are the actual empty, result, errorProc and top-frame namespace
    // roles in Jim_CreateInterp, before a distribution extension is loaded.
    assert_eq!(unsafe { (*empty.as_ptr()).ref_count }, 4);
    assert!(interp.find_command_id(GLOBAL, b"binary").is_none());
    assert!(interp.find_command_id(GLOBAL, b"set").is_some());
}

#[test]
fn native_core_root_cells_match_six_original_constructor_inventories() {
    use tcl_registry::special_vars::NativeBootstrapInputs;
    let fixture =
        include_str!("../../../../../rust/tcl-registry/tests/data/native_bootstrap/core-roots.tsv");
    for (profile_name, version) in [
        ("tcl8.4", "8.4"),
        ("tcl8.5", "8.5"),
        ("tcl8.6", "8.6"),
        ("tcl9.0", "9.0"),
        ("tcl9.1", "9.1"),
        ("jim", "jim"),
    ] {
        counters::reset();
        {
            let interp = Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect(profile_name),
                NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: Some(b"/native/build/library".to_vec()),
                },
            )
            .unwrap();
            assert_eq!(
                interp.find_command_id(GLOBAL, b"binary").is_some(),
                version != "jim",
                "{profile_name}"
            );
            assert_eq!(
                interp.find_command_id(GLOBAL, b"oo::class").is_some(),
                matches!(version, "8.6" | "9.0" | "9.1"),
                "{profile_name}"
            );
            assert_eq!(
                interp.find_command_id(GLOBAL, b"try").is_some(),
                matches!(version, "8.6" | "9.0" | "9.1" | "jim"),
                "{profile_name}"
            );
            assert_eq!(
                interp.find_command_id(GLOBAL, b"throw").is_some(),
                matches!(version, "8.6" | "9.0" | "9.1"),
                "{profile_name}"
            );
            assert!(
                interp.find_command_id(GLOBAL, b"auto_load").is_none(),
                "{profile_name}"
            );
            assert!(
                interp.find_command_id(GLOBAL, b"auto_import").is_none(),
                "{profile_name}"
            );
            let expected: Vec<_> = fixture
                .lines()
                .filter_map(|line| {
                    let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(line).collect();
                    (fields[0] == version && fields[2] == "1")
                        .then_some(fields[1].as_bytes().to_vec())
                })
                .collect();
            assert!(!expected.is_empty());
            let namespaces = interp.namespaces.borrow();
            let table = namespaces.var_table(GLOBAL);
            assert_eq!(
                table
                    .names()
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect::<Vec<_>>(),
                expected,
                "{profile_name}"
            );
            for line in fixture
                .lines()
                .filter(|line| line.starts_with(&format!("{version}\t")))
            {
                let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(line).collect();
                assert!(
                    table.has_native_namespace_cell(fields[1].as_bytes()),
                    "{profile_name}: {}",
                    fields[1]
                );
            }
            for name in [b"argv".as_slice(), b"argc", b"argv0", b"tcl_library"] {
                assert!(
                    !table.has_native_namespace_cell(name),
                    "{profile_name}: {name:?}"
                );
            }
        }
        assert_eq!(
            counters::finalize(),
            0,
            "{profile_name}: {} objects, {} buffers",
            counters::live_objs(),
            counters::live_bufs()
        );
    }
}

#[test]
fn fresh_interp_installs_the_shared_platform_schema_before_init() {
    leak_free(|i| {
        let mut expected = tcl_platform::bootstrap::entries()
            .iter()
            .map(|entry| entry.name())
            .collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(
            ok(i, b"lsort [array names ::tcl_platform]"),
            expected.join(" ").as_bytes()
        );
        assert_eq!(ok(i, b"set ::tcl_platform(osVersion)"), b"");
        assert!(!ok(i, b"set ::tcl_platform(machine)").is_empty());
        assert_eq!(
            ok(
                i,
                b"list [info exists ::env] [info exists ::argv] \
                      [info exists ::argv0] [info exists ::argc] \
                      [info exists ::auto_path] [info exists ::tcl_library]"
            ),
            b"1 1 1 1 1 1"
        );
    });
}

#[test]
fn selected_host_bootstrap_and_rebind_replace_all_host_globals() {
    counters::reset();
    {
        let first = Rc::new(SyntheticHost::new(&[
            ("USER", "first-user"),
            ("TCL_LIBRARY", "/first/lib"),
            ("TCL_WASM_SPEC", "first-wasm"),
            ("FIRST_ONLY", "stale"),
        ]));
        let mut interp = Interp::with_host(first);
        assert_eq!(
            interp.host().capabilities(),
            tcl_platform::Capabilities::empty()
        );
        assert_eq!(
            ok(
                &mut interp,
                b"list $::tcl_platform(user) $::tcl_platform(wasm) \
                      $::tcl_library $::env(FIRST_ONLY)"
            ),
            b"first-user first-wasm /first/lib stale"
        );
        ok(
            &mut interp,
            b"set ::env(EMBEDDER_STALE) old; \
                  set ::tcl_platform(user) old; \
                  set ::auto_path /old/auto; \
                  set ::tclDefaultLibrary /old/default; \
                  set ::tcl_pkgPath /old/pkg",
        );

        interp.set_host(Rc::new(SyntheticHost::new(&[
            ("USER", "second-user"),
            ("TCL_LIBRARY", "/second/lib"),
            ("TCL_WASM_SPEC", "second-wasm"),
            ("SECOND_ONLY", "fresh"),
        ])));
        assert_eq!(
            ok(
                &mut interp,
                b"list $::tcl_platform(user) $::tcl_platform(wasm) \
                      $::tcl_library $::env(SECOND_ONLY) \
                      [info exists ::env(FIRST_ONLY)] \
                      [info exists ::env(EMBEDDER_STALE)] \
                      [info exists ::tclDefaultLibrary] \
                      [info exists ::tcl_pkgPath] [llength $::auto_path]"
            ),
            b"second-user second-wasm /second/lib fresh 0 0 0 0 0"
        );
        assert_eq!(
            ok(
                &mut interp,
                b"interp create child; child eval {list $::tcl_platform(user) \
                      $::tcl_library $::env(SECOND_ONLY)}"
            ),
            b"second-user /second/lib fresh"
        );
    }
    assert_eq!(counters::finalize(), 0);
    assert_eq!(counters::double_free_count(), 0);
}

#[test]
fn child_and_safe_platform_schemas_come_from_the_shared_owner() {
    leak_free(|i| {
        let mut child_keys = tcl_platform::bootstrap::entries()
            .iter()
            .map(|entry| entry.name())
            .collect::<Vec<_>>();
        child_keys.sort_unstable();
        assert_eq!(
            ok(
                i,
                b"interp create child; child eval {lsort [array names ::tcl_platform]}"
            ),
            child_keys.join(" ").as_bytes()
        );

        let scrubbed = tcl_platform::bootstrap::safe_scrub_keys().collect::<Vec<_>>();
        let mut safe_keys = tcl_platform::bootstrap::entries()
            .iter()
            .map(|entry| entry.name())
            .filter(|name| !scrubbed.contains(name))
            .collect::<Vec<_>>();
        safe_keys.sort_unstable();
        assert_eq!(
            ok(
                i,
                b"interp create -safe safe; safe eval {lsort [array names ::tcl_platform]}"
            ),
            safe_keys.join(" ").as_bytes()
        );
        assert!(safe_keys.contains(&"threaded"));
        assert_eq!(
            ok(
                i,
                b"safe eval {list [info exists ::env] \
                      [info exists ::tcl_library] [info exists ::auto_path] \
                      [info exists ::tclDefaultLibrary] [info exists ::tcl_pkgPath]}"
            ),
            b"0 0 0 0 0"
        );
    });
}
