# Original native frame selectors

The five counted transcripts contain 80 actual C Tcl observations. Each row
retains the same original level object across `uplevel`: raw String, embedded
NUL, native WideInt, or Double. The probe records the object cache before and
after execution plus the result bytes; it does not inspect guest error globals.
The two procedure activations establish current level 2.

Build a pinned Tcl Unix static library, then run this fixture from the repository:

```sh
native_tcl_root="$PWD/tmp/tcl8.6.18"
native_tcl_version=8.6
native_frame_build=$(mktemp -d)
cc -std=c11 -I"$native_tcl_root/generic" -I"$native_tcl_root/unix" \
    rust/tcl-syntax/tests/data/native_frame_levels/probe.c \
    "$native_tcl_root/unix/libtcl$native_tcl_version.a" \
    -lpthread -ldl -lm -lz -o "$native_frame_build/probe"
"$native_frame_build/probe" > "$native_frame_build/8.6.18.jsonl"
diff -u rust/tcl-syntax/tests/data/native_frame_levels/8.6.18.jsonl \
    "$native_frame_build/8.6.18.jsonl"
```

C8.4/C8.5 digit-first selectors use a temporary CString Int conversion;
C8.5 retains signed relative frame references even on a failed lookup.
C8.6+ probes the original object before string access. C8.6 numeric parsing can
cache Double even when the Int probe fails. C9 validates the unwrapped original
wide magnitude and excludes negative numeric selectors from the script branch.
An absolute frame cache is separate from Numeric and requires its original
resident spelling. These receipts do not select a source grammar, prove a
physical frame exists outside the observed activation, or close object hooks.

Jim's 21 counted observations use `Jim_String` at the command-level first-byte
probe, then original `Jim_GetLong` for a digit-leading relative level. A hash
suffix uses signed `jim_strtol`, exact CString end-pointer validation and no
object numeric cache. The current frame is also 2. Build/run the pinned
math-enabled Jim library with its configured link dependencies:

```sh
native_jim_root=/path/to/pinned/jimtcl
cc -std=c11 -I"$native_jim_root" \
    rust/tcl-syntax/tests/data/native_frame_levels/jim-probe.c \
    "$native_jim_root/libjim.a" -lm -ldl -lssl -lcrypto -lz -o /tmp/native-jim-frame
/tmp/native-jim-frame > /tmp/native-jim-frame.jsonl
diff -u rust/tcl-syntax/tests/data/native_frame_levels/jim0.84.jsonl \
    /tmp/native-jim-frame.jsonl
```
