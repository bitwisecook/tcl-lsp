set prefix ::__tcl_lsp_2286_r2286m_command_cross_
set format_pre $prefix
append format_pre [format %c 233]
set bytes_pre $prefix
append bytes_pre [binary format H* c3a9]
set format_decomposed $prefix
append format_decomposed e [format %c 769]
set bytes_decomposed $prefix
append bytes_decomposed [binary format H* 65cc81]
set rows {}
foreach group [list [list pre $format_pre $bytes_pre] [list decomposed $format_decomposed $bytes_decomposed]] {
    set label [lindex $group 0]
    set format_ns [lindex $group 1]
    set bytes_ns [lindex $group 2]
    set format_proc $format_ns
    append format_proc ::identity
    set bytes_proc $bytes_ns
    append bytes_proc ::identity
    set renamed $bytes_proc
    append renamed _renamed
    foreach pair [list [list format $format_ns] [list bytes $bytes_ns]] {
        set ns [lindex $pair 1]
        binary scan $ns H* raw
        set rc [catch {namespace eval $ns {}} result]
        lappend rows [list namespace_create $label [lindex $pair 0] [string length $ns] $raw $rc $result]
    }
    set rc [catch {eval [list proc $format_proc {} [list return FORMAT_COMMAND]]} result]
    lappend rows [list proc_create $label format $rc $result]
    set rc [catch {eval [list proc $bytes_proc {} [list return BYTES_COMMAND]]} result]
    lappend rows [list proc_create $label bytes $rc $result]
    foreach pair [list [list format $format_proc] [list bytes $bytes_proc]] {
        set name [lindex $pair 1]
        set lookup_rc [catch {namespace which -command $name} lookup]
        set call_rc [catch {eval [list $name]} value]
        lappend rows [list before_rename $label [lindex $pair 0] $lookup_rc $lookup $call_rc $value]
    }
    set rc [catch {rename $bytes_proc $renamed} result]
    lappend rows [list rename_bytes $label $rc $result]
    foreach pair [list [list format $format_proc] [list bytes $bytes_proc] [list renamed $renamed]] {
        set name [lindex $pair 1]
        set lookup_rc [catch {namespace which -command $name} lookup]
        set call_rc [catch {eval [list $name]} value]
        lappend rows [list after_rename $label [lindex $pair 0] $lookup_rc $lookup $call_rc $value]
    }
}
foreach ns [list $format_pre $bytes_pre $format_decomposed $bytes_decomposed] {catch {namespace delete $ns}}
set rows
