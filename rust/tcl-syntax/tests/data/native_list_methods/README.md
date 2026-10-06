# Native Tcl 9 list-method observations

These C programs install a real `Tcl_ObjType` with native list methods. They
measure which methods execute, their interpreter side effects, and the completion
codes returned by the actual interpreter. They do not implement command lookup
or predict execution from a command's spelling.

The checked output files contain 50 observations from Tcl 9.0.4 and 9.1.0:

| Program | Observations per release | Purpose |
| --- | ---: | --- |
| `methods-probe.c` | 11 | Length, Index and Slice side effects; native completion normalization; stock and replaced sequence creators |
| `ordered-methods-probe.c` | 4 | Duplicate and Elements callbacks for actual variable/value lists and grouped indices |
| `literal-pool-probe.c` | 4 | Fresh compiled literals versus a host-mutated pooled literal; existing/new compilation and generic execution |
| `empty-foreach-probe.c` | 6 | Zero-trip compiled versus generic foreach; callbacks can change a procedure-local variable even when its body never runs |

Build the pinned Tcl releases with their static Unix libraries first. For each
release, set `native_tcl_root` to that release's source/build directory, then run
from the repository root (change both `native_tcl_root` and `native_tcl_version`
for the second release):

```sh
native_tcl_root="$PWD/tmp/tcl9.0.4"
native_tcl_version=9.0
native_probe_dir="$PWD/rust/tcl-syntax/tests/data/native_list_methods"
native_probe_build=$(mktemp -d)
for native_probe in methods-probe ordered-methods-probe empty-foreach-probe literal-pool-probe; do
    cc -std=c11 -I"$native_tcl_root/generic" -I"$native_tcl_root/unix" \
        "$native_probe_dir/$native_probe.c" \
        "$native_tcl_root/unix/libtcl$native_tcl_version.a" \
        -ldl -lz -lrt -lpthread -lm -o "$native_probe_build/$native_probe"
    "$native_probe_build/$native_probe" > "$native_probe_build/$native_probe.txt"
    diff -u "$native_probe_dir/$native_probe-$native_tcl_version.txt" \
        "$native_probe_build/$native_probe.txt"
done
rm -rf "$native_probe_build"
```

For 9.1 use `tmp/tcl9.1.0` and `native_tcl_version=9.1`. Native library flags
come from the release's `unix/tclConfig.sh`; the command includes the union of
the two pinned releases' Unix flags. No network access or repository binary is
required once Tcl is built.

Each output line is `label|status|result|length_calls`, except empty foreach,
which appends `elements_calls|duplicate_calls`. Counts are cumulative within
`methods-probe.c` and `ordered-methods-probe.c`, and reset for each empty-foreach
observation. The callback writes are intentionally performed through Tcl's
native API. The empty probe uses the current procedure frame; `ran NO` verifies
that no iteration body ran.

Tcl 9.1's generic Slice route can retain `TCL_RETURN`; its compiled route and
9.0 normalize that callback result to error. Interpreter-world effect closure
therefore cannot supply a completion guarantee. Compiled foreach and generic
foreach also consume different runtime object methods. A read-only Length
provider cannot discharge Index, Slice, Duplicate or Elements obligations.

The corresponding portable contract tests are
`list_object_methods::tests` in `tcl-registry`, and `object_callbacks::tests`
in `tcl-compiler`. Those tests enforce selected operation, actual compiler
selection, original operand evidence and residual uncertainty. The C probes
supply reproducible native evidence for those contracts; they are deliberately
separate from the portable implementation's execution engine.

`literal-pool-probe.c` uses a compiled helper to give the native host the
interpreter's shared literal object. Its mutation preserves the object's bytes
while installing an abstract-list internal type. Existing and newly compiled
`foreach` procedures then invoke its callbacks instead of iterating the visible
`a b` text. A directly interpreted host call uses a different object and would
not establish this pool counterexample. The pool output format is
`label|status|result|world|length_calls|elements_calls|duplicate_calls`, with
counts reset for each observation. Fresh-pool provenance must therefore be
withdrawn after unknown host/object effects; known text is insufficient.

Source and output digests can be recorded before rebuilding with
`sha256sum "$native_probe_dir/literal-pool-probe.c"
"$native_probe_dir/literal-pool-9.0.txt"
"$native_probe_dir/literal-pool-9.1.txt"`. The build loop compares the new
native output directly with these checked files.
