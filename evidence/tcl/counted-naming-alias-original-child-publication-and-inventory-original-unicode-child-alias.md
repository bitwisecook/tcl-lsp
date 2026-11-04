# Tcl counted-source execution evidence

## Question

> Child-alias publication, counted invocation and public command/alias inventories use distinct provider API paths. A child declaration cannot borrow the standalone core-alias name storage or inventory purpose; rooted spelling, produced NUL/FF bytes and literal Unicode need their own original controls.

> For each unchanged original child-alias source, which name is published, callable and independently reported by command and alias inventories on the six authentic providers, including rooted spelling and counted binary/Unicode boundaries?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Six unchanged LF originals use genuinely available C interp alias versus Jim child-handle alias APIs in fresh fully initialized interpreters. Five sources are ASCII; one contains literal UTF-8 U+00E9. NUL and FF are produced by binary format H* and are absent as literal source bytes. NAME_BEFORE, caught definition, counted call, command inventory, selected alias inventory and opposite-surface failure are independent original source fields. C inventory member hex uses explicit encoding convertto utf-8; Jim uses raw binary scan. Whole result order, original character lengths, hex and every caught message are retained without normalisation or a hash-order contract. The counted-source driver adds no original script envelope. Complete actual request/driver/source/compiler/ELF/process/version/stdout/stderr and required pins are preserved. Exact Jim child installer/macro and command registration source excerpts are separate read-only evidence; public empty aliases does not identify a private class or table. BIG-IP is not tested, and no general alias, child bootstrap, header/cache/frame/compiler admission or runtime result follows beyond these fixed controls. The shared ChildAliasPublication source projection retains the complete counted original input, selected C-string registration extent and independently selected alias report. C alias registration reports retain original counted naming units; Jim child reports use its actual C-string registration extent, separately from standalone counted AliasPublication. These pure projections and actual Runtime/VM parent/child owner adapters grant no original native header, prefix pointer, current table or entered frame. The linked Runtime and VM controls each compare all thirty-six whole original public completion/results, including opposite-API failure and counted-call behavior; their definitions establish no executed software pass. Missing/foreign issuer controls are independently authored host assertions and supply no Native failure observation. The independently pinned Jim_GetCommand source retains its exact invalid-command producer: on original lookup failure it passes the original object to Jim_SetResultFormatted with the %#s conversion. This source interpretation remains separate from the fixed produced-NUL Native298 diagnostic rows and from command registration/inventory extents. Runtime invalid_command_result and VM command_lookup_error_value consume the shared CommandLookupError report purpose under the actual engine policy rather than a local registration-name projection. The same unchanged thirty-six-window comparator definition supplies no current executed pass; independently authored child refusal state is not a Native failure observation. Exact Jim_SetResultFormatted source obtains the counted object string length for allocation, then uses final snprintf string formatting and its returned length for the result object. These source facts describe that producer/formatter boundary only; they establish no new executed NUL specimen, original header identity or software pass.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set r2286_is_jim [string match 0.* [info patchlevel]]
if {$r2286_is_jim} {set r2286_child [interp]} else {set r2286_child [interp create]}
set r2286_name {r2286_é}
set r2286_name_before_length [string length $r2286_name]
binary scan $r2286_name H* r2286_name_before_hex
if {$r2286_is_jim} {
    set r2286_definition_code [catch {$r2286_child alias $r2286_name list VALUE} r2286_definition_result]
    set r2286_call_code [catch {$r2286_child eval [list $r2286_name]} r2286_call_result]
    set r2286_command_code [catch {$r2286_child eval {info commands -all *r2286*}} r2286_command_result]
    set r2286_alias_code [catch {$r2286_child eval {info aliases}} r2286_alias_result]
    set r2286_other_alias_code [catch {interp aliases $r2286_child} r2286_other_alias_result]
} else {
    set r2286_definition_code [catch {interp alias $r2286_child $r2286_name {} list VALUE} r2286_definition_result]
    set r2286_call_code [catch {interp eval $r2286_child [list $r2286_name]} r2286_call_result]
    set r2286_command_code [catch {interp eval $r2286_child {info commands *r2286*}} r2286_command_result]
    set r2286_alias_code [catch {interp aliases $r2286_child} r2286_alias_result]
    set r2286_other_alias_code [catch {interp eval $r2286_child {info aliases}} r2286_other_alias_result]
}
set r2286_command_members {}
if {$r2286_command_code == 0} {
foreach r2286_member $r2286_command_result {
    if {$r2286_is_jim} {binary scan $r2286_member H* r2286_hex} else {
        binary scan [encoding convertto utf-8 $r2286_member] H* r2286_hex
    }
    lappend r2286_command_members [list [string length $r2286_member] $r2286_hex]
}
}
set r2286_alias_members {}
if {$r2286_alias_code == 0} {
foreach r2286_member $r2286_alias_result {
    if {$r2286_is_jim} {binary scan $r2286_member H* r2286_hex} else {
        binary scan [encoding convertto utf-8 $r2286_member] H* r2286_hex
    }
    lappend r2286_alias_members [list [string length $r2286_member] $r2286_hex]
}
}
if {$r2286_is_jim} {$r2286_child delete} else {interp delete $r2286_child}
list [list NAME_BEFORE $r2286_name_before_length $r2286_name_before_hex] [list DEFINITION $r2286_definition_code $r2286_definition_result] [list CALL $r2286_call_code $r2286_call_result] [list COMMANDS $r2286_command_code $r2286_command_result $r2286_command_members] [list ALIASES $r2286_alias_code $r2286_alias_result $r2286_alias_members] [list OTHER_ALIAS_SURFACE $r2286_other_alias_code $r2286_other_alias_result]
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|7b4e414d455f4245464f524520372037323332333233383336356665397d207b444546494e4954494f4e20302072323238365fc3a97d207b43414c4c20302056414c55457d207b434f4d4d414e445320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b414c494153455320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b4f544845525f414c4941535f535552464143452031207b626164206f7074696f6e2022616c6961736573223a206d75737420626520617267732c20626f64792c20636d64636f756e742c20636f6d6d616e64732c20636f6d706c6574652c2064656661756c742c206578697374732c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{NAME_BEFORE 7 72323238365fe9} {DEFINITION 0 r2286_é} {CALL 0 VALUE} {COMMANDS 0 r2286_é {{7 72323238365fc3a9}}} {ALIASES 0 r2286_é {{7 72323238365fc3a9}}} {OTHER_ALIAS_SURFACE 1 {bad option "aliases": must be args, body, cmdcount, commands, complete, default, exists, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars}}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|7b4e414d455f4245464f524520372037323332333233383336356665397d207b444546494e4954494f4e20302072323238365fc3a97d207b43414c4c20302056414c55457d207b434f4d4d414e445320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b414c494153455320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b4f544845525f414c4941535f535552464143452031207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022616c6961736573223a206d75737420626520617267732c20626f64792c20636d64636f756e742c20636f6d6d616e64732c20636f6d706c6574652c2064656661756c742c206578697374732c206672616d652c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{NAME_BEFORE 7 72323238365fe9} {DEFINITION 0 r2286_é} {CALL 0 VALUE} {COMMANDS 0 r2286_é {{7 72323238365fc3a9}}} {ALIASES 0 r2286_é {{7 72323238365fc3a9}}} {OTHER_ALIAS_SURFACE 1 {unknown or ambiguous subcommand "aliases": must be args, body, cmdcount, commands, complete, default, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, patchlevel, procs, script, sharedlibextension, tclversion, or vars}}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|7b4e414d455f4245464f524520372037323332333233383336356665397d207b444546494e4954494f4e20302072323238365fc3a97d207b43414c4c20302056414c55457d207b434f4d4d414e445320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b414c494153455320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b4f544845525f414c4941535f535552464143452031207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022616c6961736573223a206d75737420626520617267732c20626f64792c20636c6173732c20636d64636f756e742c20636f6d6d616e64732c20636f6d706c6574652c20636f726f7574696e652c2064656661756c742c206572726f72737461636b2c206578697374732c206672616d652c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c206f626a6563742c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{NAME_BEFORE 7 72323238365fe9} {DEFINITION 0 r2286_é} {CALL 0 VALUE} {COMMANDS 0 r2286_é {{7 72323238365fc3a9}}} {ALIASES 0 r2286_é {{7 72323238365fc3a9}}} {OTHER_ALIAS_SURFACE 1 {unknown or ambiguous subcommand "aliases": must be args, body, class, cmdcount, commands, complete, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars}}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|7b4e414d455f4245464f524520372037323332333233383336356665397d207b444546494e4954494f4e20302072323238365fc3a97d207b43414c4c20302056414c55457d207b434f4d4d414e445320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b414c494153455320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b4f544845525f414c4941535f535552464143452031207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022616c6961736573223a206d75737420626520617267732c20626f64792c20636c6173732c20636d64636f756e742c20636d64747970652c20636f6d6d616e64732c20636f6d706c6574652c20636f6e7374616e742c20636f6e7374732c20636f726f7574696e652c2064656661756c742c206572726f72737461636b2c206578697374732c206672616d652c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c206f626a6563742c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{NAME_BEFORE 7 72323238365fe9} {DEFINITION 0 r2286_é} {CALL 0 VALUE} {COMMANDS 0 r2286_é {{7 72323238365fc3a9}}} {ALIASES 0 r2286_é {{7 72323238365fc3a9}}} {OTHER_ALIAS_SURFACE 1 {unknown or ambiguous subcommand "aliases": must be args, body, class, cmdcount, cmdtype, commands, complete, constant, consts, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars}}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|7b4e414d455f4245464f524520372037323332333233383336356665397d207b444546494e4954494f4e20302072323238365fc3a97d207b43414c4c20302056414c55457d207b434f4d4d414e445320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b414c494153455320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b4f544845525f414c4941535f535552464143452031207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022616c6961736573223a206d75737420626520617267732c20626f64792c20636c6173732c20636d64636f756e742c20636d64747970652c20636f6d6d616e64732c20636f6d706c6574652c20636f6e7374616e742c20636f6e7374732c20636f726f7574696e652c2064656661756c742c206572726f72737461636b2c206578697374732c206672616d652c2066756e6374696f6e732c20676c6f62616c732c20686f73746e616d652c206c6576656c2c206c6962726172792c206c6f616465642c206c6f63616c732c206e616d656f6665786563757461626c652c206f626a6563742c2070617463686c6576656c2c2070726f63732c207363726970742c207368617265646c6962657874656e73696f6e2c2074636c76657273696f6e2c206f7220766172737d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{NAME_BEFORE 7 72323238365fe9} {DEFINITION 0 r2286_é} {CALL 0 VALUE} {COMMANDS 0 r2286_é {{7 72323238365fc3a9}}} {ALIASES 0 r2286_é {{7 72323238365fc3a9}}} {OTHER_ALIAS_SURFACE 1 {unknown or ambiguous subcommand "aliases": must be args, body, class, cmdcount, cmdtype, commands, complete, constant, consts, coroutine, default, errorstack, exists, frame, functions, globals, hostname, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars}}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|7b4e414d455f4245464f5245203720373233323332333833363566633361397d207b444546494e4954494f4e2030207b7d7d207b43414c4c20302056414c55457d207b434f4d4d414e445320302072323238365fc3a9207b7b3720373233323332333833363566633361397d7d7d207b414c49415345532030207b7d207b7d7d207b4f544845525f414c4941535f535552464143452031207b77726f6e67202320617267733a2073686f756c642062652022696e74657270227d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{NAME_BEFORE 7 72323238365fc3a9} {DEFINITION 0 {}} {CALL 0 VALUE} {COMMANDS 0 r2286_é {{7 72323238365fc3a9}}} {ALIASES 0 {} {}} {OTHER_ALIAS_SURFACE 1 {wrong # args: should be "interp"}}⟦EOF⟧
```
