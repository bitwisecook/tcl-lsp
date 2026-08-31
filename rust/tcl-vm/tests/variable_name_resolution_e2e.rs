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

//! Variable-name resolution on the compiled path — issues #1602, #1616, #1578.
//!
//! One rule underlies the first two: **a variable name is the name word's
//! substituted *value*, resolved exactly once.** The compiled path used to get
//! it wrong in both directions — leaving a quoted word's backslash escapes
//! undecoded so the table key was the source spelling (#1616), and pushing an
//! already-resolved name back through the VM's runtime word substitution so it
//! was substituted a *second* time (#1602: `set {{a}} V` created `a` rather
//! than `{a}`, `set {a[bogus]} V` ran `bogus`, `set {${x}} V` read `x`).
//!
//! Two corollaries of that rule are pinned here too: the resolved name must
//! also be pushed *byte-exactly*, because a `\<newline>` inside a resolved name
//! is name content and not a word continuation; and the `incr` generic-invoke
//! fallback must build its word from the resolved halves rather than hand the
//! VM a name whose decoded base would substitute again.
//!
//! #1602's other half is not a name bug at all: the CFG builder's opaque
//! caller-frame widening named the *callee* on its `Statement::Barrier`, and
//! codegen dispatches a barrier that names a command — so the call site was
//! emitted twice and the callee's body ran twice. The same defect reached the
//! VM as `invalid command name "<global-frame-script>"` for the `uplevel #0`
//! widening, whose marker codegen did not filter either. Synthetic identity is
//! now the typed `ir::SyntheticMarker`, because every reserved spelling is a
//! legal Tcl command name a script may define and call — matching on the name
//! silently dropped such a call, which is the vector below.
//!
//! #1578 is separate: C's `Tcl_ArrayObjCmd` set path resolves its target
//! through the standard variable lookup, which parses the name — so an
//! element-form target (`array set (x) …`, `array set arr(k) …`) is refused
//! before the value list is even looked at. The VM never element-parsed it.
//!
//! Every vector runs through the VM at `V8_6` and `V9_0` and, when the matching
//! real tclsh is installed, under it too — so the table cannot drift from C
//! Tcl. The two `${{}}` vectors are release-parameterised (8.x's first-close
//! `${…}` rule versus 9.x's nesting rule), which is why the pair of columns is
//! kept rather than one expectation.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        compile_exact_profile(src, profile)
    }
}

fn compile_exact_profile(
    src: &str,
    profile: &'static DialectProfile,
) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error_with_config(src, config) {
        return Err(CompileError(msg));
    }
    let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
        src,
        registry,
        config,
        Some(profile),
    );
    let cfg = build_cfg_codegen(&ir, false);
    Ok(codegen_module(&cfg, &ir, registry))
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run `src` in the VM at `version`, returning its trimmed `puts` output — the
/// same shape the tclsh leg produces, so the two are directly comparable.
fn vm_output(src: &str, version: TclVersion) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let service = CompilerSvc {
        registry: CommandRegistry::build_default(),
    };
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(service));
    vm.set_runtime_version(version);
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&cap.0.borrow()).trim().to_string()
}

/// Run `src` under a real tclsh, or `None` when that binary isn't available.
fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    use std::io::Write as _;
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var(bin_env) {
        candidates.push(explicit);
    }
    candidates.extend(names.iter().map(ToString::to_string));
    for name in candidates {
        let Ok(mut child) = std::process::Command::new(&name)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            continue;
        };
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(src.as_bytes())
            .expect("write");
        let out = child.wait_with_output().expect("run");
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
        }
    }
    None
}

/// One behaviour vector: the script prints its observations; `want_8x` and
/// `want_90` are the full expected stdout under each release's semantics.
struct Vector {
    name: &'static str,
    script: &'static str,
    want_8x: &'static str,
    want_90: &'static str,
}

