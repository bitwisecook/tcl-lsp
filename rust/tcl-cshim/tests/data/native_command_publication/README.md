# Native C command publication

`ordering.c` exercises `Tcl_CreateObjCommand` and `Tcl_DeleteCommand` with a delete callback that inspects the original command and may create a replacement at the same slot. The expected files cover C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. The manifest records the source and native library digests.

A replacement callback sees the original command. An outer creation replaces a callback-created command; an outer deletion leaves a callback-created replacement in place. These controls require original-token guards, rather than an unconditional removal after the callback.

Build against each pinned native tree, using its `tclConfig.sh` library flags:

```sh
cc -I "$TCL_NATIVE_TREE/generic" -I "$TCL_NATIVE_TREE/unix" ordering.c "$TCL_NATIVE_LIBRARY" -ldl -lpthread -lm -lz -o native-command-ordering
./native-command-ordering > actual.txt
diff -u "c$TCL_NATIVE_VERSION.txt" actual.txt
```

The C API has a separate name address protocol: an unqualified `Tcl_CreateObjCommand` name is global, while a visibly qualified relative name uses the current namespace and can create missing namespaces. The script command-publication protocol does not supply that rule. Resident bytes and C-string termination are independent: modified NUL (`c0 80`) remains part of a name, whereas raw NUL ends a C-string input.

`scope.c` selects the C API publication address inside the actual namespace `::a:`. Every audited C release publishes unqualified `p` globally. Relative-qualified `q::p` uses the physical `a:` namespace and its child `q`, so local `q::p` resolves but neither global display spelling resolves to that slot. This differs from the C8.4/C8.5 script `proc` publication protocol. Build `scope.c` with the same native command and compare its output with the release’s `.scope.txt` file. C8.4’s exact exported namespace/query declarations are taken from its pinned internal header.
