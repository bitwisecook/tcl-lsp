# BIG-IP corrected naming: qualified nul

## Question

Do qualified scalar and array names containing embedded NUL bytes reach distinct variables through reads, writes and traces? This affects qualified-name resolution in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. The result is scoped to the named context and inputs; it does not establish behavior in another context or version. Other assumptions were not separately recorded.

## Script

```tcl-visible
set ns ::__tcl_lsp_2286_r2286r_qualified
set traceproc ::__tcl_lsp_2286_r2286r_qualified_trace
set rows {}
set trace_rows {}
set ns_create_rc [catch {namespace eval $ns {}} ns_create_result]
lappend rows [list namespace_create $ns_create_rc $ns_create_result]
set callback_body {upvar 1 trace_rows trace_rows; binary scan $name1 H* root_hex; binary scan $name2 H* index_hex; lappend trace_rows [list $root_hex $index_hex $op]}
catch {eval [list rename $traceproc {}]}
set trace_proc_rc [catch {eval [list proc $traceproc {name1 name2 op} $callback_body]} trace_proc_result]
lappend rows [list trace_proc $trace_proc_rc $trace_proc_result]
set plain $ns
append plain ::A
set counted $plain
append counted [binary format H* 0042]
binary scan $plain H* plain_hex
binary scan $counted H* counted_hex
lappend rows [list scalar_roots $plain_hex $counted_hex]
foreach pair [list [list $counted COUNTED] [list $plain PLAIN]] {
    set rc [catch {set [lindex $pair 0] [lindex $pair 1]} result]
    lappend rows [list scalar_write $rc $result]
}
set rc [catch {upvar 0 $counted scalar_alias; set scalar_alias COUNTED_MUTATION} result]
lappend rows [list scalar_upvar $rc $result]
set rc [catch {unset $plain} result]
lappend rows [list scalar_unset_plain $rc $result]
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list scalar_after_unset $rc $result]
}
catch {unset scalar_alias}
catch {unset $plain}
catch {unset $counted}
foreach root [list $plain $counted] {
    set rc [catch {trace variable $root rwu $traceproc} result]
    lappend rows [list array_trace_add $rc $result]
}
set plain_var $plain
append plain_var (k)
set counted_var $counted
append counted_var (k)
foreach pair [list [list $plain_var PLAIN_ARRAY] [list $counted_var COUNTED_ARRAY]] {
    set rc [catch {set [lindex $pair 0] [lindex $pair 1]} result]
    lappend rows [list array_write $rc $result]
}
set rc [catch {upvar 0 $counted array_alias; set array_alias(k) COUNTED_ARRAY_MUTATION} result]
lappend rows [list array_upvar $rc $result]
set rc [catch {unset $counted} result]
lappend rows [list array_unset_counted $rc $result]
foreach name [list $plain_var $counted_var] {
    set rc [catch {set $name} result]
    lappend rows [list array_after_unset $rc $result]
}
set index_root $ns
append index_root ::IndexRoot
set plain_index A
set counted_index $plain_index
append counted_index [binary format H* 0042]
set rc [catch {trace variable $index_root rwu $traceproc} result]
lappend rows [list index_trace_add $rc $result]
set plain_index_var $index_root
append plain_index_var ( $plain_index )
set counted_index_var $index_root
append counted_index_var ( $counted_index )
foreach pair [list [list $counted_index_var COUNTED_INDEX] [list $plain_index_var PLAIN_INDEX]] {
    set rc [catch {set [lindex $pair 0] [lindex $pair 1]} result]
    lappend rows [list index_write $rc $result]
}
set rc [catch {upvar 0 $counted_index_var index_alias; set index_alias COUNTED_INDEX_MUTATION} result]
lappend rows [list index_upvar $rc $result]
set rc [catch {unset $plain_index_var} result]
lappend rows [list index_unset_plain $rc $result]
foreach name [list $plain_index_var $counted_index_var] {
    set rc [catch {set $name} result]
    lappend rows [list index_after_unset $rc $result]
}
lappend rows [list trace_rows $trace_rows]
foreach root [list $plain $counted $index_root] {catch {trace vdelete $root rwu $traceproc}; catch {unset $root}}
catch {eval [list rename $traceproc {}]}
set ns_current_rc [catch {namespace current} ns_current_result]
set ns_which_rc [catch {namespace which -command set} ns_which_result]
lappend rows [list namespace_inventory $ns_current_rc $ns_current_result $ns_which_rc $ns_which_result]
catch {namespace delete $ns}
set rows
```

## Results

