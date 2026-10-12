set prefix __tcl_lsp_2286_r2286m_arrayindex_
set root $prefix
append root root
set plain A
set counted $plain
append counted [binary format H* 0042]
set plain_var $root
append plain_var ( $plain )
set counted_var $root
append counted_var ( $counted )
set procname ::__tcl_lsp_2286_r2286m_nul_array_index_trace
set rows [list [list indexes [string length $plain] [string length $counted]]]
set callback_body [binary format H* 62696e617279207363616e20246e616d653120482a20726f6f745f6865780a62696e617279207363616e20246e616d653220482a20696e6465785f6865780a6c6f67206c6f63616c302e2022523232383654524143457c72323238366d7c6e756c5f61727261795f696e6465787c746d6d3d5b544d4d3a3a636d705f67726f75705d3a5b544d4d3a3a636d705f756e69745d7c726f6f745f6865783d24726f6f745f6865787c696e6465785f6865783d24696e6465785f6865787c6f703d246f7022]
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
