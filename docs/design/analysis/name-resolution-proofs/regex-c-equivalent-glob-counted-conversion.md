# naming.regex.c-equivalent-glob-counted-conversion

Kind: `native-observation`

## Problem statement

An equivalent glob is available only for a subset of RegExp syntax. Treating every pattern as a glob, or borrowing another release converter ABI, changes matching and can skip the actual compile path.

## Question

Which of the fourteen exact counted patterns does TclReToGlob convert on the four captured C releases, and what glob bytes does it produce?

## Conclusion

C8.5/8.6/9.0/9.1 return identical rows for these fourteen patterns: nine conversions return exact recorded glob bytes and five decline with code one and empty output. Declines include a.*b.*c, bracket syntax, an interior anchor, an unfinished escape and the anchored wildcard case. No C8.4 converter run, Jim behavior or general glob/RegExp equivalence is inferred.

## Scope

Original private TclReToGlob C API probe with counted strlen of fourteen fixed ASCII pattern literals; C8.5 has the recorded four-argument ABI and later builds the five-argument ABI. Exact original build warnings, headers, libraries, executable digests and completed stdout/stderr are retained. The probe does not record exact/quantifier outputs or a runtime version query. No Rust execution, C8.4, Jim or BIG-IP observation is asserted.

## Provider answers

### tcl8.4, jim, bigip

Status: `not-tested`.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Direct counted TclReToGlob call on original C ASCII pattern literals. Version: tcl8.5 8.5.19 (recorded build label; runtime query not retained); tcl8.6 8.6.18 (recorded build label; runtime query not retained); tcl9.0 9.0.4 (recorded build label; runtime query not retained); tcl9.1 9.1.0 (recorded build label; runtime query not retained).

Build exit 0, process exit 0, empty stderr. Original code/glob-hex rows: "0\t0\t2a\n1\t0\t2a666f6f2a\n2\t0\t666f6f\n3\t0\t2a\n4\t0\t2a3f2a\n5\t1\t\n6\t0\t2a5c5c2a\n7\t0\t2a0a2a\n8\t0\t2a5c2a2a\n9\t0\t2a785c3f2a\n10\t1\t\n11\t1\t\n12\t1\t\n13\t1\t\n". Code one is a converter decline, not a missing guest command.

## Consumer bindings

- [rust/tcl-syntax/src/native_regex.rs](../../../../rust/tcl-syntax/src/native_regex.rs), `NativeRegexpRecipe`: Independent release-selected converter recipe; outputs alone supply no selected command/cache/body authority.
- [rust/tcl-syntax/src/native_regex.rs](../../../../rust/tcl-syntax/src/native_regex.rs), `native_regex::tests::equivalent_glob_matches_actual_counted_converter_rows` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

## Evidence

- [rust/tcl-syntax/tests/data/native_regexp_glob/manifest.json](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/manifest.json)
- [rust/tcl-syntax/tests/data/native_regexp_glob/probe.c](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/probe.c)
- [rust/tcl-syntax/tests/data/native_regexp_glob/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/8.5.19.txt)
- [rust/tcl-syntax/tests/data/native_regexp_glob/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/8.6.18.txt)
- [rust/tcl-syntax/tests/data/native_regexp_glob/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/9.0.4.txt)
- [rust/tcl-syntax/tests/data/native_regexp_glob/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/9.1.0.txt)
