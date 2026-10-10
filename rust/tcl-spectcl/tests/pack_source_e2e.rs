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

//! From `.tclspec` source to a folded call site, through the real loader and
//! the real install path.
//!
//! `tcl-spec-hooks/tests/const_fold_e2e.rs` proves the host and the optimiser
//! agree; this proves the *seam* between the loader and the host — that a hook
//! written in a pack file as a `const_fold {words ctx} { … }` statement,
//! carried by the loader as a `HookDecl`, becomes the function pointer the
//! installed `CommandSpec` holds.
//!
//! The adapter is no longer written out here: it is
//! [`tcl_spectcl::hooks`](tcl_spectcl::hooks), which the workspace install path
//! runs for real. What this file still owns is the claim that the two halves
//! line up end to end.

use std::path::PathBuf;
use std::rc::Rc;

use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::optimiser::manager::optimise_raw;
use tcl_registry::CommandRegistry;
use tcl_registry::hover::ScriptTiming;
use tcl_registry::pack_hooks::{self, HookInputs};
use tcl_spec_hooks::tclvm_host;
use tcl_spectcl::discovery::{Origin, PackFile, Tier};
use tcl_spectcl::hooks;
use tcl_spectcl::loader::{self, HookSource};
use tcl_spectcl::pack::PackSet;

/// The body shared by the legacy and 2.0 spellings of the live-hook pack.
/// Keeping one body makes the DSL release the only axis in the end-to-end
/// comparison.
const FOLD_SOURCE_BODY: &str = r"
    command mylib::strlen {
        arity 1
        arg 0 -role Value
        const_fold -inputs {words} {words ctx} {
            set subject [lindex $words 0]
            if {![string is ascii $subject]} { return }
            fold [string length $subject]
        }
    }
";

fn fold_source(dsl_version: &str) -> String {
    [
        "\nspeclib mylib ",
        dsl_version,
        " {\n",
        FOLD_SOURCE_BODY,
        "}\n",
    ]
    .concat()
}

/// A command-prefix position whose timing depends on another argument. This
/// exercises the new family through the loader, binder, host, thunk and the
/// registry's exact current-position query rather than testing any seam in
/// isolation.
const TIMING_SOURCE: &str = r#"
speclib mylib 1.2 {
    command mylib::callback {
        arity 2
        arg 1 -appends {Exactly 0}
        script_timing_resolver {words ctx} {
            if {[lindex $words 0] eq "later"} {
                timing 1 Deferred
            } else {
                timing 1 SameInvocation
            }
        }
    }
}
"#;

