# Original formal slots and local enumeration

The two maintained C/Jim probes use original List member objects as procedure
parameters, original counted body bytes, and observer-safe completion capture.
`compiled-local-names.c` records 216 observations across five C releases and
pinned Jim. `compiled-local-inventory.c` records 240 observations across the
same engines. Manifests retain source/header/library/log hashes and exact build
commands; `USE_JIM` selects the pinned Jim API.

C binds each formal declaration into its own compiled cell. Duplicate names
resolve dynamically to the first declared cell and appear separately in
`info locals`/`info vars`. Its compiled name comparator is distinct from dynamic
lookup: equal-length counted names that differ after raw NUL can select the
same compiled index while preserving distinct dynamic cells. Jim has named
locals, binds a duplicate name last, and enumerates it once. C8.4/8.5's parameter
stringification uses a CString parser; C8.6+ and Jim retain counted bytes.

`formal_slot_identity.rs` tests duplicate binding, enumeration and the
compiled-versus-dynamic raw-NUL distinction through actual VM procedure entry.
Native observation files are unchanged. Completion options are captured before
any public error-global lookup; object/name identity checks are independent of
Unicode display.

To reproduce, use a manifest command with local native source/library paths.
For a C9 example:

```sh
cc -I /path/tcl9.0.4/unix -I /path/tcl9.0.4/generic \
  compiled-local-names.c /path/tcl9.0.4/unix/libtcl9.0.a \
  -lm -ldl -lpthread -lz -o names
./names
```
