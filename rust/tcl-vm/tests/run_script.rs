//! End-to-end: compile real Tcl to bytecode (via `tcl-compiler`, dev-dep only)
//! then run it through `tcl-vm`, asserting result + captured `puts` output.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, Commands, CompileError, CompileService, Traces, Value, Vm};

/// A `tcl-compiler`-backed compile service so the VM can resolve runtime
/// `eval` / `[command substitution]` (the injection seam — `tcl-vm` itself
/// never depends on the compiler).
struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

/// A `Write` sink backed by a shared buffer the test can read afterwards.
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `src`; return `(ok, result-string, captured-stdout)`.
fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

/// `Commands::dispatch` runs a compiled proc to completion — the nested
/// `is_proc` activation path (`invoke_command` pushes the call-frame, runs the
/// body, and absorbs `return` into an ok completion), and surfaces the proc's
/// arg-count usage error when binding fails.
#[test]
fn commands_dispatch_runs_proc() {
    let registry = CommandRegistry::build_default();
    let asm = {
        let src = "proc add {a b} { return [expr {$a + $b}] }";
        let ir = lower_to_ir(src, &registry);
        let cfg = build_cfg(&ir, false);
        codegen_module(&cfg, &ir, &registry)
    };

    let mut vm = Vm::new();
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    // Running the module registers `add`.
    assert!(vm.run_module(&asm).code.is_ok());

    // Dispatch the proc by name with name-stripped argv.
    let c = vm.dispatch("add", &[Value::string("3"), Value::string("4")]);
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "7");

    // Wrong arg count surfaces the proc usage error (the `enter_proc` Err path).
    let c = vm.dispatch("add", &[Value::string("3")]);
    assert_eq!(c.code, Code::Error);
    assert_eq!(&*c.result.to_str(), "wrong # args: should be \"add a b\"");
}

/// `Traces::fire` runs a variable's registered traces, aborting the access with
/// the wrapped `can't read "var": <msg>` error when a read/write callback fails,
/// and swallowing `unset`-trace errors (matching C and the runtime). Traces are
/// registered via the `trace` command (callbacks evaluate through the compiler).
#[test]
fn traces_fire_ok_error_and_unset() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    // `;#` comments out the appended `name elem op` trace words.
    for add in [
        "trace add variable x read {list ok;#}",
        "trace add variable y read {error boom;#}",
        "trace add variable z unset {error nope;#}",
    ] {
        assert!(vm.eval_source(add).expect("compiles").code.is_ok());
    }

    // Read trace, callback succeeds → the access proceeds.
    assert!(Traces::fire(&mut vm, "x", "read").is_ok());
    // Read trace, callback errors → the access aborts with the wrapped error.
    assert_eq!(
        &*Traces::fire(&mut vm, "y", "read").unwrap_err().to_str(),
        "can't read \"y\": boom",
    );
    // An `unset`-trace error does not abort the access.
    assert!(Traces::fire(&mut vm, "z", "unset").is_ok());
}

/// `info level` runs through the shared Family-B core
/// (`tcl_cmd_core::info::level`, over the `Introspect` role trait): the current
/// depth with no argument, and the correct coercion error for a non-integer
/// argument (the VM previously diverged from the runtime / real Tcl with a
/// "bad level" message — routing through the shared core unifies the behaviour).
#[test]
fn info_level_shared_core() {
    assert_eq!(run("info level").1, "0"); // global scope: depth 0
    let (ok, result, _out) = run("info level foo");
    assert!(!ok);
    assert_eq!(result, "expected integer but got \"foo\"");
}

/// `info exists` runs through the shared core (`VarStore::exists`). Routing it
/// surfaced and fixed a VM bug: the current-frame existence check was scalar-only
/// (`var_exists`), so arrays like `::env` / `a` reported as not existing.
#[test]
fn info_exists_shared_core() {
    assert_eq!(run("info exists nope").1, "0");
    assert_eq!(run("set x 1\ninfo exists x").1, "1");
    // Arrays and array elements (the cases the old scalar-only check missed).
    assert_eq!(run("set a(k) v\ninfo exists a").1, "1");
    assert_eq!(run("set a(k) v\ninfo exists a(k)").1, "1");
    assert_eq!(run("set a(k) v\ninfo exists a(nope)").1, "0");
}

/// `namespace tail`/`qualifiers` run through the shared pure core
/// (`tcl_cmd_core::namespace`). Routing fixed the VM's `::`-run handling: a run
/// of 3+ colons is one separator (C semantics), where the VM's old `rsplit("::")`
/// yielded a stray `:`.
#[test]
fn namespace_tail_qualifiers_colon_runs() {
    assert_eq!(run("namespace tail ::a::b::c").1, "c");
    assert_eq!(run("namespace qualifiers ::a::b::c").1, "::a::b");
    assert_eq!(run("namespace tail foo:::").1, ""); // was ":" before the fix
    assert_eq!(run("namespace qualifiers foo:::").1, "foo");
}

