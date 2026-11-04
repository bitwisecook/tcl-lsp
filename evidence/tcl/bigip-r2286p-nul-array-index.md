# BIG-IP cross-context naming: nul array index

## Question

Does an embedded NUL distinguish an array index from its shorter prefix through writes, reads, links and unsets? This affects array-element resolution in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

```tcl-visible
set prefix __tcl_lsp_2286_r2286p_arrayindex_
set root $prefix
append root root
set plain A
set counted $plain
append counted [binary format H* 0042]
set plain_var $root
append plain_var ( $plain )
set counted_var $root
append counted_var ( $counted )
set procname ::__tcl_lsp_2286_r2286p_nul_array_index_trace
set rows [list [list indexes [string length $plain] [string length $counted]]]
set callback_body [binary format H* 62696e617279207363616e20246e616d653120482a20726f6f745f6865780a62696e617279207363616e20246e616d653220482a20696e6465785f6865780a6c6f67206c6f63616c302e2022523232383654524143457c7232323836707c6e756c5f61727261795f696e6465787c746d6d3d5b544d4d3a3a636d705f67726f75705d3a5b544d4d3a3a636d705f756e69745d7c726f6f745f6865783d24726f6f745f6865787c696e6465785f6865783d24696e6465785f6865787c6f703d246f7022]
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 1 {can't set "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {write 42 1 {can't set "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {exists plain 1} {read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {exists counted 1} {read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {upvar_write 0 COUNTED_MUTATION} {after_link plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {after_link counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}}⟦EOF⟧
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 1 {can't set "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {write 42 1 {can't set "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {exists plain 1} {read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {exists counted 1} {read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {upvar_write 0 COUNTED_MUTATION} {after_link plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {after_link counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}}⟦EOF⟧
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 1 {can't set "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {write 42 1 {can't set "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {exists plain 1} {read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {exists counted 1} {read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {upvar_write 0 COUNTED_MUTATION} {after_link plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {after_link counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A)": invalid command name "TMM::cmp_group"}} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayindex_root(A⟦U+0000⟧B)": invalid command name "TMM::cmp_group"}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
