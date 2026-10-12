set prefix __tcl_lsp_2286_r2286m_nul_
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
set rc [catch {unset $counted} result]
lappend rows [list unset_counted $rc $result]
foreach name [list $plain $counted] {
    set rc [catch {set $name} result]
    lappend rows [list after_unset [string length $name] $rc $result]
}
foreach name [list $plain $counted ${prefix}link] {catch {unset $name}}
set rows
