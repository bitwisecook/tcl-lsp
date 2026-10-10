if {![llength [info commands lseq]]} {
    list UNAVAILABLE
} else {
    set sequence [lseq 9]
    set code [catch {lindex $sequence 0} result]
    set description [::tcl::unsupported::representation $sequence]
    if {![regexp {^value is a ([^ ]+)} $description -> type]} {
        error {unrecognised representation kind}
    }
    list $code $result $type
}