const VECTORS: &[Vector] = &[
    // -- #1602, the name half: a resolved name is never substituted again. --
    Vector {
        name: "a braced name keeps its own braces (set {{zz}} names `{zz}`)",
        script: r"set {{zz}} V
set n [lindex [info vars *zz*] 0]
puts [info exists {{zz}}]:[set {{zz}}]:$n:[string length $n]
",
        want_8x: "1:V:{zz}:4",
        want_90: "1:V:{zz}:4",
    },
    Vector {
        name: "a `[…]` inside a braced name is data, not a command to run",
        script: r"set {zc[bogus]} V
set n [lindex [info vars zc*] 0]
puts [info exists {zc[bogus]}]:$n:[string length $n]
",
        want_8x: "1:zc[bogus]:9",
        want_90: "1:zc[bogus]:9",
    },
    Vector {
        name: "a `${…}` inside a braced name is data, not a variable to read",
        script: r"set {zd${nope}} V
set n [lindex [info vars zd*] 0]
puts [info exists {zd${nope}}]:$n:[string length $n]:[info exists nope]
",
        want_8x: "1:zd${nope}:9:0",
        want_90: "1:zd${nope}:9:0",
    },
    Vector {
        name: "a braced array key keeps its braces too",
        script: r"set za({k}) V
set k [lindex [array names za] 0]
puts $k:[string length $k]:[set za($k)]
",
        want_8x: "{k}:3:V",
        want_90: "{k}:3:V",
    },
    Vector {
        // The *read* side of the vector above: a key the compiler has already
        // resolved must reach the VM byte-exactly, or `subst_word` strips its
        // braces and the load looks up `zb(k)`. Storing the key correctly while
        // reading it through a substituting push is worse than getting both
        // wrong — the round-trip stops working — so the two sides are pinned
        // together.
        name: "an escape-braced array key round-trips through a `$` read",
        script: r"set zb(\{k\}) V
set k [lindex [array names zb] 0]
puts $k:[string length $k]:$zb(\{k\}):[set zb(\{k\})]
",
        want_8x: "{k}:3:V:V",
        want_90: "{k}:3:V:V",
    },
    Vector {
        // Same rule inside a composite key: the parser has decoded the literal
        // halves, so they are finished too and only the `$i` still substitutes.
        name: "a composite array key keeps its decoded literal half byte-exact",
        script: r"set i K
set zd(\{k\}$i) V
set d [lindex [array names zd] 0]
puts $d:[string length $d]:$zd(\{k\}$i):[set zd(\{k\}$i)]
",
        want_8x: "{k}K:4:V:V",
        want_90: "{k}K:4:V:V",
    },
    Vector {
        name: "a `${{}}` load follows the release's `${…}` close rule",
        script: r"set {{}} Z
puts [info exists {{}}]:[catch {set r ${{}}} m]:$m
",
        want_8x: "1:1:can't read \"{\": no such variable",
        want_90: "1:0:Z",
    },
    Vector {
        name: "a `${{}}` array key follows the same release rule",
        script: r"set {{}} Z
set za(Z) hit
puts [catch {set r $za(${{}})} m]:$m
",
        want_8x: "1:can't read \"{\": no such variable",
        want_90: "0:hit",
    },
    Vector {
        name: "a proc-local braced name round-trips through a compiled read",
        script: r"proc pl {} { set {{loc}} L ; return [set {{loc}}] }
puts [pl]
",
        want_8x: "L",
        want_90: "L",
    },
    // The negative boundary for the whole name half: names that legitimately
    // hold parens and braces must keep working, and a *live* `$` in a key must
    // still substitute.
    Vector {
        name: "ordinary and substituted array keys are untouched; a braced key stays literal",
        script: r"set i K
set za($i) 1
set za(lit) 2
set {zb($x)} 3
puts [lsort [array names za]]:[lsort [array names zb]]:[info exists {zb($x)}]:[info exists x]
",
        want_8x: "K lit:{$x}:1:0",
        want_90: "K lit:{$x}:1:0",
    },
    Vector {
        name: "escapes decide where the element starts; an escaped `$` key stays literal",
        script: r"set i IDX
set q\(b\) 1
set r(\$i) 2
set s($i) 3
puts [array names q]:[array names r]:[array names s]
puts [set q(b)]:[set r(\$i)]:[set s(IDX)]
",
        want_8x: "b:{$i}:IDX\n1:2:3",
        want_90: "b:{$i}:IDX\n1:2:3",
    },
    // -- #1602, the barrier half: a synthetic marker must never be dispatched. --
    Vector {
        name: "a braced-name upvar runs the proc body once, not twice",
        script: r#"set {a b} OUTER
proc p {} { upvar 1 {a b} v ; puts "u=$v" }
p
puts done
"#,
        want_8x: "u=OUTER\ndone",
        want_90: "u=OUTER\ndone",
    },
    Vector {
        name: "a callee that runs an unreadable global-frame script is invoked once",
        script: r#"proc setter {body} { uplevel #0 $body }
setter {set q 1}
puts "q=$q"
set y [setter {set r 2}]
puts "r=$r y=$y"
"#,
        want_8x: "q=1\nr=2 y=2",
        want_90: "q=1\nr=2 y=2",
    },
    Vector {
        name: "the same callee inside an `if` / `while` condition stays dispatch-free",
        script: r#"proc setter {body} { uplevel #0 $body }
if {[setter {set q 1}] eq "1"} { puts if-yes } else { puts if-no }
set i 0
while {[setter {set w 2}] eq "2" && $i < 1} { incr i }
puts "i=$i w=$w q=$q"
"#,
        want_8x: "if-yes\ni=1 w=2 q=1",
        want_90: "if-yes\ni=1 w=2 q=1",
    },
    // -- #1616: the name is the word's value, not its source spelling. --
    Vector {
        name: "a quoted / bare name word is backslash-substituted; a braced one is not",
        script: r#"set "z1\\" A
set "z2\}" B
set "z3\ x" C
set z4\\ D
set {z5\\} E
set "z6\x41" F
set "z7\t" G
foreach n [lsort [info vars z*]] { puts "[string length $n]:$n:[set $n]" }
"#,
        want_8x: "3:z1\\:A\n3:z2}:B\n4:z3 x:C\n3:z4\\:D\n4:z5\\\\:E\n3:z6A:F\n3:z7\t:G",
        want_90: "3:z1\\:A\n3:z2}:B\n4:z3 x:C\n3:z4\\:D\n4:z5\\\\:E\n3:z6A:F\n3:z7\t:G",
    },
    // -- #1578: `array set` refuses an element-form target. --
    Vector {
        name: "array set rejects an element-form target before it parses the list",
        script: r"puts [catch {array set (x) {a 1}} m]:$m
puts [catch {array set (x) {}} m]:$m
puts [catch {array set zarr(k) {a 1}} m]:$m
puts [catch {array set {zarr(k)} {a 1}} m]:$m
puts [catch {array set zodd {a}} m]:$m
puts [catch {array set {z)b} {a 1}} m]:$m
puts [catch {array set {z(b} {a 1}} m]:$m
puts [catch {array set zok {a 1}} m]:$m
puts [array get zok]:[array get {z)b}]:[array names {z(b}]
",
        want_8x: "1:can't set \"(x)\": variable isn't array\n\
                  1:can't set \"(x)\": variable isn't array\n\
                  1:can't set \"zarr(k)\": variable isn't array\n\
                  1:can't set \"zarr(k)\": variable isn't array\n\
                  1:list must have an even number of elements\n\
                  0:\n0:\n0:\na 1:a 1:a",
        want_90: "1:can't set \"(x)\": variable isn't array\n\
                  1:can't set \"(x)\": variable isn't array\n\
                  1:can't set \"zarr(k)\": variable isn't array\n\
                  1:can't set \"zarr(k)\": variable isn't array\n\
                  1:list must have an even number of elements\n\
                  0:\n0:\n0:\na 1:a 1:a",
    },
    // -- The review round on #1602/#1616/#1578: a marker is typed, a resolved
    //    name is pushed once and byte-exact. --
    Vector {
        name: "a proc named like a CFG marker is still a command, not a marker",
        script: r#"proc <cond> {} { puts "hit-cond" }
proc <caller-frame-opaque> {} { puts "hit-cfo" }
proc <global-frame-script> {} { puts "hit-gfs" }
proc <upvar-invalidate> {} { puts "hit-uv" }
proc <empty_clause> {} { puts "hit-ec" }
<cond>
<caller-frame-opaque>
<global-frame-script>
<upvar-invalidate>
<empty_clause>
puts done
"#,
        want_8x: "hit-cond\nhit-cfo\nhit-gfs\nhit-uv\nhit-ec\ndone",
        want_90: "hit-cond\nhit-cfo\nhit-gfs\nhit-uv\nhit-ec\ndone",
    },
    Vector {
        name: "the incr fallback keeps a resolved base literal and substitutes only the key",
        script: r#"proc b {} { puts "BOOM-b-ran" ; return 7 }
set i K
set a\133b\135(K) 5
incr a\133b\135($i) 999
foreach n [lsort [info vars a*]] {
    if {[array exists $n] && [string length $n] <= 6} {
        puts "array <$n> len=[string length $n] : [array get $n]"
    }
}
"#,
        want_8x: "array <a[b]> len=4 : K 1004",
        want_90: "array <a[b]> len=4 : K 1004",
    },
    Vector {
        name: "a resolved name keeps its backslash-newline bytes (no brace-word collapse)",
        script: r"set n [format a%c%cb 92 10]
set $n VALUE
set {a b} COLLAPSED
set out ${a\
b}
puts assign=$out
puts ${a\
b}
",
        want_8x: "assign=VALUE\nVALUE",
        want_90: "assign=VALUE\nVALUE",
    },
    // The negative boundary for that one: a braced or quoted *word* really does
    // collapse `\<newline>`, so the name it spells is the collapsed one.
    Vector {
        name: "a braced or quoted name word still collapses its continuation",
        script: r#"set {z1\
y} BRACED
set "z2\
y" QUOTED
foreach n [lsort [info vars z*]] {
    puts "[string length $n]:[string map [list { } _SP_] $n]:[set $n]"
}
"#,
        want_8x: "4:z1_SP_y:BRACED\n4:z2_SP_y:QUOTED",
        want_90: "4:z1_SP_y:BRACED\n4:z2_SP_y:QUOTED",
    },
    Vector {
        name: "a proc-local braced continuation name agrees with its local-variable slot",
        script: r#"proc p {} {
    set {z1\
y} LOCAL
    return "[lsort [info locals]]:[set {z1\
y}]"
}
puts [p]
"#,
        want_8x: "{z1 y}:LOCAL",
        want_90: "{z1 y}:LOCAL",
    },
    // -- #1729: array element access keeps `(base, key)` as a pair. --
    Vector {
        name: "array commands preserve bases containing parentheses",
        script: r#"foreach name [list {z(b} {z)b} {z(b)c}] {
    array set $name {a 1 b 2}
    puts "$name get=[array get $name] names=[array names $name] size=[array size $name]"
    array unset $name a
    puts "$name after=[array get $name]"
}
set r(\$i) 2
puts "literal-key=[lindex [array names r] 0]"
"#,
        want_8x: "z(b get=a 1 b 2 names=a b size=2\n\
                  z(b after=b 2\n\
                  z)b get=a 1 b 2 names=a b size=2\n\
                  z)b after=b 2\n\
                  z(b)c get=a 1 b 2 names=a b size=2\n\
                  z(b)c after=b 2\n\
                  literal-key=$i",
        want_90: "z(b get=a 1 b 2 names=a b size=2\n\
                  z(b after=b 2\n\
                  z)b get=a 1 b 2 names=a b size=2\n\
                  z)b after=b 2\n\
                  z(b)c get=a 1 b 2 names=a b size=2\n\
                  z(b)c after=b 2\n\
                  literal-key=$i",
    },
    // -- #1582 / #1588: `upvar` resolves semantic homes before shape checks. --
    Vector {
        name: "upvar validates namespace targets and rejects inverted proc links",
        script: r#"namespace eval x { variable ok READY }
puts "control=[catch {upvar #0 ::x::ok top} m opts]:[set top]"
proc p {} {
    set proc_local 1
    puts "inverted=[catch {upvar 0 proc_local ::x::link(k)} m]:$m:$::errorCode"
}
p
puts "target=[catch {upvar #0 ::missing::x local} m]:$m:$::errorCode"
proc compiled_target {} { upvar #0 ::missing::compiled local }
puts "compiled-target=[catch {compiled_target} m]:$m:$::errorCode"
puts "local=[catch {upvar #0 x ::missing::local} m]:$m:$::errorCode"
puts "element=[catch {upvar #0 x local(k)} m]:$m:$::errorCode"
"#,
        want_8x: "control=0:READY\n\
                  inverted=1:bad variable name \"::x::link(k)\": can't create namespace variable that refers to procedure variable:TCL UPVAR INVERTED\n\
                  target=1:can't access \"::missing::x\": parent namespace doesn't exist:TCL LOOKUP VARNAME ::missing::x\n\
                  compiled-target=1:can't access \"::missing::compiled\": parent namespace doesn't exist:TCL LOOKUP VARNAME ::missing::compiled\n\
                  local=1:can't create \"::missing::local\": parent namespace doesn't exist:TCL LOOKUP VARNAME ::missing::local\n\
                  element=1:bad variable name \"local(k)\": can't create a scalar variable that looks like an array element:TCL UPVAR LOCAL_ELEMENT",
        want_90: "control=0:READY\n\
                  inverted=1:bad variable name \"::x::link(k)\": can't create namespace variable that refers to procedure variable:TCL UPVAR INVERTED\n\
                  target=1:can't access \"::missing::x\": parent namespace doesn't exist:TCL LOOKUP VARNAME ::missing::x\n\
                  compiled-target=1:can't access \"::missing::compiled\": parent namespace doesn't exist:TCL LOOKUP VARNAME ::missing::compiled\n\
                  local=1:can't create \"::missing::local\": parent namespace doesn't exist:TCL LOOKUP VARNAME ::missing::local\n\
                  element=1:bad variable name \"local(k)\": can't create a scalar variable that looks like an array element:TCL UPVAR LOCAL_ELEMENT",
    },
];

