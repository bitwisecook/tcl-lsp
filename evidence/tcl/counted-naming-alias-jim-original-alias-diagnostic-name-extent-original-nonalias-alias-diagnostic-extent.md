# Tcl counted-source execution evidence

## Question

> Counted command-name values and public alias diagnostic text can retain different byte extents. Missing-command lookup and a successful original procedure lookup followed by a not-an-alias error require separate observations, including names produced at evaluation with NUL, UTF-8 and invalid bytes.

> What exact public name lengths and missing-command/not-an-alias diagnostic bytes do two original Jim info-alias sources return for middle-NUL, leading-NUL, UTF-8 and invalid-byte names?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Two unchanged ASCII LF originals with four name cases each. binary format H* produces NUL, UTF-8 and FF bytes at evaluation; there is no literal NUL/non-ASCII source. Each original executes independently in a fresh fully initialised provider through the unchanged counted-source CAPI driver. No private object/header/key/lookup identity, physical cache, arbitrary error producer, Rust assertion or BIG-IP result is observed. The body does not replay external processes, inspect private slots/headers, infer a full Jim distribution or validate a source-character channel from binary-produced bytes. The exact original sources and C NOT_APPLICABLE/Jim result boundaries remain intact. Binary-produced bytes do not establish physical source-character ingress, a full Jim distribution, header/cache identity or private lookup extent. C unavailable branches remain independent original results, without substituting another alias API.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {[catch {info -commands} r2286_selectors] || [lsearch -exact $r2286_selectors alias] < 0} {
    set r2286_result NOT_APPLICABLE
} else {
    set rows {}
    set name [binary format H* 72323238365f657874656e74005441494c]
    proc $name {} {return ORIGINAL_PROC}
    set code [catch {info alias $name} message]
    lappend rows [list nul-middle [string length $name] $code $message]
    set name [binary format H* 0072323238365f657874656e74]
    proc $name {} {return ORIGINAL_PROC}
    set code [catch {info alias $name} message]
    lappend rows [list nul-leading [string length $name] $code $message]
    set name [binary format H* 72323238365fc3a9]
    proc $name {} {return ORIGINAL_PROC}
    set code [catch {info alias $name} message]
    lappend rows [list utf8 [string length $name] $code $message]
    set name [binary format H* 72323238365fff]
    proc $name {} {return ORIGINAL_PROC}
    set code [catch {info alias $name} message]
    lappend rows [list invalid-byte [string length $name] $code $message]
    set rows
}
```

## Results

### jim, run 1

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|7b6e756c2d6d6964646c652031372031207b636f6d6d616e64202272323238365f657874656e7422206973206e6f7420616e20616c6961737d7d207b6e756c2d6c656164696e672031332031207b636f6d6d616e64202222206973206e6f7420616e20616c6961737d7d207b7574663820372031207b636f6d6d616e64202272323238365fc3a922206973206e6f7420616e20616c6961737d7d207b696e76616c69642d6279746520372031207b636f6d6d616e64202272323238365fff22206973206e6f7420616e20616c6961737d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{nul-middle 17 1 {command "r2286_extent" is not an alias}} {nul-leading 13 1 {command "" is not an alias}} {utf8 7 1 {command "r2286_é" is not an alias}} {invalid-byte 7 1 {command "r2286_⟦BYTE:FF⟧" is not an alias}}⟦EOF⟧
```
