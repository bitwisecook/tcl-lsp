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

//! `info`: commands, procedures, variables, defaults, `info exists` and `info complete`, and procedure bodies built at run time.

use super::*;

#[test]
fn info_introspection() {
    out_eq("puts [info exists nope]\n", "0\n");
    out_eq("set x 1\nputs [info exists x]\n", "1\n");
    out_eq(
        "proc f {a {b 5}} { return $a }\nputs [info args f]\n",
        "a b\n",
    );
    out_eq(
        "proc f {a {b 5}} { return $a }\nputs [info body f]\n",
        " return $a \n",
    );
    // `info exists` checks variables, so a proc name is 0; the proc is listed
    // by `info procs` (lsearch finds it at index 0 as the only user proc).
    out_eq("proc f {} {}\nputs [info exists f]\n", "0\n");
    out_eq("proc f {} {}\nputs [lsearch [info procs] f]\n", "0\n");
    out_eq("puts [info level]\n", "0\n");
    out_eq("proc g {} { return [info level] }\nputs [g]\n", "1\n");
    out_eq("puts [info tclversion]\n", "9.0\n");
    // body/args/default on a non-proc (unknown or a builtin) → the shared
    // `"name" isn't a procedure` error (pinned against tclsh 9.0).
    let (ok, msg, _) = run("info body nosuch");
    assert!(!ok);
    assert_eq!(msg, "\"nosuch\" isn't a procedure");
    let (ok, msg, _) = run("info args nosuch");
    assert!(!ok);
    assert_eq!(msg, "\"nosuch\" isn't a procedure");
    let (ok, msg, _) = run("info body set");
    assert!(!ok);
    assert_eq!(msg, "\"set\" isn't a procedure");
}

#[test]
fn dynamic_proc_body() {
    // A proc whose name and/or body is built at runtime is compiled on demand.
    out_eq(
        "set n foo\nproc $n {x} { return [expr {$x*2}] }\nputs [foo 21]\n",
        "42\n",
    );
    out_eq("set b {return hi}\nproc dyn {} $b\nputs [dyn]\n", "hi\n");
}

#[test]
fn info_complete() {
    out_eq("puts [info complete {set x 1}]\n", "1\n");
    // `{[a}` is a braced word → value `[a` (unbalanced bracket) → incomplete.
    out_eq("puts [info complete {[a}]\n", "0\n");
    out_eq("puts [info complete {puts hello}]\n", "1\n");
}

#[test]
fn info_vars_locals_globals() {
    // Variable listing routes through the shared cores. Pinned against tclsh 9.0
    // (C `InfoVarsCmd`/`InfoLocalsCmd`/`InfoGlobalsCmd`).
    let setup = "namespace eval foo { variable a 1; variable b 2 }\n\
                 namespace eval foo::sub { variable deep 9 }\nset gx 10\n";
    // In a proc: `info locals` is the genuine locals only; `info vars` also lists
    // the linked namespace variable (by its local alias `a`).
    out_eq(
        &format!(
            "{setup}proc p {{}} {{ set loc 5; variable ::foo::a; \
             puts \"[lsort [info locals]]|[lsort [info vars]]\" }}\np\n"
        ),
        "loc|a loc\n",
    );
    // `info globals` lists the global namespace's vars only — not `foo::a`.
    out_eq(
        &format!("{setup}puts [lsearch -exact [info globals] foo::a]\n"),
        "-1\n",
    );
    out_eq(
        &format!("{setup}puts [expr {{[lsearch -exact [info globals] gx] >= 0}}]\n"),
        "1\n",
    );
    // A qualified `info vars` lists that namespace, re-qualified, direct members.
    out_eq(
        &format!("{setup}puts [lsort [info vars ::foo::*]]\n"),
        "::foo::a ::foo::b\n",
    );
    // At namespace scope, `info vars` lists the namespace's own variables.
    out_eq(
        &format!("{setup}namespace eval foo {{ puts [lsort [info vars]] }}\n"),
        "a b\n",
    );
}

