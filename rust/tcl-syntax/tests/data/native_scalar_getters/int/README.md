# Native C GetInt

`probe.c` observes the primitive `Tcl_GetIntFromObj` stage on the original object,
including its int cache, signed 32-bit return, and private seeded error-code state.
This getter differs from Wide, expression arithmetic, and Jim Long. The 70 rows
cover raw NUL, integer width boundaries, fresh floating spellings, and cached
Double/NaN/Int. Hex columns preserve native messages and error-code lists.

Build against each pinned C tree (substitute the release and library basename):

```sh
cc -Itmp/tcl8.6.18/generic rust/tcl-syntax/tests/data/native_scalar_getters/int/probe.c tmp/tcl8.6.18/unix/libtcl8.6.a -lm -ldl -lpthread -lz -o /tmp/native-int
/tmp/native-int
```

The source/library hashes and original JSON observations are in `manifest.json`.
The Syntax regression compares selected conversions and primitive error updates
with these observations, including cache changes before failures.
