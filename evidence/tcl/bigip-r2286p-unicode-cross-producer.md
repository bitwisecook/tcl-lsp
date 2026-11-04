# BIG-IP cross-context naming: unicode cross producer

## Question

Do variable names produced by format %c and by binary format reach the same or different cells for the tested character values? This affects the LSP, compiler and VM's treatment of name producers.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

```tcl-visible
set prefix __tcl_lsp_2286_r2286p_unicodecross_
set format_pre $prefix
append format_pre [format %c 233]
set bytes_pre $prefix
append bytes_pre [binary format H* c3a9]
set format_decomposed $prefix
append format_decomposed e [format %c 769]
set bytes_decomposed $prefix
append bytes_decomposed [binary format H* 65cc81]
set rows {}
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {
    set label [lindex $pair 0]
    set name [lindex $pair 1]
    binary scan $name H* raw
    lappend rows [list representation $label [string length $name] $raw]
}
foreach pair [list [list $format_pre FORMAT_PRE] [list $bytes_pre BYTES_PRE] [list $format_decomposed FORMAT_DECOMPOSED] [list $bytes_decomposed BYTES_DECOMPOSED]] {
    set rc [catch {set [lindex $pair 0] [lindex $pair 1]} result]
    lappend rows [list write [lindex $pair 1] $rc $result]
}
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {
    set rc [catch {set [lindex $pair 1]} result]
    lappend rows [list read [lindex $pair 0] $rc $result]
}
set rc [catch {upvar 0 $bytes_pre pre_alias; set pre_alias BYTES_PRE_MUTATION} result]
lappend rows [list mutate_bytes_pre $rc $result]
set rc [catch {upvar 0 $bytes_decomposed decomposed_alias; set decomposed_alias BYTES_DECOMPOSED_MUTATION} result]
lappend rows [list mutate_bytes_decomposed $rc $result]
foreach pair [list [list format_pre $format_pre] [list bytes_pre $bytes_pre] [list format_decomposed $format_decomposed] [list bytes_decomposed $bytes_decomposed]] {
    set rc [catch {set [lindex $pair 1]} result]
    lappend rows [list after_mutation [lindex $pair 0] $rc $result]
}
set rc [catch {unset $format_pre} result]
lappend rows [list unset_format_pre $rc $result]
set rc [catch {unset $format_decomposed} result]
lappend rows [list unset_format_decomposed $rc $result]
foreach pair [list [list bytes_pre $bytes_pre] [list bytes_decomposed $bytes_decomposed]] {
    set rc [catch {set [lindex $pair 1]} result]
    lappend rows [list after_format_unset [lindex $pair 0] $rc $result]
}
foreach name [list $format_pre $bytes_pre $format_decomposed $bytes_decomposed] {catch {unset $name}}
set rows
```

## Results

### TMM 0:0, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{representation format_pre 36 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:1, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{representation format_pre 36 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{representation format_pre 36 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{representation format_pre 36 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### IAppImplementation

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: IAppImplementation.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{representation format_pre 36 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.

### ICallScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: ICallScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{representation format_pre 36 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.

### TmshCliScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TmshCliScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{representation format_pre 36 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f7232323836705f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
