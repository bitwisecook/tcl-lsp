# Tcl counted-source execution evidence

## Question

> A missing info selector and an explicit empty selector enter different public diagnostic paths. C8.4 option dispatch, the other selected C releases and Jim require their own original usage and selector-list messages; a generic ensemble renderer can erase these distinctions.

> What exact public completion and diagnostic do the two original info and info {} source commands publish on each of the six pinned interpreters?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Two unchanged ASCII LF originals retain their own explicit catch/list witnesses; the counted-source driver adds no original source wrapper. Each source is evaluated in a fresh fully initialised selected interpreter. Both commands use genuinely available info APIs on all six providers, with no unavailable substitute. Original outer completion, caught command completion and whole result remain independent fields. Complete request/source/driver/compiler/ELF/process/stdout/stderr/version and required provider pins are retained. The separate C8.4 whole source and exact excerpt are read-only evidence, not a private native observation. No current command binding, opaque AST, compiler/instruction admission, object/header/cache/frame identity or arbitrary selector behavior follows from these finite outputs. The Registry software selectors keep C8.4 option lookup and C release-specific missing-selector usage separate from ensemble and Jim dispatch purposes. Both Runtime and VM controls compare all twelve whole unchanged Native292 original completion/results. A pure selected-protocol control checks C8.4 option versus C8.5 argument versus C8.6+ arg usage and unavailable Jim C protocol, independently of the measured public messages. Those software definitions add no private native worker, original header/table/cache or current executed Rust result. Independently retained exact Jim source registers InfoCore with minargs=1 and usage subcommand ?arg ...?. JimCallNative checks that registered worker arity before entering Jim_InfoCoreCommand/Jim_ParseSubCmd, so bare input and an empty selector have distinct selected mechanisms. JimCmdUsage provides its own final formatter boundary. The renamed and NUL head examples in the marked shared recipe are software renderer controls, not additional executed Native originals. Both linked backend definitions compare the same twelve complete original public windows without a current software pass claim; source interpretation supplies no private worker/header/table identity or generic ensemble execution result.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set r2286_code [catch {info {}} r2286_message]
list $r2286_code $r2286_message
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|31207b616d626967756f7573206f7074696f6e2022223a206d75737420626520617267732c20626f64792c20636d64636f756e742c20636f6d6d616e64732c20636f6d706c6574652c2064656661756c742c206578697374732c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {ambiguous option "": must be args, body, cmdcount, commands, complete, default, exists, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|31207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022223a206d75737420626520617267732c20626f64792c20636d64636f756e742c20636f6d6d616e64732c20636f6d706c6574652c2064656661756c742c206578697374732c206672616d652c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {unknown or ambiguous subcommand "": must be args, body, cmdcount, commands, complete, default, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|31207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022223a206d75737420626520617267732c20626f64792c20636c6173732c20636d64636f756e742c20636f6d6d616e64732c20636f6d706c6574652c20636f726f7574696e652c2064656661756c742c206572726f72737461636b2c206578697374732c206672616d652c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c206f626a6563742c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {unknown or ambiguous subcommand "": must be args, body, class, cmdcount, commands, complete, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|31207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022223a206d75737420626520617267732c20626f64792c20636c6173732c20636d64636f756e742c20636d64747970652c20636f6d6d616e64732c20636f6d706c6574652c20636f6e7374616e742c20636f6e7374732c20636f726f7574696e652c2064656661756c742c206572726f72737461636b2c206578697374732c206672616d652c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c206f626a6563742c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {unknown or ambiguous subcommand "": must be args, body, class, cmdcount, cmdtype, commands, complete, constant, consts, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|31207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022223a206d75737420626520617267732c20626f64792c20636c6173732c20636d64636f756e742c20636d64747970652c20636f6d6d616e64732c20636f6d706c6574652c20636f6e7374616e742c20636f6e7374732c20636f726f7574696e652c2064656661756c742c206572726f72737461636b2c206578697374732c206672616d652c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c206f626a6563742c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {unknown or ambiguous subcommand "": must be args, body, class, cmdcount, cmdtype, commands, complete, constant, consts, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|31207b696e666f2c20616d626967756f757320636f6d6d616e642022223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {info, ambiguous command "": should be alias, aliases, args, body, channels, commands, complete, exists, frame, globals, help, hostname, level, locals, nameofexecutable, patchlevel, procs, references, returncodes, script, source, stacktrace, statics, tainted, usage, vars, version}⟦EOF⟧
```
