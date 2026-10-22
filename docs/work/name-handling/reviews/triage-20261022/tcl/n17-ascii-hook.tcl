# Same guarded body as pack_source_e2e::FOLD_SOURCE_BODY, evaluated as Tcl.
proc fold {value} {return $value}
proc hook {words ctx} {
    set subject [lindex $words 0]
    if {![string is ascii $subject]} { return }
    fold [string length $subject]
}
puts "ascii=[string is ascii abcde]"
puts "hook=[hook {abcde} {}]"
