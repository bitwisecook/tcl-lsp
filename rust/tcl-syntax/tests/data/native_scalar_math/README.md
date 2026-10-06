# Selected native scalar math objects

These 30 scalar observations inspect actual `sqrt` result objects and
operand caches on Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and math-enabled
Jim 0.84 (`5bac7c99ad65864c87da513e22e2f01703fa4e03`). Each case uses a fresh
interpreter. Integer, double and numeric string inputs produce Double normal
results. C preserves existing numeric input categories; Jim's integer inputs
become coerced-double caches. Bad strings error; negative square roots error
on C and produce normal Double NaN on this Jim build.

The C probe retains the immediate result in `math_result` before returning
through legacy `Tcl_Eval`. Inspecting only that API's final result would observe
its compatibility string-result projection on C 8.x, rather than the native
math object received by Tcl's next command. No object freshness or effect
permission follows from the measured representation.

After building each pinned Tcl Unix static library, run from the repository:

```sh
native_tcl_root="$PWD/tmp/tcl8.6.18"
native_tcl_version=8.6
native_math_dir="$PWD/rust/tcl-syntax/tests/data/native_scalar_math"
native_math_build=$(mktemp -d)
cc -std=c11 -I"$native_tcl_root/generic" -I"$native_tcl_root/unix" \
    "$native_math_dir/sqrt-probe.c" \
    "$native_tcl_root/unix/libtcl$native_tcl_version.a" \
    -lpthread -ldl -lm -lz -o "$native_math_build/sqrt-probe"
"$native_math_build/sqrt-probe" "$native_tcl_root/library" \
    > "$native_math_build/sqrt.txt"
diff -u "$native_math_dir/sqrt-$native_tcl_version.txt" "$native_math_build/sqrt.txt"
```

Repeat with the other release directory/version pairs. For the pinned Jim
math-enabled build, set `native_jim_root` to its source/build directory:

```sh
cc -std=c11 -I"$native_jim_root" "$native_math_dir/jim-sqrt-probe.c" \
    "$native_jim_root/libjim.a" -ldl -lm -o "$native_math_build/jim-sqrt-probe"
"$native_math_build/jim-sqrt-probe" > "$native_math_build/jim-sqrt.txt"
diff -u "$native_math_dir/sqrt-jim084.txt" "$native_math_build/jim-sqrt.txt"
rm -rf "$native_math_build"
```

The authored result protocol follows the native sources: C 8.4's fixed table
and `ExprUnaryFunc` in `tclExecute.c`, modern `ExprSqrtFunc` in `tclBasic.c`,
`Tcl_GetDoubleFromObj` in `tclObj.c`, and Jim's `JimExprOpDoubleUnary`,
`Jim_GetDouble`, and `Jim_NewDoubleObj` in `jim.c`. Mutable command replacement,
fixed registration identity, observers and operand effects are separate
obligations in the registry/source controls.

`implicit-sqrt-cache-probe.c` adds 60 direct/procedure expression observations
across all five C releases. It compares actual `incr` results and host-created
WideInt objects at three magnitudes. Every implicit `sqrt` preserves its input
category, including Tcl8.4 WideInt. This is distinct from direct Tcl8.4
`Tcl_GetDoubleFromObj`, which can replace a WideInt cache with Double. Do not
derive the expression protocol from the direct getter's cache behavior.
Build with the same pinned C libraries and run without arguments; compare
against `implicit-sqrt-cache-VERSION.txt`. The original measured manifest is
`/workspace/.proofs/2286-root-native-sqrt-cache.json`, with source, library and
binary hashes. These observations justify input category preservation only;
they grant no numeric value, object freshness or callback-effect permission.

`custom-string-probe.c` adds five C-engine observations. A custom scalar's
`updateStringProc` retains the interpreter and changes the current procedure's
local `keep` while `GetDouble` materializes its bytes. Every native `sqrt`
still returns normal `2.0`, and `keep` becomes `MUTATED`. The normal result and
OK/Error completion therefore do not close operand-world effects. A current
native numeric object bypasses this updater; unknown/custom input needs its
independent residual. Outer List/Dict shape alone also cannot close nested
element string updaters. Build this probe with the same C command, substitute
`custom-string-probe.c`, and compare against `custom-string-VERSION.txt`.
