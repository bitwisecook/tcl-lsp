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

//! Special-variable surface and startup lifecycle tests.

use tcl_dialect::model::Family;
use tcl_dialect::model::SurfaceQuery;

use super::*;

#[test]
fn startup_oracle_rosters_come_from_the_selected_registry() {
    for release in tcl_dialect::TclVersion::ALL {
        if let Some(shell) = tcl_test_support::witness_tclsh(release) {
            check_startup_oracle(&shell.path, &format!("tcl{}", release.version_string()));
        }
    }
    if let Some(jim) = tcl_test_support::locate_jimsh().unwrap()
        && jim.version == "0.84"
    {
        check_startup_oracle(&jim.path, "jimtcl");
    }
    if let Some(wish) = std::env::var_os("TCL_LSP_WISH") {
        check_startup_oracle(std::path::Path::new(&wish), "wish");
    }
}

#[test]
fn an_active_package_surface_does_not_imply_a_startup_binding() {
    let package = [tcl_dialect::model::PackageFloor::named("Tk")];
    let query = Some(d("tcl8.6").with_packages(&package));
    let tcl = crate::model::ingress::static_context_for("tcl8.6").commands();
    assert!(tcl.special_var_in_dialect("tk_version", query).is_some());
    assert!(!tcl.startup_existence("tk_version", query).readable());

    let wish = crate::model::ingress::static_context_for("wish");
    assert!(
        wish.commands()
            .startup_existence("tk_version", Some(wish.context().authoring_query()))
            .initially_bound
    );
}

fn check_startup_oracle(shell: &std::path::Path, dialect: &str) {
    let context = crate::model::ingress::static_context_for(dialect);
    let commands = context.commands();
    let query = Some(context.context().authoring_query());
    let jim = dialect == "jimtcl";
    let wish = dialect == "wish";
    let observed_at_startup = |spec: &SpecialVarSpec| !wish || spec.surface == SpecSurface::TK;
    let specs: Vec<_> = commands
        .special_vars_for_dialect(query)
        .filter(|spec| spec.kind != SpecialVarKind::Namespace && observed_at_startup(spec))
        .collect();
    let names: Vec<_> = specs.iter().map(|spec| spec.name).collect();
    let source = format!(
        "unset ::oracle_fixture_root\n\
         puts [lsort [info globals]]\n\
         foreach n {{{names}}} {{\n\
             puts [list $n [info exists ::$n] [array exists ::$n]]\n\
         }}\nexit\n",
        names = names.join(" "),
    );
    let output = tcl_test_support::run_script_fixture(shell, &source, &[])
        .unwrap()
        .strict_text()
        .unwrap();
    let mut lines = output.lines();
    let globals: Vec<_> = lines.next().unwrap().split_ascii_whitespace().collect();
    let conditional_jim = |spec: &SpecialVarSpec| {
        jim && spec.surface == JIM_SHELL && spec.startup_binding == StartupBinding::None
    };
    if !wish {
        let mut expected: Vec<_> = specs
            .iter()
            .filter(|spec| {
                commands.startup_existence(spec.name, query).initially_bound
                    || (conditional_jim(spec) && globals.contains(&spec.name))
            })
            .map(|spec| spec.name)
            .collect();
        expected.sort_unstable();
        assert_eq!(globals, expected, "{dialect} startup roster");
    }
    for (line, spec) in lines.zip(&specs) {
        let fact = commands.startup_existence(spec.name, query);
        if conditional_jim(spec) {
            continue;
        }
        let mut columns = line.split_ascii_whitespace();
        assert_eq!(columns.next(), Some(spec.name), "{dialect}");
        assert_eq!(
            columns.next(),
            Some(if fact.readable() { "1" } else { "0" }),
            "{dialect} {line}"
        );
        if !jim {
            let array = spec.kind == SpecialVarKind::Array && fact.readable();
            assert_eq!(
                columns.next(),
                Some(if array { "1" } else { "0" }),
                "{dialect} {line}"
            );
        }
    }
    assert_eq!(output.lines().count(), specs.len() + 1, "{dialect}");
}

/// The catalogue point the tests exercise these queries under — the
/// production callers resolve an environment once and thread the
/// profile's point instead.
fn d(dialect: &str) -> SurfaceQuery<'static> {
    // The seam's sink behaviour: a non-catalogue name (`tcl`, `tk`)
    // answers the permissive fallback's point, as the environment
    // ingress resolves it.
    tcl_dialect::DialectProfile::find(dialect)
        .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl)
        .surface_query()
}

