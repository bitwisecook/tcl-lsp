# Scalar string/free callbacks and zero-trip list preparation

These 82 observations retain arbitrary native object hooks and run actual Tcl
commands. The string updater writes `keep=MUTATED`; the intrep free hook writes
`keep=FREED`. Native scalar Length conversion invokes these hooks across all six
engines. Ordinary outer List/Dict Length reads/conversion keeps `keep=SAFE` in
this measured protocol; the container members remain custom objects and the
observation does not close independent StringAccess, nested index or variable-name
protocols. C8.4 has no Dict probe. The zero-trip foreach scalar string is empty,
but its updater still changes `keep` before any body iteration.

C outputs cover the actual generic and procedure-compiled paths. Jim includes
ordinary invocation and alias routes, and both updater/free cases. All input
factories install the custom original native object; no TclOO or text-type proxy
stands in for it. These rows distinguish interpreter-world closure from current
cache shape, bytes, allocation identity and completion bounds.

Build each C source against its pinned native tree, using the actual release ABI:

```sh
cc -std=c11 -I "$TCL_TREE/generic" -I "$TCL_TREE/unix" probe.c \
  "$TCL_TREE/unix/libtcl9.0.a" -lpthread -ldl -lm -lz -o string-callbacks
./string-callbacks "$TCL_TREE/library"
cc -std=c11 -I "$JIM_TREE" jim-probe.c "$JIM_TREE/libjim.a" -ldl -lm -o jim-callbacks
./jim-callbacks
```

Repeat for `free-probe.c` and `empty-foreach-probe.c`; Jim's zero-trip source is
`jim-empty-foreach-probe.c`. `manifest.json` records all original sources, pinned
native library SHA-256 values and complete output hashes. The Compiler
`object_callbacks` controls consume these native obligations separately from
unknown/custom-object effect withdrawal and original stock-class receipts.
