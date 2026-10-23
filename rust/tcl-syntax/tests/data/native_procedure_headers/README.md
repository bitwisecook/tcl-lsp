# Native byte procedure headers

The probe inspects the actual `Command.compileProc` pointer after direct
procedure definition. It covers 40 counted C observations, preserving raw NUL
and `ff` parameter/body bytes. The byte formal-storage grammar and the header
compiler's CString parameter comparison are distinct operations.

```sh
native_tcl_root="$PWD/tmp/tcl8.6.18"
native_tcl_version=8.6
native_header_build=$(mktemp -d)
cc -std=c11 -D_GNU_SOURCE -DHAVE_UNISTD_H \
    -I"$native_tcl_root/generic" -I"$native_tcl_root/unix" \
    rust/tcl-syntax/tests/data/native_procedure_headers/probe.c \
    "$native_tcl_root/unix/libtcl$native_tcl_version.a" \
    -lpthread -ldl -lm -lz -o "$native_header_build/probe"
"$native_header_build/probe" > "$native_header_build/8.6.18.txt"
diff -u rust/tcl-syntax/tests/data/native_procedure_headers/8.6.18.txt \
    "$native_header_build/8.6.18.txt"
```

The internal header and linked library must come from the same pinned build.
C8.4 tests the body CString prefix; later C releases use the complete counted
body. Neither a header pointer nor this recipe supplies command-token identity,
object callback closure, accepted procedure arguments or a body execution proof.
