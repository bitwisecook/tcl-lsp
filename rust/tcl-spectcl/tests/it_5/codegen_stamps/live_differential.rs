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

//! The VM's own compile of a caller against C Tcl: each shape of pack definition, spliced where the release admits it, answers what tclsh answers for the same script.

use super::*;
use tcl_dialect::TclVersion;

/// A pack of the shapes: `(name, arity, formal count, definition)`.
const SHAPES: &[(&str, &str, usize, &str)] = &[
    ("vthree", "0", 0, "proc vthree {} {expr {1 + 2}}"),
    ("vupper", "1", 1, "proc vupper {x} {string toupper $x}"),
    ("vdouble", "1", 1, "proc vdouble {x} {expr {$x * 2}}"),
    ("vapply", "2", 2, "proc vapply {cmd x} {$cmd $x}"),
    ("vdef", "1..2", 2, "proc vdef {x {y 10}} {expr {$x + $y}}"),
    ("vcount", "0..", 0, "proc vcount {args} {llength $args}"),
];

/// Each shape's caller, the calls made of it, and how many definitions the
/// VM's compile of the caller splices in where the release splices at all: as
/// many as the release-unit compile splices, which keeps a body that runs a
/// parameter as a command a call.
const CASES: &[(&str, &[&str], usize)] = &[
    ("proc caller {} {vthree}", &["caller"], 1),
    ("proc caller {n} {vupper $n}", &["caller abc"], 1),
    (
        "proc caller {n} {vdouble $n}",
        &["caller 21", "caller abc"],
        1,
    ),
    (
        "proc caller {c n} {vapply $c $n}",
        &["caller llength {a b c}", "caller nosuch x"],
        0,
    ),
    ("proc caller {n} {vdef $n}", &["caller 5", "caller x"], 1),
    (
        "proc caller {a b} {vcount $a $b}",
        &["caller 1 2", "caller {} {}"],
        1,
    ),
];

fn shapes_pack() -> String {
    use std::fmt::Write as _;

    let mut pack = String::from("speclib vendor 2.0 {\n");
    for (name, arity, formals, definition) in SHAPES {
        let _ = write!(pack, "    command {name} {{\n        arity {arity}\n");
        for index in 0..*formals {
            let _ = writeln!(pack, "        arg {index} -role Value");
        }
        let _ = write!(
            pack,
            "        runtime_backing tcl-body {{-pack-text {{{definition}}}}}\n    }}\n"
        );
    }
    pack.push_str("}\n");
    pack
}

/// `caller` and `call` run by a VM of `version` whose compile service inlines
/// the pack's bodies and which holds the pack's facts: the call's completion as
/// a list of its code and its result, which is what tclsh prints for the same.
fn run_inlined(set: &PackSet, version: TclVersion, caller: &str, call: &str) -> Outcome {
    let dialect = version.dialect_name();
    // Installs the packs' overlay for the release, which the service reads.
    let _registry = tcl_spectcl::bundled::registry_for_dialect_from(dialect, set);
    let profile = tcl_spectcl::environment::profile_for_dialect(dialect);
    let service = BytecodeCompileService::for_profile_with_overlay(profile, set.key)
        .expect("the packs' overlay is installed");
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    let (plain, spliced) = PlainCounting::installed_counting(&mut vm, service, None);
    vm.set_pack_facts(facts(set));
    for (_, _, _, definition) in SHAPES {
        let completion = vm.eval_source(definition).expect("compiles");
        assert_eq!(completion.code, Code::Ok, "{definition}");
    }
    let completion = vm.eval_source(caller).expect("compiles");
    assert_eq!(completion.code, Code::Ok, "{caller}");
    let completion = vm
        .eval_source(&format!("list [catch {{{call}}} r] $r"))
        .expect("compiles");
    Outcome {
        code: completion.code,
        answer: completion.result.to_str().to_string(),
        plain: plain.get(),
        spliced: spliced.get(),
    }
}

/// The same under tclsh.
fn run_tclsh(tclsh: &std::path::Path, caller: &str, call: &str) -> String {
    let definitions: Vec<&str> = SHAPES
        .iter()
        .map(|(_, _, _, definition)| *definition)
        .collect();
    let script = format!(
        "{}\n{caller}\nputs -nonewline [list [catch {{{call}}} r] $r]\n",
        definitions.join("\n")
    );
    let outcome = tcl_test_support::run_script(tclsh, script.as_bytes()).expect("tclsh runs");
    assert!(outcome.success(), "{script}");
    String::from_utf8(outcome.stdout).expect("tclsh prints text")
}

/// Every shape — no parameters, one, one an expression reads, one used as the
/// command word, a default and `args` — answers, result and error text alike,
/// what tclsh of each release answers. 8.4 enters a procedure body afresh at
/// each call and splices nothing; every later release splices each shape the
/// inliner proves safe.
#[test]
fn each_shape_spliced_by_the_live_compile_answers_as_tclsh_does() {
    let (set, _registry) = bundled_source("live-differential", &shapes_pack());
    for version in TclVersion::ALL {
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        for (caller, calls, spliced) in CASES {
            for call in *calls {
                let want = run_tclsh(&tclsh.path, caller, call);
                let got = run_inlined(&set, version, caller, call);
                let spliced = if version.revalidates_running_bytecode() {
                    *spliced
                } else {
                    0
                };
                assert_eq!(
                    got,
                    Outcome {
                        code: Code::Ok,
                        answer: want,
                        plain: 0,
                        spliced,
                    },
                    "{} {caller}: {call}",
                    version.version_string()
                );
            }
        }
    }
}