/// The variable the number-spelling release fact names
/// (`NumberSyntax::precision_variable`) is the special variable this
/// registry records, in exactly the releases that have it.
#[test]
fn the_precision_variable_is_the_registered_tcl_precision() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let named = profile.grammar.numbers.precision_variable();
        let registered = special_var_in_dialect("tcl_precision", Some(d(dialect)));
        assert_eq!(named.is_some(), registered.is_some(), "{dialect}");
        if let (Some(named), Some(spec)) = (named, registered) {
            assert_eq!(named, spec.name);
            assert_eq!(spec.write_effect, Some(SideEffectTarget::InterpState));
        }
    }
}

#[test]
fn native_precision_hook_is_selected_by_engine_policy() {
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let dialect = crate::InvocationDialect::of_profile(profile);
        let expected = match version {
            tcl_dialect::TclVersion::V8_4
            | tcl_dialect::TclVersion::V8_5
            | tcl_dialect::TclVersion::V8_6 => Some("::tcl_precision"),
            tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1 => None,
        };
        assert_eq!(
            SpecialVariableHook::DoublePrecision.name_in(dialect),
            expected
        );
    }
    let jim = crate::model::ingress::resolve_environment("jim").analyser_profile();
    assert_eq!(
        SpecialVariableHook::DoublePrecision.name_in(crate::InvocationDialect::of_profile(jim)),
        None
    );
    assert!(SpecialVariableHook::DoublePrecision.changes_read_value());
    assert_eq!(
        special_var("tcl_precision").unwrap().runtime_hook,
        Some(SpecialVariableHook::DoublePrecision)
    );
}

#[test]
fn auto_path_is_externally_read_in_tcl() {
    // `set auto_path …` writes a runtime-observed
    // variable, so it must not be a dead store.
    assert!(is_externally_read("auto_path", Some(d("tcl8.6"))));
    assert!(is_externally_read("auto_path", Some(d("")))); // generic Tcl
    assert!(is_special_var("auto_path", Some(d("tcl8.6"))));
}

#[test]
fn plain_user_var_is_not_special() {
    assert!(!is_special_var("myVar", Some(d("tcl8.6"))));
    assert!(!is_externally_read("myVar", Some(d("tcl8.6"))));
    assert!(special_var("myVar").is_none());
}

#[test]
fn startup_readability_is_lifecycle_not_special_var_availability() {
    // This fixture is the audited default-host contract for an ordinary
    // Tcl script: it catches both an omitted startup global and an
    // accidental promotion of a lazy/configuration-only special var.
    // In particular, 8.4's successful default Tcl_Init runs init.tcl's
    // unconditional `set errorCode ""` / `set errorInfo ""`; 8.5+ does
    // not seed either name until error handling creates it.
    let common = [
        "argc",
        "argv",
        "argv0",
        "auto_path",
        "env",
        "tcl_interactive",
        "tcl_library",
        "tcl_patchLevel",
        "tcl_pkgPath",
        "tcl_platform",
        "tcl_rcFileName",
        "tcl_version",
    ];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut expected = common.to_vec();
        if dialect == "tcl8.4" {
            // 8.4's `Tcl_Init` also seeds `tcl_libPath` before sourcing
            // `init.tcl` (`tclUnixInit.c` / `tclWinInit.c`); 8.5's
            // `tclInterp.c` records the name as OBSOLETE and no release
            // since restores it.
            expected.extend(["errorCode", "errorInfo", "tcl_libPath"]);
        }
        if matches!(dialect, "tcl8.4" | "tcl8.5" | "tcl8.6") {
            expected.push("tcl_precision");
        }
        expected.sort_unstable();

        let mut actual: Vec<_> = SPECIAL_VARS
            .iter()
            .filter(|spec| spec.readable_at_startup_in(Some(d(dialect))))
            .map(|spec| spec.name)
            .collect();
        actual.sort_unstable();
        assert_eq!(actual, expected, "startup-readable vars for {dialect}");
    }

    // The embedded iRules runtime has a deliberately smaller proven
    // startup surface.  In particular, namespace availability for
    // `static::` does not promise a value for arbitrary static variables.
    let mut irules: Vec<_> = SPECIAL_VARS
        .iter()
        .filter(|spec| spec.readable_at_startup_in(Some(d("f5-irules"))))
        .map(|spec| spec.name)
        .collect();
    irules.sort_unstable();
    assert_eq!(irules, ["tcl_patchLevel", "tcl_platform", "tcl_version"]);

    assert_eq!(
        special_var("argv").unwrap().startup_binding,
        StartupBinding::TclMain
    );
    assert_eq!(
        special_var("tcl_precision").unwrap().startup_binding,
        StartupBinding::ReadTrace
    );
    assert!(is_readable_at_startup("tcl_precision", Some(d("tcl8.6"))));
    assert!(!is_readable_at_startup("tcl_precision", Some(d("tcl9.0"))));
    assert!(!is_initially_bound("tcl_precision", Some(d("tcl8.6"))));
    assert!(is_initially_bound("argv", Some(d("tcl8.6"))));
    assert!(is_lazily_readable("tcl_precision", Some(d("tcl8.6"))));
    assert!(!is_lazily_readable("tcl_precision", Some(d("tcl9.0"))));
    assert!(!is_lazily_readable("argv", Some(d("tcl8.6"))));
    // `tcl_libPath` exists only in 8.4 — availability, not just the
    // startup fact, ends there.
    assert!(is_special_var("tcl_libPath", Some(d("tcl8.4"))));
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(
            !is_special_var("tcl_libPath", Some(d(dialect))),
            "{dialect}"
        );
    }
    // iApps uses its host Tcl interpreter profile, whereas the embedded
    // iRules runtime has only the explicitly evidenced metadata globals.
    assert!(is_readable_at_startup("argv", Some(d("f5-iapps"))));
    assert!(!is_readable_at_startup("argv", Some(d("f5-irules"))));
    // `auto_index` can materialise after interactive activity or an
    // auto-loader operation, but an ordinary default-host script begins
    // with no array and a direct `$auto_index` / `$auto_index(key)` read
    // still errors. The other names below are likewise special metadata,
    // not default startup bindings.
    for name in [
        "auto_index",
        "auto_execs",
        "auto_noexec",
        "auto_noload",
        "static",
    ] {
        assert!(!is_readable_at_startup(name, Some(d("tcl8.6"))), "{name}");
    }
}

