# BIG-IP corrected naming: nul array root portable

## Question

Does an embedded NUL distinguish an array root from its shorter prefix through writes, reads, links, unsets and supported trace callbacks across BIG-IP Tcl contexts? This affects array-root modeling in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. The result is scoped to the named context and inputs; it does not establish behavior in another context or version. Other assumptions were not separately recorded.

## Script

```tcl-visible
set trace_rows {}
set prefix __tcl_lsp_2286_r2286s_arrayroot_
set plain $prefix
append plain A
set counted $plain
append counted [binary format H* 0042]
set plain_var $plain
append plain_var (k)
set counted_var $counted
append counted_var (k)
set procname ::__tcl_lsp_2286_r2286s_nul_array_root_trace
set rows [list [list roots [string length $plain] [string length $counted]]]
set callback_body {upvar 1 trace_rows trace_rows; binary scan $name1 H* root_hex; binary scan $name2 H* index_hex; lappend trace_rows [list $root_hex $index_hex $op]}
catch {eval [list rename $procname {}]}
set rc [catch {eval [list proc $procname {name1 name2 op} $callback_body]} result]
lappend rows [list trace_proc $rc $result]
set rc [catch {foreach root [list $plain $counted] {trace variable $root rwu $procname}} result]
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
set rc [catch {upvar 0 $counted alias; set alias(k) COUNTED_MUTATION} result]
lappend rows [list upvar_write $rc $result]
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set rc [catch {set [lindex $item 1]} result]
    lappend rows [list after_link [lindex $item 0] $rc $result]
}
set rc [catch {unset $counted} result]
lappend rows [list unset_counted_root $rc $result]
foreach item [list [list plain $plain_var] [list counted $counted_var]] {
    set varname [lindex $item 1]
    lappend rows [list after_unset_exists [lindex $item 0] [info exists $varname]]
    set rc [catch {set $varname} result]
    lappend rows [list after_unset_read [lindex $item 0] $rc $result]
}
foreach root [list $plain $counted] {catch {trace vdelete $root rwu $procname}; catch {unset $root}}
catch {eval [list rename $procname {}]}
lappend rows [list trace_rows $trace_rows]
set rows
```

## Results

The initial HTTP collection labeled for this root test returned the array-index test's result, so it cannot establish a result for this script. The separately loaded root rerun is recorded below.

### TMM 0:0, HTTP_REQUEST, rerun; TMM 0:1, HTTP_REQUEST, rerun; TMM 0:2, HTTP_REQUEST, rerun; TMM 0:3, HTTP_REQUEST, rerun

- Recorded version: Not recorded; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST, rerun; TMM 0:1, HTTP_REQUEST, rerun; TMM 0:2, HTTP_REQUEST, rerun; TMM 0:3, HTTP_REQUEST, rerun.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{roots 33 35} {trace_proc 0 {}} {trace_add 0 {}} {write 36 0 PLAIN} {write 38 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_root 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286s_arrayroot_A⟦U+0000⟧B(k)": no such variable}} {trace_rows {{5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {616c696173 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 {} u} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r}}}⟦EOF⟧
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
{roots 33 35} {trace_proc 0 {}} {trace_add 0 {}} {write 36 0 PLAIN} {write 38 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_root 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286s_arrayroot_A⟦U+0000⟧B(k)": no such variable}} {trace_rows {{5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {616c696173 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 {} u} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r}}}⟦EOF⟧
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
{roots 33 35} {trace_proc 0 {}} {trace_add 0 {}} {write 36 0 PLAIN} {write 38 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_root 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286s_arrayroot_A⟦U+0000⟧B(k)": no such variable}} {trace_rows {{5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {616c696173 6b w} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f410042 {} u} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r} {5f5f74636c5f6c73705f323238365f7232323836735f6172726179726f6f745f41 6b r}}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
