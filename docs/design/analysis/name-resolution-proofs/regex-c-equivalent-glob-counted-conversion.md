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

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded build label; runtime query not retained). Build: Original library SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; probe executable SHA256 8fce5d637533517944149880f3a6db15e7337793ad71ebd91cd26e5cc49cf15c. Channel: Direct counted TclReToGlob call on original C ASCII pattern literals.. Dialect: C Tcl.

Build exit 0, process exit 0, empty stderr. Original code/glob-hex rows: "0\t0\t2a\n1\t0\t2a666f6f2a\n2\t0\t666f6f\n3\t0\t2a\n4\t0\t2a3f2a\n5\t1\t\n6\t0\t2a5c5c2a\n7\t0\t2a0a2a\n8\t0\t2a5c2a2a\n9\t0\t2a785c3f2a\n10\t1\t\n11\t1\t\n12\t1\t\n13\t1\t\n". Code one is a converter decline, not a missing guest command.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded build label; runtime query not retained). Build: Original library SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; probe executable SHA256 9e397a137587a4dbfda5d803c5077f7b709ef4218996ccafe56f632f67a2d1d7. Channel: Direct counted TclReToGlob call on original C ASCII pattern literals.. Dialect: C Tcl.

Build exit 0, process exit 0, empty stderr. Original code/glob-hex rows: "0\t0\t2a\n1\t0\t2a666f6f2a\n2\t0\t666f6f\n3\t0\t2a\n4\t0\t2a3f2a\n5\t1\t\n6\t0\t2a5c5c2a\n7\t0\t2a0a2a\n8\t0\t2a5c2a2a\n9\t0\t2a785c3f2a\n10\t1\t\n11\t1\t\n12\t1\t\n13\t1\t\n". Code one is a converter decline, not a missing guest command.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build label; runtime query not retained). Build: Original library SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; probe executable SHA256 adfad5badfb0d97e896003476009b819ce776bb1214f98f88d4107c6cddbd298. Channel: Direct counted TclReToGlob call on original C ASCII pattern literals.. Dialect: C Tcl.

Build exit 0, process exit 0, empty stderr. Original code/glob-hex rows: "0\t0\t2a\n1\t0\t2a666f6f2a\n2\t0\t666f6f\n3\t0\t2a\n4\t0\t2a3f2a\n5\t1\t\n6\t0\t2a5c5c2a\n7\t0\t2a0a2a\n8\t0\t2a5c2a2a\n9\t0\t2a785c3f2a\n10\t1\t\n11\t1\t\n12\t1\t\n13\t1\t\n". Code one is a converter decline, not a missing guest command.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build label; runtime query not retained). Build: Original library SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; probe executable SHA256 bf7eecafa1a871669181c6d4bf587dde3cc2687f27829851a23dfedcb4be000b. Channel: Direct counted TclReToGlob call on original C ASCII pattern literals.. Dialect: C Tcl.

Build exit 0, process exit 0, empty stderr. Original code/glob-hex rows: "0\t0\t2a\n1\t0\t2a666f6f2a\n2\t0\t666f6f\n3\t0\t2a\n4\t0\t2a3f2a\n5\t1\t\n6\t0\t2a5c5c2a\n7\t0\t2a0a2a\n8\t0\t2a5c2a2a\n9\t0\t2a785c3f2a\n10\t1\t\n11\t1\t\n12\t1\t\n13\t1\t\n". Code one is a converter decline, not a missing guest command.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Exact evidence

- `original-build-and-rows` (provider): [rust/tcl-syntax/tests/data/native_regexp_glob/manifest.json](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/manifest.json). SHA-256 `c948a2263a22c072d9377ed90ee814482e5a06577a2e17ad97f475d16fab37bc`. Exact original builds, warnings, process statuses and converter output rows.
- `original-probe` (input): [rust/tcl-syntax/tests/data/native_regexp_glob/probe.c](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/probe.c). SHA-256 `12f235e3a3e124f12fd567e6daaa3c58b5b05a304a5c8622cc1405288fa5c879`. Exact counted pattern strings, version-selected ABI and observed output fields.
- `tcl8.5-rows` (observation): [rust/tcl-syntax/tests/data/native_regexp_glob/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/8.5.19.txt). SHA-256 `0f6bb55aa687df36661ee1c0a2f0523ad9fede3f401903a67397e6d37d2506f7`. Exact completed original fourteen converter result rows.
- `tcl8.6-rows` (observation): [rust/tcl-syntax/tests/data/native_regexp_glob/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/8.6.18.txt). SHA-256 `0f6bb55aa687df36661ee1c0a2f0523ad9fede3f401903a67397e6d37d2506f7`. Exact completed original fourteen converter result rows.
- `tcl9.0-rows` (observation): [rust/tcl-syntax/tests/data/native_regexp_glob/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/9.0.4.txt). SHA-256 `0f6bb55aa687df36661ee1c0a2f0523ad9fede3f401903a67397e6d37d2506f7`. Exact completed original fourteen converter result rows.
- `tcl9.1-rows` (observation): [rust/tcl-syntax/tests/data/native_regexp_glob/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_regexp_glob/9.1.0.txt). SHA-256 `0f6bb55aa687df36661ee1c0a2f0523ad9fede3f401903a67397e6d37d2506f7`. Exact completed original fourteen converter result rows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_regex.rs](../../../../rust/tcl-syntax/src/native_regex.rs), `NativeRegexpRecipe`: Independent release-selected converter recipe; outputs alone supply no selected command/cache/body authority.
- [rust/tcl-syntax/src/native_regex.rs](../../../../rust/tcl-syntax/src/native_regex.rs), `native_regex::tests::equivalent_glob_matches_actual_counted_converter_rows` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.
