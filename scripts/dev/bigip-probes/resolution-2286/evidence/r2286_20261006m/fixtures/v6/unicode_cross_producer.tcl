set prefix __tcl_lsp_2286_r2286m_unicodecross_
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