#[test]
fn irules_has_static_but_not_argv_or_env() {
    assert!(is_special_var("static", Some(d("f5-irules"))));
    assert!(!is_special_var("static", Some(d("tcl8.6"))));
    // Command-line / environment globals do not exist in the embedded
    // iRules interpreter.
    assert!(!is_special_var("argv", Some(d("f5-irules"))));
    assert!(!is_special_var("env", Some(d("f5-irules"))));
    assert!(!is_special_var("auto_path", Some(d("f5-irules"))));
}

#[test]
fn tcl_platform_available_in_both_tcl_and_irules() {
    assert!(is_special_var("tcl_platform", Some(d("tcl9.0"))));
    assert!(is_special_var("tcl_platform", Some(d("f5-irules"))));
    // Read-only platform info: a bare read is fine, but it is not the
    // "write is runtime-observed" class, so writes may still be flagged.
    assert!(!is_externally_read("tcl_platform", Some(d("tcl9.0"))));
    // CMP demotion flag is iRules-relevant.
    assert!(special_var("tcl_platform").unwrap().cmp_unsafe);
}

#[test]
fn tcl_platform_keys_are_dialect_versioned() {
    let spec = special_var("tcl_platform").unwrap();
    // `tmmVersion` is iRules-only.
    let irules_keys: Vec<_> = spec.keys_in(Some(d("f5-irules"))).map(|k| k.key).collect();
    assert!(irules_keys.contains(&"tmmVersion"));
    assert!(irules_keys.contains(&"os"));
    assert!(!irules_keys.contains(&"pointerSize")); // Tcl-only key

    // `pointerSize` arrived in Tcl 8.5, while `pathSeparator` arrived in
    // Tcl 8.6; build-only keys are deliberately not advertised by a
    // release-only profile.
    let keys_85: Vec<_> = spec.keys_in(Some(d("tcl8.5"))).map(|k| k.key).collect();
    let keys_86: Vec<_> = spec.keys_in(Some(d("tcl8.6"))).map(|k| k.key).collect();
    let keys_84: Vec<_> = spec.keys_in(Some(d("tcl8.4"))).map(|k| k.key).collect();
    assert!(keys_85.contains(&"pointerSize"));
    assert!(keys_86.contains(&"pointerSize"));
    assert!(!keys_84.contains(&"pointerSize"));
    assert!(!keys_85.contains(&"pathSeparator"));
    assert!(keys_86.contains(&"pathSeparator"));
    assert!(!keys_86.contains(&"threaded"));
    assert!(!keys_86.contains(&"debug"));
    assert!(!keys_86.contains(&"tmmVersion")); // iRules-only key absent in Tcl
}

