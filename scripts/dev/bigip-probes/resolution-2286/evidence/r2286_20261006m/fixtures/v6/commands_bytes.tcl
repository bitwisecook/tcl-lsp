set prefix ::__tcl_lsp_2286_r2286m_command_bytes_
set pre $prefix
append pre [binary format H* c3a9]
set decomposed $prefix
append decomposed [binary format H* 65cc81]
set rows {}
foreach pair [list [list pre $pre PRE_COMMAND] [list decomposed $decomposed DECOMPOSED_COMMAND]] {
    set label [lindex $pair 0]
    set ns [lindex $pair 1]
    set value [lindex $pair 2]
    set procname $ns
    append procname ::identity
    set renamed $procname
    append renamed _renamed
    binary scan $ns H* ns_hex
    set rc [catch {namespace eval $ns {}} result]
    lappend rows [list namespace_create $label $ns_hex $rc $result]
    set body [list return $value]
    set rc [catch {eval [list proc $procname {} $body]} result]
    lappend rows [list proc_create $label $rc $result]
    set rc [catch {namespace which -command $procname} result]
    lappend rows [list lookup_original $label $rc $result]
    set rc [catch {eval [list $procname]} result]
    lappend rows [list call_original $label $rc $result]
    set rc [catch {rename $procname $renamed} result]
    lappend rows [list rename $label $rc $result]
    set rc [catch {namespace which -command $procname} result]
    lappend rows [list lookup_old_after_rename $label $rc $result]
    set rc [catch {namespace which -command $renamed} result]
    lappend rows [list lookup_new_after_rename $label $rc $result]
    set rc [catch {eval [list $renamed]} result]
    lappend rows [list call_renamed $label $rc $result]
}
foreach ns [list $pre $decomposed] {catch {namespace delete $ns}}
set rows
