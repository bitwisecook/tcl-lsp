p = "runtime/rust/src/cmd_trace.rs"
s = open(p).read()

anchor = """    /// The registry retires the three legacy forms at 9.0, and the runtime
    /// reads that rather than carrying its own list — so the same script is a
    /// working trace at 8.x and `bad option` at 9.x, with the option
    /// enumeration following too. Issue #1444."""

new_tests = '''    /// Where #1444's letter convention meets the teardown path: a trace
    /// installed the deprecated way still receives the `rwua` **letter** when
    /// it is fired by `namespace delete` rather than an explicit `unset`.
    /// Teardown collects callbacks into a reduced list, so the flag has to be
    /// carried through that reduction. tclsh 8.6.16 gives `L:u` for the legacy
    /// registration, `M:unset` for a modern one, and — with both on one
    /// variable, newest-first — `Mz:unset Lz:u`.
    #[test]
    fn legacy_letter_survives_namespace_teardown() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label n1 n2 op} {lappend ::log $label:$op}");

            ok(
                i,
                b"namespace eval ::legacy {variable x 1\\ntrace variable x u {rec L}}",
            );
            ok(i, b"namespace delete ::legacy");
            assert_eq!(ok(i, b"set ::log"), b"L:u");

            ok(i, b"set ::log {}");
            ok(
                i,
                b"namespace eval ::modern {variable y 1\\ntrace add variable y unset {rec M}}",
            );
            ok(i, b"namespace delete ::modern");
            assert_eq!(ok(i, b"set ::log"), b"M:unset");

            // Both conventions on one variable: each callback keeps its own.
            ok(i, b"set ::log {}");
            ok(
                i,
                b"namespace eval ::both {variable z 1\\ntrace variable z u {rec Lz}\\ntrace add variable z unset {rec Mz}}",
            );
            ok(i, b"namespace delete ::both");
            assert_eq!(ok(i, b"set ::log"), b"Mz:unset Lz:u");

            // An array element registered the legacy way behaves the same.
            ok(i, b"set ::log {}");
            ok(
                i,
                b"namespace eval ::arr {variable a\\nset a(k) 1\\ntrace variable a(k) u {rec A}}",
            );
            ok(i, b"namespace delete ::arr");
            assert_eq!(ok(i, b"set ::log"), b"A:u");
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

    /// The 9.x side of the same seam: the legacy form does not exist there, so
    /// a modern registration fired by teardown must still say `unset`.
    #[test]
    fn teardown_op_word_is_the_full_word_at_9x() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            ok(i, b"set ::log {}");
            ok(i, b"proc rec {label n1 n2 op} {lappend ::log $label:$op}");
            ok(
                i,
                b"namespace eval ::modern {variable y 1\\ntrace add variable y unset {rec M}}",
            );
            ok(i, b"namespace delete ::modern");
            assert_eq!(ok(i, b"set ::log"), b"M:unset");
            assert_eq!(
                err(i, b"trace variable q u {rec L}"),
                b"bad option \\"variable\\": must be add, info, or remove"
            );
            i.eval_str(b"unset -nocomplain ::log");
        });
    }

'''

assert s.count(anchor) == 1
s = s.replace(anchor, new_tests + anchor, 1)
open(p, "w").write(s)
print("tests added")
