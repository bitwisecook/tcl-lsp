puts [list PATCHLEVEL [info patchlevel]]

proc reportArrayWrite {label script} {
    uplevel #0 {array set a {k OLD}}
    set status [uplevel #0 [list catch $script result]]
    set result [uplevel #0 {set result}]
    if {[info exists ::errorCode]} {
        set code $::errorCode
    } else {
        set code {}
    }
    puts [list RESULT $label $status $result $code [uplevel #0 {array exists a}]]
    uplevel #0 {unset a}
}

reportArrayWrite literal_array_write {set a NEW}
reportArrayWrite dynamic_array_write {set cmd set; $cmd a NEW}
proc writeArrayRoot {} {
    global a
    set a NEW
}
reportArrayWrite procedure_array_write {writeArrayRoot}
