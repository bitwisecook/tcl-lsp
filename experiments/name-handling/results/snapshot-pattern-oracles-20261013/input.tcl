proc emit_query {label words} {
    set code [catch {uplevel #0 $words} value]
    puts [list ROW $label $code $value]
}
puts [list IDENTITY [info patchlevel]]
emit_query library [list info library]
namespace eval {::N[0]} {variable x BRACKET_VARIABLE; proc p {} {return BRACKET_COMMAND}}
namespace eval ::N0 {variable y SIBLING_VARIABLE; proc q {} {return SIBLING_COMMAND}}
emit_query vars_original [list lsort [info vars {::N[0]::*}]]
emit_query vars_escaped [list lsort [info vars {::N\[0\]::*}]]
emit_query vars_sibling [list lsort [info vars ::N0::*]]
emit_query commands_original [list lsort [info commands {::N[0]::*}]]
emit_query commands_escaped [list lsort [info commands {::N\[0\]::*}]]
emit_query commands_sibling [list lsort [info commands ::N0::*]]
emit_query children_original [list namespace children :: {::N[0]}]
emit_query children_escaped [list namespace children :: {::N\[0\]}]
emit_query children_relative_original [list namespace children :: {N[0]}]
emit_query children_relative_escaped [list namespace children :: {N\[0\]}]
emit_query bracket_scalar_original [list set {::N[0]::x}]
emit_query sibling_scalar_original [list set ::N0::y]
emit_query bracket_command_original [list {::N[0]::p}]
emit_query sibling_command_original [list ::N0::q]
set {:errorInfo} COLON_ERROR_INFO
set errorInfo PLAIN_ERROR_INFO
set report_names {}
foreach name [info vars ::*] {
    if {[string match *errorInfo $name]} {lappend report_names $name}
}
puts [list ROW error_report_names 0 [lsort $report_names]]
emit_query error_unqualified_colon [list set {:errorInfo}]
emit_query error_unqualified_plain [list set errorInfo]
emit_query error_rooted_plain [list set ::errorInfo]
emit_query error_rooted_colon_spelling [list set :::errorInfo]
foreach name [lsort $report_names] {
    set errorInfo PLAIN_ERROR_INFO
    emit_query [list reuse_report $name] [list set $name]
    puts [list ROW [list trim_filter $name] 0 [list [string trimleft $name :] [expr {[string trimleft $name :] eq "errorInfo"}]]]
}
puts [list DONE snapshot_pattern_probe]