### TMM 0:0, HTTP_REQUEST, initial; TMM 0:1, HTTP_REQUEST, initial; TMM 0:2, HTTP_REQUEST, initial; TMM 0:3, HTTP_REQUEST, initial

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST, initial; TMM 0:1, HTTP_REQUEST, initial; TMM 0:2, HTTP_REQUEST, initial; TMM 0:3, HTTP_REQUEST, initial.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{namespace_create 0 {}} {trace_proc 0 {}} {scalar_roots 3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042} {scalar_write 0 COUNTED} {scalar_write 0 PLAIN} {scalar_upvar 0 COUNTED_MUTATION} {scalar_unset_plain 0 {}} {scalar_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::A": no such variable}} {scalar_after_unset 0 COUNTED_MUTATION} {array_trace_add 0 {}} {array_trace_add 0 {}} {array_write 0 PLAIN_ARRAY} {array_write 0 COUNTED_ARRAY} {array_upvar 0 COUNTED_ARRAY_MUTATION} {array_unset_counted 0 {}} {array_after_unset 0 PLAIN_ARRAY} {array_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::A⟦U+0000⟧B(k)": no such variable}} {index_trace_add 0 {}} {index_write 0 COUNTED_INDEX} {index_write 0 PLAIN_INDEX} {index_upvar 0 COUNTED_INDEX_MUTATION} {index_unset_plain 0 {}} {index_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::IndexRoot(A)": no such element in array}} {index_after_unset 0 COUNTED_INDEX_MUTATION} {trace_rows {{3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 6b w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042 6b w} {61727261795f616c696173 6b w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042 {} u} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 6b r} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 410042 w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 u} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 r} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 410042 r}}} {namespace_inventory 0 :: 0 ::set}⟦EOF⟧
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
{namespace_create 0 {}} {trace_proc 0 {}} {scalar_roots 3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042} {scalar_write 0 COUNTED} {scalar_write 0 PLAIN} {scalar_upvar 0 COUNTED_MUTATION} {scalar_unset_plain 0 {}} {scalar_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::A": no such variable}} {scalar_after_unset 0 COUNTED_MUTATION} {array_trace_add 0 {}} {array_trace_add 0 {}} {array_write 0 PLAIN_ARRAY} {array_write 0 COUNTED_ARRAY} {array_upvar 0 COUNTED_ARRAY_MUTATION} {array_unset_counted 0 {}} {array_after_unset 0 PLAIN_ARRAY} {array_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::A⟦U+0000⟧B(k)": no such variable}} {index_trace_add 0 {}} {index_write 0 COUNTED_INDEX} {index_write 0 PLAIN_INDEX} {index_upvar 0 COUNTED_INDEX_MUTATION} {index_unset_plain 0 {}} {index_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::IndexRoot(A)": no such element in array}} {index_after_unset 0 COUNTED_INDEX_MUTATION} {trace_rows {{3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 6b w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042 6b w} {61727261795f616c696173 6b w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042 {} u} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 6b r} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 410042 w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 u} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 r} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 410042 r}}} {namespace_inventory 0 :: 0 ::set}⟦EOF⟧
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
{namespace_create 0 {}} {trace_proc 0 {}} {scalar_roots 3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042} {scalar_write 0 COUNTED} {scalar_write 0 PLAIN} {scalar_upvar 0 COUNTED_MUTATION} {scalar_unset_plain 0 {}} {scalar_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::A": no such variable}} {scalar_after_unset 0 COUNTED_MUTATION} {array_trace_add 0 {}} {array_trace_add 0 {}} {array_write 0 PLAIN_ARRAY} {array_write 0 COUNTED_ARRAY} {array_upvar 0 COUNTED_ARRAY_MUTATION} {array_unset_counted 0 {}} {array_after_unset 0 PLAIN_ARRAY} {array_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::A⟦U+0000⟧B(k)": no such variable}} {index_trace_add 0 {}} {index_write 0 COUNTED_INDEX} {index_write 0 PLAIN_INDEX} {index_upvar 0 COUNTED_INDEX_MUTATION} {index_unset_plain 0 {}} {index_after_unset 1 {can't read "::__tcl_lsp_2286_r2286r_qualified::IndexRoot(A)": no such element in array}} {index_after_unset 0 COUNTED_INDEX_MUTATION} {trace_rows {{3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 6b w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042 6b w} {61727261795f616c696173 6b w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a410042 {} u} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a41 6b r} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 410042 w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 w} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 u} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 41 r} {3a3a5f5f74636c5f6c73705f323238365f7232323836725f7175616c69666965643a3a496e646578526f6f74 410042 r}}} {namespace_inventory 0 ::script 0 ::set}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
