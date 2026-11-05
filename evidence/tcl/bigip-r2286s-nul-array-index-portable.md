# BIG-IP corrected naming: nul array index portable

## Question

Does an embedded NUL distinguish an array index from its shorter prefix through writes, reads, links, unsets and supported trace callbacks across BIG-IP Tcl contexts? This affects array-element modeling in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. The result is scoped to the named context and inputs; it does not establish behavior in another context or version. Other assumptions were not separately recorded.

## Script

```tcl-visible
set trace_rows {}
set prefix __tcl_lsp_2286_r2286s_arrayindex_
set root $prefix
append root root
set plain A
set counted $plain
append counted [binary format H* 0042]
set plain_var $root
append plain_var ( $plain )
set counted_var $root
append counted_var ( $counted )
set procname ::__tcl_lsp_2286_r2286s_nul_array_index_trace
set rows [list [list indexes [string length $plain] [string length $counted]]]
set callback_body {upvar 1 trace_rows trace_rows; binary scan $name1 H* root_hex; binary scan $name2 H* index_hex; lappend trace_rows [list $root_hex $index_hex $op]}
catch {eval [list rename $procname {}]}
set rc [catch {eval [list proc $procname {name1 name2 op} $callback_body]} result]
lappend rows [list trace_proc $rc $result]
set rc [catch {trace variable $root rwu $procname} result]
lappend rows [list trace_add $rc $result]
foreach item [list [list $plain_var PLAIN] [list $counted_var COUNTED]] {
    set varname [lindex $item 0]
    set rc [catch {set $varname [lindex $item 1]} result]
    lappend rows [list write [string length $varname] $rc $result]
}
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set label [lindex $item 0]
    set varname [lindex $item 1]
    lappend rows [list exists $label [info exists $varname]]
    set rc [catch {set $varname} result]
    lappend rows [list read $label $rc $result]
}
set rc [catch {upvar 0 $counted_var alias; set alias COUNTED_MUTATION} result]
lappend rows [list upvar_write $rc $result]
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set rc [catch {set [lindex $item 1]} result]
    lappend rows [list after_link [lindex $item 0] $rc $result]
}
set rc [catch {unset $counted_var} result]
lappend rows [list unset_counted_index $rc $result]
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set varname [lindex $item 1]
    lappend rows [list after_unset_exists [lindex $item 0] [info exists $varname]]
    set rc [catch {set $varname} result]
    lappend rows [list after_unset_read [lindex $item 0] $rc $result]
}
catch {trace vdelete $root rwu $procname}
catch {unset $root}
catch {eval [list rename $procname {}]}
lappend rows [list trace_rows $trace_rows]
set rows
```

## Results

The initial HTTP collection had 2 transport failures with no recorded Tcl result or TMM unit:

```text-visible
[Errno 98] Address already in use⟦EOF⟧
```

### TMM 0:0, HTTP_REQUEST, initial; TMM 0:3, HTTP_REQUEST, initial

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST, initial; TMM 0:3, HTTP_REQUEST, initial.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286s_arrayindex_root(A⟦U+0000⟧B)": no such element in array}} {trace_rows {{5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 u} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r}}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 31 responses with status HTTP/1.0 200 OK.

### TMM 0:1, HTTP_REQUEST, initial; TMM 0:2, HTTP_REQUEST, initial; TMM 0:0, HTTP_REQUEST, rerun; TMM 0:1, HTTP_REQUEST, rerun; TMM 0:2, HTTP_REQUEST, rerun; TMM 0:3, HTTP_REQUEST, rerun

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST, initial; TMM 0:2, HTTP_REQUEST, initial; TMM 0:0, HTTP_REQUEST, rerun; TMM 0:1, HTTP_REQUEST, rerun; TMM 0:2, HTTP_REQUEST, rerun; TMM 0:3, HTTP_REQUEST, rerun.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286s_arrayindex_root(A⟦U+0000⟧B)": no such element in array}} {trace_rows {{5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 u} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r}}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### IAppImplementation; ICallScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: IAppImplementation; ICallScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286s_arrayindex_root(A⟦U+0000⟧B)": no such element in array}} {trace_rows {{5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 u} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r}}}⟦EOF⟧
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286s_arrayindex_root(A⟦U+0000⟧B)": no such element in array}} {trace_rows {{5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 u} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 41 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179696e6465785f726f6f74 410042 r}}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