/// `info complete` runs through the shared core (`tcl_cmd_core::info::complete`,
/// C's `Tcl_CommandComplete`). Routing fixed the VM, whose old counter tracked
/// brackets even inside `{braces}` (where `[` is literal): `{[}` is complete.
#[test]
fn info_complete_shared_core() {
    assert_eq!(run("info complete {set x 1}").1, "1");
    assert_eq!(run("info complete {set x [}").1, "0"); // unclosed bracket
    assert_eq!(run("info complete {{[}}").1, "1"); // `{[}` — was "0" before the fix
}

#[test]
fn set_expr_puts() {
    let (ok, _result, out) = run("set x 5\nputs [expr {$x * 2}]\n");
    assert!(ok);
    assert_eq!(out, "10\n");
}

#[test]
fn expr_precedence() {
    let (ok, result, _out) = run("expr {3 + 4 * 2}");
    assert!(ok);
    assert_eq!(result, "11");
}

#[test]
fn floored_integer_division() {
    let (ok, result, _out) = run("expr {-7 / 2}");
    assert!(ok);
    assert_eq!(result, "-4");
}

#[test]
fn incr_and_var_substitution() {
    let (ok, _result, out) = run("set n 0\nincr n\nincr n 5\nputs $n\n");
    assert!(ok);
    assert_eq!(out, "6\n");
}

#[test]
fn string_and_numeric_compare() {
    let (ok, result, _out) = run("expr {9 < 10}");
    assert!(ok);
    assert_eq!(result, "1");
}

/// The `string` subcommands ported into the shared `tcl-cmd-core` (driven over
/// the VM's `ValueOps`) behave identically end-to-end.
#[test]
fn string_core_helpers() {
    assert_eq!(run("puts [string length hello]").2, "5\n");
    assert_eq!(run("puts [string index hello 1]").2, "e\n");
    assert_eq!(run("puts [string range hello 1 3]").2, "ell\n");
    assert_eq!(run("puts [string reverse hello]").2, "olleh\n");
    assert_eq!(run("puts [string repeat ab 3]").2, "ababab\n");
    // end-relative and arithmetic index forms resolve in the shared core.
    assert_eq!(run("puts [string index hello end]").2, "o\n");
    assert_eq!(run("puts [string index hello 1+1]").2, "l\n");
    assert_eq!(run("puts [string range hello 1 end-1]").2, "ell\n");
    // count <= 0 yields empty; an out-of-range index yields empty.
    assert_eq!(run("puts \"[string repeat ab 0]<\"").2, "<\n");
    assert_eq!(run("puts \"[string index hello 99]<\"").2, "<\n");
}

/// A non-integer `string repeat` count produces the canonical coercion error,
/// surfaced through `tcl-cmd-core`'s `CmdError` from `ValueError`.
#[test]
fn string_repeat_bad_count_errors() {
    let (ok, result, _out) = run("string repeat ab x");
    assert!(!ok);
    assert_eq!(result, "expected integer but got \"x\"");
}

/// The `list`-family commands ported into the shared `tcl-cmd-core` (driven over
/// the VM's `ValueOps` list operations) behave identically end-to-end.
#[test]
fn list_core_helpers() {
    assert_eq!(run("puts [llength {a b c}]").2, "3\n");
    assert_eq!(run("puts [lindex {a b c} 1]").2, "b\n");
    assert_eq!(run("puts [lindex {a b c} end]").2, "c\n");
    assert_eq!(run("puts [lindex {{a b} {c d}} 1 0]").2, "c\n");
    assert_eq!(run("puts [lrange {a b c d} 1 2]").2, "b c\n");
    assert_eq!(run("puts [lreverse {a b c}]").2, "c b a\n");
    assert_eq!(run("puts [lrepeat 3 x]").2, "x x x\n");
    assert_eq!(run("puts [concat a {b c} d]").2, "a b c d\n");
    assert_eq!(run("puts [join {a b c} -]").2, "a-b-c\n");
    assert_eq!(run("puts [llength [split a,b,c ,]]").2, "3\n");
    assert_eq!(run("puts [list x {a b} y]").2, "x {a b} y\n");
}

/// A malformed list index errors faithfully (the shared core errors where the
/// VM's old lenient path silently returned empty).
#[test]
fn lindex_bad_index_errors() {
    let (ok, result, _out) = run("lindex {a b c} foo");
    assert!(!ok);
    assert!(result.contains("bad index"), "got: {result}");
}

