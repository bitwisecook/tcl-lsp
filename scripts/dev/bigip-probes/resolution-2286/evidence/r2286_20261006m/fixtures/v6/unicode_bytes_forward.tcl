set prefix __tcl_lsp_2286_r2286m_unicodebytes_
set pre $prefix
append pre [binary format H* c3a9]
set decomposed $prefix
append decomposed [binary format H* 65cc81]
set rows [list [list lengths [string length $pre] [string length $decomposed]]]
foreach name [list $pre $decomposed] {
    set raw_rc [catch {binary scan $name H* raw} raw_result]
    if {$raw_rc != 0} {set raw $raw_result}
    set rc [catch {encoding convertto utf-8 $name} encoded]
    if {$rc == 0} {binary scan $encoded H* encoded}
    lappend rows [list raw_conversion $raw_rc $raw utf8_conversion $rc $encoded]
}
set rc [catch {set $pre PRECOMPOSED} result]
lappend rows [list write_pre $rc $result]
set rc [catch {set $decomposed DECOMPOSED} result]
lappend rows [list write_decomposed $rc $result]
foreach name [list $pre $decomposed] {
    set rc [catch {set $name} result]
    lappend rows [list read $rc $result]
}
set rc [catch {unset $pre} result]
lappend rows [list unset_pre $rc $result]
set rc [catch {set $decomposed} result]
lappend rows [list remaining_decomposed $rc $result]
foreach name [list $pre $decomposed] {catch {unset $name}}
set rows