#[test]
fn read_only_info_vars_are_not_dead_store_suppressed_by_external_read() {
    // These are readable interpreter values; writing then never reading
    // them is genuinely suspicious, so `externally_read` is false.
    for name in ["tcl_version", "tcl_patchLevel"] {
        assert!(is_special_var(name, Some(d("tcl8.6"))));
        assert!(!is_externally_read(name, Some(d("tcl8.6"))));
    }
}

#[test]
fn every_spec_name_is_unique() {
    let mut names: Vec<&str> = SPECIAL_VARS.iter().map(|v| v.name).collect();
    names.sort_unstable();
    let len = names.len();
    names.dedup();
    assert_eq!(names.len(), len, "duplicate special-variable name in table");
}

#[test]
fn dialect_iteration_excludes_out_of_dialect_vars() {
    let irules: Vec<&str> = special_vars_for_dialect(Some(d("f5-irules")))
        .map(|v| v.name)
        .collect();
    assert!(irules.contains(&"static"));
    assert!(irules.contains(&"tcl_platform"));
    assert!(!irules.contains(&"argv"));
}

#[test]
fn write_effects_are_recorded_for_interpreter_state_writes() {
    // Writing these mutates interpreter/runtime state, not just a variable.
    for name in ["auto_path", "tcl_precision", "env", "tcl_library"] {
        assert_eq!(
            special_var_write_effect(name, Some(d("tcl8.6"))),
            Some(SideEffectTarget::InterpState),
            "{name} should carry an InterpState write effect",
        );
    }
    // Read-only info globals and pure-data globals carry no write effect.
    for name in ["tcl_version", "tcl_platform", "argv", "argc"] {
        assert_eq!(
            special_var_write_effect(name, Some(d("tcl8.6"))),
            None,
            "{name}"
        );
    }
}

#[test]
fn read_taint_marks_external_input_variables() {
    for name in ["env", "argv", "argv0"] {
        assert_eq!(
            special_var_read_taint(name, Some(d("tcl8.6"))),
            Some(TaintColour::TAINTED),
            "{name} is attacker-influenced external input",
        );
    }
    // Interpreter-provided info is not attacker-controlled.
    for name in ["tcl_version", "tcl_platform", "auto_path"] {
        assert_eq!(
            special_var_read_taint(name, Some(d("tcl8.6"))),
            None,
            "{name}"
        );
    }
}

#[test]
fn tcl_derived_dialects_keep_the_standard_globals() {
    // Regression for the dialect-resolution fix: Tk / Expect / EDA shells
    // are Tcl supersets and must still recognise the standard interpreter
    // globals (they did before the registry existed).
    for dialect in ["tk", "expect", "synopsys-eda-tcl", "cadence-eda-tcl"] {
        assert!(
            is_special_var("env", Some(d(dialect))),
            "env must be a special var in {dialect}",
        );
        assert!(is_special_var("auto_path", Some(d(dialect))), "{dialect}");
        assert!(is_special_var("argv", Some(d(dialect))), "{dialect}");
        // The write-effect / taint queries resolve there too.
        assert_eq!(
            special_var_read_taint("env", Some(d(dialect))),
            Some(TaintColour::TAINTED),
            "{dialect}",
        );
    }
    // The restricted TMM sandbox (iRules) still does NOT get them.
    assert!(!is_special_var("env", Some(d("f5-irules"))));
    assert!(!is_special_var("argv", Some(d("f5-irules"))));
    assert!(!is_special_var("auto_path", Some(d("f5-irules"))));
    // iApps are NOT the TMM sandbox: they run a real Tcl 8.5.13 *host*
    // interpreter (dialect-profile-model.md §7, D3-adjacent ratification),
    // so the standard interpreter globals resolve there like any other
    // Tcl superset. The old bare-`IAPPS` view wrongly hid them.
    assert!(is_special_var("env", Some(d("f5-iapps"))));
    assert!(is_special_var("auto_path", Some(d("f5-iapps"))));
}

#[test]
fn version_dialects_keep_exact_bit_for_per_key_gating() {
    // A specific Tcl version must not be widened to ALL_TCL, or per-key
    // version gating (pointerSize is 8.5+) would leak into 8.4.
    assert_eq!(
        Some(d("tcl8.4")),
        Some(SurfaceQuery::core(Family::Tcl, "8.4"))
    );
    let spec = special_var("tcl_platform").unwrap();
    let keys_84: Vec<_> = spec.keys_in(Some(d("tcl8.4"))).map(|k| k.key).collect();
    assert!(!keys_84.contains(&"pointerSize"));
}
