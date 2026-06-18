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

/// `namespace current`/`which` route through the shared `Namespaces` cores
/// (`current`/`name` and `find_command`/`command_name`).
#[test]
fn namespace_current_which_shared_core() {
    assert_eq!(run("namespace current").1, "::");
    assert_eq!(run("namespace eval foo {namespace current}").1, "::foo");
    assert_eq!(run("namespace which -command set").1, "::set");
    assert_eq!(run("namespace which -command no_such_cmd_xyz").1, "");
}

/// `file dirname`/`tail`/`extension`/`rootname` run through the shared
/// `/`-based byte path core (platform-independent), replacing the VM's old
/// `std::path::Path` versions.
#[test]
fn file_path_ops_shared_core() {
    assert_eq!(run("file tail /a/b/c").1, "c");
    assert_eq!(run("file dirname /a/b/c").1, "/a/b");
    assert_eq!(run("file extension a/b.txt").1, ".txt");
    assert_eq!(run("file rootname a/b.txt").1, "a/b");
    assert_eq!(run("file tail /a/b/").1, "b"); // trailing slash ignored
    assert_eq!(run("file dirname foo").1, ".");
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

/// `incr` routed through the shared `tcl_cmd_core::var::incr_value` core (the
/// `VarStore` read + `ValueOps::int_add` tower seam). Covers the unset → 0 start,
/// array elements (the name carries `base(key)`; the VM parses it), and the
/// canonical coercion errors. Crucially, the VM has no bignum, so an overflowing
/// `incr` now **errors** (`integer value too large to represent`) rather than
/// silently wrapping as the old hand-rolled `wrapping_add` did.
#[test]
fn incr_shared_core() {
    // Unset variable starts at 0 (no prior `set`).
    assert_eq!(run("incr fresh 5").1, "5");
    assert_eq!(run("incr fresh").1, "1");
    // Array element: the VM's var_get/var_set parse `a(k)` from the name.
    assert_eq!(run("set a(k) 10\nincr a(k) 5\nset a(k)").1, "15");
    // Non-integer current value / increment → the canonical coercion error.
    let (ok, msg, _) = run("set x abc\nincr x");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"abc\"");
    let (ok, msg, _) = run("set y 1\nincr y xyz");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"xyz\"");
    // i64::MAX + 1 overflows the VM's fixed-width integer: error, not wrap.
    let (ok, msg, _) = run("set big 9223372036854775807\nincr big");
    assert!(!ok);
    assert_eq!(msg, "integer value too large to represent");
}

/// `append`/`lappend` routed through the shared cores
/// (`tcl_cmd_core::var::{append_bytes, lappend_value}`). Pins the user-visible
/// behaviour against tclsh: concatenation/list building, the no-values read
/// form, and — the fix — `append`/`lappend` of an unset variable with no values
/// errors (`can't read`) rather than the VM's old silent empty-variable create.
#[test]
fn append_lappend_shared_core() {
    assert_eq!(run("set x ab\nappend x cd ef").1, "abcdef");
    assert_eq!(run("lappend y a b\nlappend y c").1, "a b c");
    // no-values read returns the value (set) and errors (unset).
    assert_eq!(run("set s hi\nappend s").1, "hi");
    let (ok, msg, _) = run("append nope");
    assert!(!ok);
    assert_eq!(msg, "can't read \"nope\": no such variable");
    // lappend no-values validates the current value as a list.
    let (ok, msg, _) = run("set z \"{\"\nlappend z");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `binary encode`/`decode` — newly added to the VM, sharing the byte codecs
/// with the WASM runtime (`tcl_cmd_core::binary`). Values cross the VM's value
/// boundary via the byte-array convention. Pinned against tclsh 9.0.
#[test]
fn binary_encode_decode_shared() {
    assert_eq!(run("binary encode hex abc").1, "616263");
    assert_eq!(run("binary decode hex 616263").1, "abc");
    assert_eq!(run("binary encode base64 hello").1, "aGVsbG8=");
    assert_eq!(run("binary decode base64 aGVsbG8=").1, "hello");
    // base64 line wrapping at -maxlen.
    assert_eq!(run("binary encode base64 -maxlen 4 Manny").1, "TWFu\nbnk=");
    // uuencode round-trips through a command substitution.
    assert_eq!(
        run("binary decode uuencode [binary encode uuencode Cat]").1,
        "Cat"
    );
    // bad codec and the bad-subcommand dispatch message.
    let (ok, msg, _) = run("binary encode zip x");
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown subcommand \"zip\": must be base64, hex, or uuencode"
    );
    let (ok, msg, _) = run("binary frobnicate x");
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown or ambiguous subcommand \"frobnicate\": must be decode, encode, format, or scan"
    );
    // decode error message + errorCode.
    let (ok, msg, _) = run("binary decode hex 6z");
    assert!(!ok);
    assert_eq!(
        msg,
        "invalid hexadecimal digit \"z\" (U+00007A) at position 1"
    );
    assert_eq!(
        run("catch {binary decode hex 6z}; set ::errorCode").1,
        "TCL BINARY DECODE INVALID"
    );
}

