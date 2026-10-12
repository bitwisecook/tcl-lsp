foreach {label body} {
    code_only {return -code ok}
    level_only {return -level 0}
    code_result {return -code ok RESULT}
    level_result {return -level 0 RESULT}
    sole_dynamic {set value -code; return $value}
} {
    proc probe {} $body
    set completion [catch {probe} result]
    puts [list $label $completion $result]
}