/// The same source as a discovered, merged pack set — the shape the install
/// path takes.
fn pack_set_from(name: &str, source: &str) -> PackSet {
    let dir = std::env::temp_dir().join(format!(
        "tcl-spectcl-pack-source-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path: PathBuf = dir.join("mylib.tclspec");
    std::fs::write(&path, source).expect("write pack");
    tcl_spectcl::pack::load(&[PackFile {
        tier: Tier::Workspace,
        path,
        origin: Origin::DotDir,
        dependency_tier: None,
    }])
}

fn pack_set(name: &str) -> PackSet {
    pack_set_from(name, &fold_source("1"))
}

fn folds(registry: &CommandRegistry) -> Vec<String> {
    optimise_raw(
        "proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n",
        registry,
        None,
    )
    .into_iter()
    .filter(|optimisation| optimisation.code.as_str() == "O129")
    .map(|optimisation| optimisation.replacement)
    .collect()
}

#[test]
fn the_loader_carries_the_declared_inputs() {
    let source = fold_source("1");
    let pack = loader::evaluate_pack(&source);
    let command = pack.command("mylib::strlen").expect("the pack declares it");
    let [hook] = &command.hooks[..] else {
        panic!("one hook, got {:?}", command.hooks.len());
    };
    let HookSource::Body { params, inputs, .. } = &hook.source else {
        panic!("a Tcl body");
    };
    assert_eq!(params, &["words".to_owned(), "ctx".to_owned()]);
    assert_eq!(inputs, &HookInputs::parse(&["words"]));
    // `words` is the one input that makes a hook uncacheable.
    assert!(!inputs.shape_only());
}

#[test]
fn a_pack_file_folds_a_call_site_in_the_optimiser() {
    let mut results = Vec::new();
    for dsl_version in ["1", "2.0"] {
        let source = fold_source(dsl_version);
        let packs = pack_set_from(&format!("folds-{dsl_version}"), &source);
        assert!(
            packs.notices.is_empty(),
            "DSL {dsl_version} pack loads cleanly: {:?}",
            packs.notices
        );
        assert_eq!(
            packs.packs.first().map(|pack| pack.dsl_version.as_str()),
            Some(dsl_version),
            "the loader preserves the DSL release presented to the hook adapter"
        );

        // The production adapter, and the production slot assignment.
        let plan = hooks::plan_for(&packs);
        assert!(!plan.is_empty(), "the pack declares one hook body");

        let host = Rc::new(tclvm_host());
        for programs in plan.packs() {
            let installed = host.install_pack_hooks(programs.clone());
            for (installation, program) in installed.iter().zip(&programs.programs) {
                assert_eq!(
                    installation.slot, program.slot,
                    "DSL {dsl_version}: the host binds the slot the plan assigned, never a fresh one: {:?}",
                    installation.declined
                );
            }
        }
        pack_hooks::install_host(host);

        // The registry the workspace would install — same call the LSP makes.
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
        let result = folds(&registry);
        assert_eq!(result, vec!["5".to_string()], "DSL {dsl_version}");
        results.push(result);
        pack_hooks::clear_host();
    }
    assert_eq!(
        results[0], results[1],
        "legacy and 2.0 hook programs have identical live semantics"
    );
}

#[test]
fn without_a_host_the_installed_pack_simply_does_not_fold() {
    // The thunk is in place, but a thread with no host installed gets the
    // family's silence — the same answer the loader's abstaining placeholder
    // gave before hooks could run.
    let packs = pack_set("hostless");
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    pack_hooks::clear_host();
    assert!(folds(&registry).is_empty());
}

#[test]
fn a_pack_timing_hook_controls_a_live_command_prefix_position() {
    let packs = pack_set_from("timing", TIMING_SOURCE);
    assert!(
        packs.notices.is_empty(),
        "the timing pack loads cleanly: {:?}",
        packs.notices
    );

    let plan = hooks::plan_for(&packs);
    assert_eq!(
        plan.packs()
            .iter()
            .map(|programs| programs.programs.len())
            .sum::<usize>(),
        1,
        "the pack declares one timing hook"
    );
    let host = Rc::new(tclvm_host());
    for programs in plan.packs() {
        let installed = host.install_pack_hooks(programs.clone());
        assert!(
            installed.iter().all(|entry| entry.declined.is_none()),
            "the timing hook installs: {installed:?}"
        );
    }
    pack_hooks::install_host(host);

    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    assert_eq!(
        registry.script_timing("mylib::callback", &["now", "callback"], 1, None,),
        Some(ScriptTiming::SameInvocation),
    );
    assert_eq!(
        registry.script_timing("mylib::callback", &["later", "callback"], 1, None,),
        Some(ScriptTiming::Deferred),
    );
    pack_hooks::clear_host();
}

/// The pack that backs `vendor::f` with `definition` as a reference body.
fn reference_source(definition: &str, arity: usize) -> String {
    format!(
        "\nspeclib vendor 2.0 {{\n    command vendor::f {{\n        arity {arity}\n        \
         runtime_backing tcl-body {{-pack-text {{{definition}}} -evaluate}}\n    }}\n}}\n"
    )
}

/// What the analyser proves `known` to be in a procedure that sets it to
/// `call`, with the host the load's plan binds running the reference body of
/// `vendor::f` for `definition`: the constant's text, or `None` when it proves
/// none.
fn proved(definition: &str, arity: usize, call: &str) -> Option<String> {
    proved_under("tcl9.0", definition, arity, call)
}

/// The same, with the call analysed under `dialect`.
fn proved_under(dialect: &str, definition: &str, arity: usize, call: &str) -> Option<String> {
    let packs = pack_set_from("reference-body", &reference_source(definition, arity));
    let plan = hooks::plan_for(&packs);
    let host = Rc::new(tclvm_host());
    for programs in plan.packs() {
        let installed = host.install_pack_hooks(programs.clone());
        assert!(
            installed.iter().all(|entry| entry.declined.is_none()),
            "the derived body installs: {installed:?}"
        );
    }
    pack_hooks::install_host(host);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
    let source = format!("proc ::p {{}} {{\n    set known [{call}]\n    return $known\n}}\n");
    let unit = CompilationUnit::build_for_dialect(&source, &registry, false, dialect);
    pack_hooks::clear_host();
    let function = unit.procedures.get("::p")?;
    let symbol = function.ssa.var_symbol("known")?;
    match function.sccp.values.get(&(symbol, 1)) {
        Some(LatticeValue::Const(value)) => Some(match value {
            ConstValue::Int(number) => number.to_string(),
            ConstValue::Float(number) => number.to_string(),
            ConstValue::Bool(flag) => u8::from(*flag).to_string(),
            ConstValue::String(text) => text.clone(),
        }),
        _ => None,
    }
}

/// What the VM answers when `definition` is the procedure and `call` is run.
fn executed(definition: &str, call: &str) -> String {
    let mut vm = tcl_vm::Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    vm.eval_source("namespace eval vendor {}")
        .expect("the namespace the definition lives in");
    let defined = vm.eval_source(definition).expect("the definition compiles");
    assert_eq!(
        defined.code,
        tcl_vm::Code::Ok,
        "{}",
        defined.result.to_str()
    );
    let completion = vm.eval_source(call).expect("the call compiles");
    assert_eq!(
        completion.code,
        tcl_vm::Code::Ok,
        "{}",
        completion.result.to_str()
    );
    completion.result.to_str().to_string()
}

/// A reference body the sandbox can express is run on the real host as the
/// declared implementation of its command, from the `.tclspec` source to a
/// constant the analyser proves — and it answers what the procedure itself does
/// when the VM runs it, for the shapes a body takes: an expression, a final
/// `return` of a variable and of a substitution, branches, loops, `switch`, a
/// dict, and the empty `return`. The negatives: an argument the analysis does
/// not know proves nothing, and a body that reaches for the frame is never run.
#[test]
fn a_reference_body_answers_through_the_host_as_the_procedure_does() {
    let cases: [(&str, usize, &str); 12] = [
        ("proc vendor::f {x} {expr {$x * 2}}", 1, "vendor::f 21"),
        (
            "proc vendor::f {a b} {\n    set s [string cat $a - $b]\n    return $s\n}",
            2,
            "vendor::f x y",
        ),
        (
            "proc vendor::f {x} {\n    if {$x > 10} {\n        set r big\n    } else {\n        set r small\n    }\n    return $r\n}",
            1,
            "vendor::f 3",
        ),
        (
            "proc vendor::f {x} {\n    if {$x > 10} {\n        set r big\n    } else {\n        set r small\n    }\n    return $r\n}",
            1,
            "vendor::f 30",
        ),
        (
            "proc vendor::f {l} {\n    set n 0\n    foreach i $l {incr n $i}\n    return $n\n}",
            1,
            "vendor::f {1 2 3}",
        ),
        ("proc vendor::f {x} {return}", 1, "vendor::f anything"),
        (
            "proc vendor::f {s} {string toupper [string trim $s]}",
            1,
            "vendor::f {  ab }",
        ),
        (
            "proc vendor::f {x} {\n    set y [expr {$x + 1}]\n    return [expr {$y * $y}]\n}",
            1,
            "vendor::f 2",
        ),
        (
            "proc vendor::f {d} {dict get $d k}",
            1,
            "vendor::f {j u k v}",
        ),
        (
            "proc vendor::f {n} {\n    set a 0\n    set b 1\n    while {$n > 0} {\n        lassign [list $b [expr {$a + $b}]] a b\n        incr n -1\n    }\n    return $a\n}",
            1,
            "vendor::f 10",
        ),
        (
            "proc vendor::f {x} {\n    switch -- $x {\n        a {set r 1}\n        b {set r 2}\n        default {set r 0}\n    }\n    return $r\n}",
            1,
            "vendor::f b",
        ),
        (
            "proc vendor::f {x} {\n    set out {}\n    for {set i 0} {$i < 3} {incr i} {lappend out [expr {$i * $x}]}\n    return $out\n}",
            1,
            "vendor::f 4",
        ),
    ];
    for (definition, arity, call) in cases {
        let ran = executed(definition, call);
        assert_eq!(
            proved(definition, arity, call).as_deref(),
            Some(ran.as_str()),
            "{definition}\n{call}"
        );
    }
    assert_eq!(
        proved(
            "proc vendor::f {x} {expr {$x * 2}}",
            1,
            "vendor::f $::unknown"
        ),
        None,
        "an argument the analysis does not know is not folded"
    );
    assert_eq!(
        proved(
            "proc vendor::f {x} {upvar 1 $x y; expr {$y * 2}}",
            1,
            "vendor::f 21"
        ),
        None,
        "a body that reaches for the frame is not run"
    );
}

/// What `release`'s own `tclsh` did when `definition` was the procedure and
/// `call` was run.
#[derive(Debug, PartialEq, Eq)]
enum Shell {
    /// No shell of that release on `PATH`, which skips the row.
    Absent,
    /// The shell raised.
    Raised,
    /// The shell answered the value.
    Answered(String),
}

impl Shell {
    /// The answer, or `None` for a shell that raised.
    fn answer(self) -> Option<String> {
        match self {
            Self::Answered(value) => Some(value),
            Self::Absent | Self::Raised => None,
        }
    }
}

/// Run `call` after `definition` in `release`'s own `tclsh`.
fn real_shell(release: &str, definition: &str, call: &str) -> Shell {
    let version = tcl_dialect::TclVersion::from_version_string(release).expect("reference release");
    let Some(shell) = tcl_test_support::witness_tclsh(version) else {
        return Shell::Absent;
    };
    let script = format!(
        "namespace eval vendor {{}}\n{definition}\n\
         if {{[catch {{{call}}} answer]}} {{puts -nonewline ERR}} else {{puts -nonewline OK:$answer}}\n"
    );
    let output = tcl_test_support::run_script(&shell.path, script.as_bytes())
        .expect("reference interpreter runs")
        .strict_text()
        .expect("reference script reports its completion");
    output
        .strip_prefix("OK:")
        .map_or(Shell::Raised, |value| Shell::Answered(value.to_owned()))
}

/// The body runs under the release the call is analysed under: `string cat` is
/// 8.6's, so a body that calls it answers under 8.6 and later and abstains under
/// 8.4 and 8.5, and a leading zero is octal up to 8.6 and decimal from 9.0, so one
/// body answers differently under 8.6 and 9.0.
///
/// The derivation is the author's to ask for because the engine under the host
/// emulates an older release imperfectly (issue #2333: `string is integer`'s width,
/// `tcl_precision` under 8.4, `incr` of an unset local, `lreplace` and `lindex`
/// bounds and index forms, `1.0/0`, `int(1e20)` and `1<<64` under 8.4, `format
/// %c`), so the rows below are those that hold, and each is held to the real shell
/// of the release it is analysed under: the analysis answers what the shell does,
/// or abstains where the shell raises. A release with no shell on `PATH` skips its
/// rows.
#[test]
fn a_derived_body_runs_under_the_release_the_call_is_analysed_under() {
    const ALL: &[&str] = &["8.4", "8.5", "8.6", "9.0", "9.1"];

    let cat = "proc vendor::f {a b} {string cat $a $b}";
    for (dialect, expected) in [
        ("tcl8.4", None),
        ("tcl8.5", None),
        ("tcl8.6", Some("xy")),
        ("tcl9.0", Some("xy")),
        ("tcl9.1", Some("xy")),
    ] {
        assert_eq!(
            proved_under(dialect, cat, 2, "vendor::f x y").as_deref(),
            expected,
            "{dialect}"
        );
    }
    let leading_zero = "proc vendor::f {x} {expr {$x + 010}}";
    for (dialect, expected) in [("tcl8.6", "8"), ("tcl9.0", "10")] {
        assert_eq!(
            proved_under(dialect, leading_zero, 1, "vendor::f 0").as_deref(),
            Some(expected),
            "{dialect}"
        );
    }

    // What the real shell of each release answers.
    let rows: &[(&str, &str, usize, &str, &[&str])] = &[
        (
            "string cat",
            "proc vendor::f {a b} {string cat $a $b}",
            2,
            "vendor::f x y",
            ALL,
        ),
        (
            "octal",
            "proc vendor::f {x} {expr {$x + 010}}",
            1,
            "vendor::f 0",
            ALL,
        ),
        (
            "digit separator",
            "proc vendor::f {x} {expr {$x + 1_000}}",
            1,
            "vendor::f 0",
            ALL,
        ),
        (
            "format %x",
            "proc vendor::f {x} {format %x $x}",
            1,
            "vendor::f -1",
            ALL,
        ),
        (
            "an astral character's length",
            "proc vendor::f {x} {string length \"\\U1F600$x\"}",
            1,
            "vendor::f a",
            ALL,
        ),
        (
            "a float past a word",
            "proc vendor::f {x} {expr {int($x)}}",
            1,
            "vendor::f 1e20",
            &["9.0", "9.1"],
        ),
    ];
    for (row, definition, arity, call, releases) in rows {
        for release in *releases {
            let shell = real_shell(release, definition, call);
            if shell == Shell::Absent {
                eprintln!("skipped: no tclsh{release} on PATH ({row})");
                continue;
            }
            let shell = shell.answer();
            let analysed = proved_under(&format!("tcl{release}"), definition, *arity, call);
            assert_eq!(
                analysed, shell,
                "{row} under {release}: the analysis against the shell's own answer\n{definition}\n{call}"
            );
        }
    }
}
