# Native callback completion and object identity

`probe.c` calls a real C command procedure with codes 0, 1, 2, 3, 4 and 7,
with and without a private error-code value. It retains the result, captures
`Tcl_GetReturnOptions` before inspecting its fields, and emits length-delimited
hex bytes. The five release logs contain 60 observations. Tcl 8.4 has no
`Tcl_GetReturnOptions` API; that log establishes only its code and result.

`alias.c` constructs a List with two references to one fresh child. All five
C releases retain both child identities and the same interpreter result.
C9 retains that same List as `-errorinfo`; C8.5 and C8.6 copy its string.
The shim models its C9 ABI and does not infer older-release object identity.

Both manifests retain the exact commands and source, header, library, binary
and observation hashes (the alias manifest retains source/header/library/log
hashes). Build against a selected native tree, replacing the release paths:

```sh
cc -std=c99 -I /path/tcl9.0.4/generic -I /path/tcl9.0.4/unix \
  probe.c /path/tcl9.0.4/unix/libtcl9.0.a -lpthread -ldl -lm -lz -o probe
./probe
cc -std=c99 -I /path/tcl9.0.4/generic -I /path/tcl9.0.4/unix \
  alias.c /path/tcl9.0.4/unix/libtcl9.0.a -lpthread -ldl -lm -lz -o alias
./alias
```

The native procedure completion differs from evaluation of an entire script:
procedure and loop boundaries interpret Return, Break and Continue. The
engine callback completion carries the raw native code and original options;
the actual executing VM boundary performs its own interpretation.
