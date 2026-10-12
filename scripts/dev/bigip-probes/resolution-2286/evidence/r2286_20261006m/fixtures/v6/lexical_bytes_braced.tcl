set prefix __tcl_lsp_2286_r2286m_lex_bytes_
set pre $prefix
append pre [binary format H* c3a9]
set decomposed $prefix
append decomposed [binary format H* 65cc81]
set rows {}
foreach pair [list [list pre $pre PRE_VALUE] [list decomposed $decomposed DECOMPOSED_VALUE]] {
    set label [lindex $pair 0]
    set name [lindex $pair 1]
    set value [lindex $pair 2]
    set write_rc [catch {set $name $value} write_result]
    set script "set \${"; append script $name; append script "}"
    binary scan $script H* script_hex
    set rc [catch {eval $script} result]
    binary scan $result H* result_hex
    lappend rows [list $label write $write_rc $write_result script_hex $script_hex eval $rc $result_hex]
}
foreach name [list $pre $decomposed] {catch {unset $name}}
set rows