/// Case conversions (with the `?first? ?last?` range form), `replace`, `insert`.
#[test]
fn string_case_replace_insert() {
    assert_eq!(run("puts [string toupper hello]").2, "HELLO\n");
    assert_eq!(run("puts [string toupper hello 0 2]").2, "HELlo\n");
    assert_eq!(run("puts [string tolower HELLO 1]").2, "HeLLO\n");
    assert_eq!(run("puts [string totitle hello]").2, "Hello\n");
    assert_eq!(run("puts [string replace abcde 1 3]").2, "ae\n");
    assert_eq!(run("puts [string replace abcde 1 3 XY]").2, "aXYe\n");
    assert_eq!(run("puts [string insert abc 1 XY]").2, "aXYbc\n");
    assert_eq!(run("puts [string insert abc end Z]").2, "abcZ\n");
}

/// `string cat` and the `string trim` family, ported into the shared core.
#[test]
fn string_cat_and_trim() {
    assert_eq!(run("puts [string cat foo bar baz]").2, "foobarbaz\n");
    assert_eq!(run("puts \"[string trim {  hi  }]<\"").2, "hi<\n");
    assert_eq!(run("puts \"[string trimleft xxabc x]<\"").2, "abc<\n");
    assert_eq!(run("puts \"[string trimright abcxx x]<\"").2, "abc<\n");
}

/// `string is` classification, ported into the shared core (`string_is`).
#[test]
fn string_is_classes() {
    assert_eq!(run("puts [string is alpha abc]").2, "1\n");
    assert_eq!(run("puts [string is alpha abc1]").2, "0\n");
    assert_eq!(run("puts [string is integer 123]").2, "1\n");
    assert_eq!(run("puts [string is integer 12x]").2, "0\n");
    assert_eq!(run("puts [string is double 1.5]").2, "1\n");
    assert_eq!(run("puts [string is boolean yes]").2, "1\n");
    assert_eq!(run("puts [string is list {a b c}]").2, "1\n");
    // -failindex reports the first failing character.
    assert_eq!(run("string is alpha -failindex fi abc5\nset fi").1, "3");
}

/// `format` rendering, ported into the shared core over `ValueOps`.
#[test]
fn format_core() {
    assert_eq!(run("puts [format %d-%s 5 hi]").2, "5-hi\n");
    assert_eq!(run("puts [format %05d 42]").2, "00042\n");
    assert_eq!(run("puts [format %05d -42]").2, "-0042\n");
    assert_eq!(run("puts [format %+d 42]").2, "+42\n");
    assert_eq!(run("puts \"[format %-5d 42]|\"").2, "42   |\n");
    assert_eq!(run("puts [format %#x 255]").2, "0xff\n");
    assert_eq!(run("puts [format %c 65]").2, "A\n");
    assert_eq!(run("puts [format %.2f 1.23456]").2, "1.23\n");
    assert_eq!(run("puts [format %.3s hello]").2, "hel\n");
    assert_eq!(run("puts [format 100%%]").2, "100%\n");
}

/// The pure `dict` family, ported into the shared core (over the default
/// list-backed `dict_pairs`/`new_dict` seam).
#[test]
fn dict_core_helpers() {
    assert_eq!(run("puts [dict get {a 1 b 2} b]").2, "2\n");
    assert_eq!(run("puts [dict size {a 1 b 2}]").2, "2\n");
    assert_eq!(run("puts [dict keys {a 1 b 2}]").2, "a b\n");
    assert_eq!(run("puts [dict exists {a 1} a]").2, "1\n");
    assert_eq!(run("puts [dict exists {a 1} z]").2, "0\n");
    assert_eq!(
        run("puts [dict merge {a 1 b 2} {b 3 c 4}]").2,
        "a 1 b 3 c 4\n"
    );
    // canonicalisation (last value wins) — the VM's old non-deduping path was
    // wrong here; the shared core corrects it.
    assert_eq!(run("puts [dict get [dict create x 1 x 2] x]").2, "2\n");
}

/// A missing key errors faithfully.
#[test]
fn dict_get_missing_key_errors() {
    let (ok, result, _out) = run("dict get {a 1} z");
    assert!(!ok);
    assert!(result.contains("not known in dictionary"), "got: {result}");
}

/// `string first` / `string last` (character-indexed substring search).
#[test]
fn string_first_last() {
    assert_eq!(run("puts [string first bc abcbc]").2, "1\n");
    assert_eq!(run("puts [string first bc abcbc 2]").2, "3\n");
    assert_eq!(run("puts [string last bc abcbc]").2, "3\n");
    assert_eq!(run("puts [string first zz abc]").2, "-1\n");
}