#[test]
fn vm_matches_the_pinned_variable_name_vectors() {
    for v in VECTORS {
        assert_eq!(
            vm_output(v.script, TclVersion::V8_6),
            v.want_8x,
            "[8.6] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_0),
            v.want_90,
            "[9.0] {}",
            v.name
        );
    }
}

/// The release-invariant vectors must also hold at 8.4, 8.5 and 9.1 — the VM is
/// release-parameterised, and only the two `${{}}` vectors have a per-release
/// answer (8.x closes the name at the first `}`, 9.x nests).
#[test]
fn release_invariant_vectors_hold_at_every_release() {
    for v in VECTORS.iter().filter(|v| v.want_8x == v.want_90) {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V9_1] {
            assert_eq!(
                vm_output(v.script, version),
                v.want_8x,
                "[{version:?}] {}",
                v.name
            );
        }
    }
}

/// The table itself is pinned to C Tcl: every vector's `want` must match what
/// the matching real tclsh prints. Skips silently per-binary when not
/// installed (CI / dev machines with `make ensure-test-deps` have both).
#[test]
fn vectors_match_real_tclsh() {
    let mut ran = 0;
    for v in VECTORS {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], v.script) {
            assert_eq!(got, v.want_8x, "[tclsh8.6] {}", v.name);
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], v.script) {
            assert_eq!(got, v.want_90, "[tclsh9.0] {}", v.name);
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!("skipping: neither tclsh8.6 nor tclsh9.0 found");
    }
}
