# Native formal storage

The 132 observations distinguish parsed formal storage from introspection and
variable lookup. C8.4/C8.5 split both parameter-list levels through CString input;
C8.6+ and Jim retain complete parsed formal bytes. C introspection enumerates a
CString formal spelling independently of that storage. The probe compares raw
String NUL, native source modified NUL, and pure C ByteArray materialization via
compiled full-name reads, short-name reads and dynamic full-name reads.

Build C probes against each pinned release, for example:

```sh
cc -Itmp/tcl9.0.4/unix -Itmp/tcl9.0.4/generic rust/tcl-syntax/tests/data/native_formal_storage/probe.c tmp/tcl9.0.4/unix/libtcl9.0.a -lm -ldl -lpthread -lz -o /tmp/native-formals
/tmp/native-formals
```

Jim uses `-DUSE_JIM` and its pinned headers/library with static extensions enabled.
`manifest.json` records the exact build commands, source/library hashes and all
observations. Each release JSONL file contains byte-exact expected output.
The reporter captures the result and completion options before inspecting
error state. C8.4 reads existing physical namespace variables without invoking
guest lookups or traces; C8.5+ reads the captured return-option dictionary.

`default-object-probe.c` constructs an optional formal with an original Double
default, defines a procedure, and compares the bound value's object identity and
cache at the native callback. C8.4/C8.5 reconstruct a String default through their
CString list parser; C8.6/C9/Jim retain the original Double object. The six
`default-object-*.jsonl` observations and `default-object-manifest.json` retain
the source and native library hashes. Build with the same C/Jim commands above,
substituting `default-object-probe.c` as the source.
