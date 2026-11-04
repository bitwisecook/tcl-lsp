# BIG-IP cross-context naming: nul plain unset first

## Question

When the shorter variable name is unset before the same prefix followed by an embedded NUL, does the counted name remain a separate cell? This affects variable-name resolution in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

```tcl-visible
set prefix __tcl_lsp_2286_r2286p_nul_
set plain ${prefix}A
set counted $plain
append counted [binary format H* 0042]
set rows [list [list names [string length $plain] [string length $counted]]]
foreach pair [list [list $plain PLAIN] [list $counted COUNTED]] {
    set name [lindex $pair 0]
    set value [lindex $pair 1]
    set rc [catch {set $name $value} result]
    lappend rows [list write [string length $name] $rc $result]
}
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list read [string length $name] $rc $result]
}
set rc [catch {upvar 0 $counted ${prefix}link; set ${prefix}link LINK_WRITE} result]
lappend rows [list upvar_write $rc $result]
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list after_link [string length $name] $rc $result]
}
set rc [catch {unset $plain} result]
lappend rows [list unset_plain $rc $result]
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list after_unset [string length $name] $rc $result]
}
foreach name [list $plain $counted ${prefix}link] {catch {unset $name}}
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
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_plain 0 {}} {after_unset 27 1 {can't read "__tcl_lsp_2286_r2286p_nul_A": no such variable}} {after_unset 29 0 LINK_WRITE}⟦EOF⟧
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
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_plain 0 {}} {after_unset 27 1 {can't read "__tcl_lsp_2286_r2286p_nul_A": no such variable}} {after_unset 29 0 LINK_WRITE}⟦EOF⟧
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
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_plain 0 {}} {after_unset 27 1 {can't read "__tcl_lsp_2286_r2286p_nul_A": no such variable}} {after_unset 29 0 LINK_WRITE}⟦EOF⟧
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
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_plain 0 {}} {after_unset 27 1 {can't read "__tcl_lsp_2286_r2286p_nul_A": no such variable}} {after_unset 29 0 LINK_WRITE}⟦EOF⟧
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
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_plain 0 {}} {after_unset 27 1 {can't read "__tcl_lsp_2286_r2286p_nul_A": no such variable}} {after_unset 29 0 LINK_WRITE}⟦EOF⟧
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
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_plain 0 {}} {after_unset 27 1 {can't read "__tcl_lsp_2286_r2286p_nul_A": no such variable}} {after_unset 29 0 LINK_WRITE}⟦EOF⟧
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
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_plain 0 {}} {after_unset 27 1 {can't read "__tcl_lsp_2286_r2286p_nul_A": no such variable}} {after_unset 29 0 LINK_WRITE}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