#[test]
fn info_commands_procs_namespaced() {
    // `info commands`/`procs` route through the shared namespace-aware core. The
    // expectations are pinned against tclsh 9.0 (see C `InfoCommandsCmd`/
    // `InfoProcsCmd`): a qualified pattern lists that namespace re-qualified
    // absolute; `info commands` merges the global namespace, `info procs` never
    // does; and a global-scope listing excludes namespaced names.
    let setup = "namespace eval foo { proc bar {} {}; proc baz {} {} }\n\
                 namespace eval foo::sub { proc deep {} {} }\nproc gproc {} {}\n";
    // A namespaced proc is not in the global command/proc listing.
    out_eq(
        &format!("{setup}puts [lsearch -exact [info commands] foo::bar]\n"),
        "-1\n",
    );
    // A qualified glob lists that namespace's members, re-qualified absolute,
    // and only direct members (not `foo::sub::deep`).
    out_eq(
        &format!("{setup}puts [lsort [info commands ::foo::*]]\n"),
        "::foo::bar ::foo::baz\n",
    );
    // A *relative* qualifier re-qualifies to absolute too.
    out_eq(
        &format!("{setup}puts [lsort [info commands foo::*]]\n"),
        "::foo::bar ::foo::baz\n",
    );
    out_eq(
        &format!("{setup}puts [lsort [info procs ::foo::*]]\n"),
        "::foo::bar ::foo::baz\n",
    );
    // Inside a namespace: `info procs` lists only that namespace's procs (no
    // global merge), while `info commands` *does* see the global `gproc`.
    out_eq(
        &format!("{setup}namespace eval foo {{ puts [lsort [info procs]] }}\n"),
        "bar baz\n",
    );
    out_eq(
        &format!(
            "{setup}namespace eval foo {{ puts [expr {{[lsearch -exact [info commands] gproc] >= 0}}] }}\n"
        ),
        "1\n",
    );
    // A missing namespace qualifier yields the empty list, not an error.
    out_eq(&format!("{setup}puts [info commands ::nope::*]\n"), "\n");
}

#[test]
fn info_default() {
    // A parameter with a default: the var is set to it and the command returns 1.
    out_eq(
        "proc f {a {b 99}} {}\nputs [info default f b d]:$d\n",
        "1:99\n",
    );
    // A parameter with no default: the var is set to the empty string, returns 0.
    out_eq(
        "proc f {a {b 99}} {}\nputs [info default f a d]:<$d>\n",
        "0:<>\n",
    );
    // An unknown parameter is the shared catalogue error (pinned vs tclsh 9.0).
    let (ok, msg, _) = run("proc f {a {b 99}} {}\ninfo default f zz d");
    assert!(!ok);
    assert_eq!(msg, "procedure \"f\" doesn't have an argument \"zz\"");
    // A non-proc target.
    let (ok, msg, _) = run("info default nosuch a d");
    assert!(!ok);
    assert_eq!(msg, "\"nosuch\" isn't a procedure");
}

#[test]
fn proc_body_command_subst() {
    // A proc body is a braced literal: its `[...]` must NOT be substituted at
    // definition time — it runs per-call, against the proc's locals.
    out_eq(
        "proc f {s} { return [string length $s] }\nputs [f hello]\n",
        "5\n",
    );
    out_eq(
        "proc f {l} { return [lindex $l 1] }\nputs [f {a b c}]\n",
        "b\n",
    );
}

#[test]
fn info_exists_local_scalar() {
    // `info exists` on a proc local compiles to `existScalar`/`existStk`.
    out_eq(
        "proc f {} { set x 1; return [info exists x] }\nputs [f]\n",
        "1\n",
    );
    out_eq("proc f {} { return [info exists nope] }\nputs [f]\n", "0\n");
    out_eq(
        "proc f {} { set x 1; unset x; return [info exists x] }\nputs [f]\n",
        "0\n",
    );
}

#[test]
fn info_exists_local_array_elem() {
    out_eq(
        "proc f {} { set a(x) 1; return [info exists a(x)] }\nputs [f]\n",
        "1\n",
    );
    out_eq(
        "proc f {} { set a(x) 1; return [info exists a(y)] }\nputs [f]\n",
        "0\n",
    );
}
