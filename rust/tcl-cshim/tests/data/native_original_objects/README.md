# Original native object callbacks

`probe.c` invokes native C command callbacks with two references to the same
original object. The observations inspect that original object's cache and
resident bytes after successful and failed getters, string materialization,
and nested `Tcl_EvalObjv` dispatch. They do not read guest error globals.

Build against each pinned C source/library pair:

```sh
cc -I"$TCL_SRC/generic" -I"$TCL_SRC/unix" probe.c \
  "$TCL_LIB" -lpthread -ldl -lm -lz -o probe
./probe
```

`manifest.json` records the source/library/output hashes and exact commands
for C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. The callback uses the
`int`-argc `Tcl_CreateObjCommand` API on every release.

A representation snapshot does not reproduce these object effects. A bridge
must retain original object identity, deduplicate argument aliases, publish
cache changes even when conversion fails, and preserve exact resident bytes.
The shim's original-object callback entry uses owned mutation capabilities.
List and Dictionary representations retain original member capabilities, so
child getter effects and returned object aliases reach their original objects.
Index-table and opaque-cache operations require explicit lifetime/origin
capabilities; the shim refuses those operations without them.
The shim does not expose `Tcl_EvalObjv` to C callbacks. Native nested-dispatch
observations describe that separate execution requirement, not shim support.