/// `binary format`/`scan` routed through the shared grammar
/// (`tcl_cmd_core::binary::{format, scan}`), lifting the VM from its old
/// integer-only subset to the runtime's full code set — floats, 64-bit and
/// big-endian ints, `*` counts, and the round-trip. Pinned against tclsh 9.0.
#[test]
fn binary_format_scan_shared() {
    // codes the VM lacked before sharing: f/d floats, W (64-bit big-endian).
    assert_eq!(
        run("binary scan [binary format f 1.5] H* h; set h").1,
        "0000c03f"
    );
    assert_eq!(
        run("binary scan [binary format d 2.5] H* h; set h").1,
        "0000000000000440"
    );
    assert_eq!(
        run("binary scan [binary format W 1] H* h; set h").1,
        "0000000000000001"
    );
    // `*` count over a list.
    assert_eq!(
        run("binary scan [binary format s* {1 2 3}] H* h; set h").1,
        "010002000300"
    );
    // float round-trips through scan.
    assert_eq!(run("binary scan [binary format f 1.5] f v; set v").1, "1.5");
    // scan returns the conversion count.
    assert_eq!(run("binary scan abcd a2a2 x y").1, "2");
}

/// `lsort` comparison modes routed through the shared `tcl_cmd_core::sort` core,
/// lifting the VM from its old crude subset: `-dictionary` (was a plain byte
/// compare), the `-integer`-vs-`-real` distinction (both were double), and
/// `-nocase`. Pinned against tclsh 9.0.
#[test]
fn lsort_shared_comparison() {
    assert_eq!(run("lsort -dictionary {x10 x9 x100}").1, "x9 x10 x100");
    assert_eq!(run("lsort -integer {10 9 100 2}").1, "2 9 10 100");
    assert_eq!(run("lsort -real {1.5 1.25 10.0}").1, "1.25 1.5 10.0");
    assert_eq!(run("lsort -nocase {B a C b}").1, "a B b C");
    // -unique removes elements equal under the mode (1 and 01 for -integer).
    assert_eq!(run("lsort -integer -unique {1 01 1 2}").1, "1 2");
    // plain ascii is unchanged (lexical, so x10 < x100 < x9).
    assert_eq!(run("lsort {x10 x9 x100}").1, "x10 x100 x9");
}

/// `::tcl::mathop::*` — newly added to the VM, sharing `tcl_cmd_core::mathop`'s
/// fold/chain logic over the VM's `ExprOps`. Pinned against tclsh 9.0.
#[test]
fn mathop_shared() {
    assert_eq!(run("::tcl::mathop::+ 1 2 3").1, "6");
    assert_eq!(run("::tcl::mathop::+").1, "0"); // identity
    assert_eq!(run("::tcl::mathop::- 10 3 2").1, "5");
    assert_eq!(run("::tcl::mathop::- 5").1, "-5"); // 1 arg negates
    assert_eq!(run("::tcl::mathop::* 2 3 4").1, "24");
    assert_eq!(run("::tcl::mathop::/ 100 5 2").1, "10");
    assert_eq!(run("::tcl::mathop::/ 4").1, "0.25"); // reciprocal
    assert_eq!(run("::tcl::mathop::** 2 3 2").1, "512"); // right-assoc: 2**(3**2)
    assert_eq!(run("::tcl::mathop::% 17 5").1, "2");
    assert_eq!(run("::tcl::mathop::<< 1 4").1, "16");
    assert_eq!(run("::tcl::mathop::& 12 10").1, "8");
    assert_eq!(run("::tcl::mathop::~ 5").1, "-6");
    assert_eq!(run("::tcl::mathop::! 0").1, "1");
    assert_eq!(run("::tcl::mathop::< 1 2 3").1, "1"); // chained
    assert_eq!(run("::tcl::mathop::< 1 3 2").1, "0");
    assert_eq!(run("::tcl::mathop::== 5 5 5").1, "1");
    assert_eq!(run("::tcl::mathop::eq abc abc").1, "1");
    assert_eq!(run("::tcl::mathop::in b {a b c}").1, "1");
    assert_eq!(run("::tcl::mathop::ni x {a b c}").1, "1");
    let (ok, msg, _) = run("::tcl::mathop::% 1");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"::tcl::mathop::% integer integer\""
    );
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
