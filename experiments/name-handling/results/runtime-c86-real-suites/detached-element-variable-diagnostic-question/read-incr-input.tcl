puts [list PROVIDER [info patchlevel]]
proc emit {id script} {
    set ::errorCode {NH UNSET}
    set code [catch $script result]
    puts [list ROW $id $code $result $::errorCode]
}
emit root-unset-read {
    proc p {} {
        array set x {1 2 3 4}
        upvar 0 x(1) foo
        set live [set foo]
        unset x
        puts {MARK root-unset-read before-read}
        set code [catch {set foo} msg]
        return [list $live $code $msg $::errorCode]
    }
    p
}
emit root-unset-incr {
    proc p {} {
        array set x {1 2 3 4}
        upvar 0 x(1) foo
        set live [set foo]
        unset x
        puts {MARK root-unset-incr before-incr}
        set code [catch {incr foo} msg]
        return [list $live $code $msg $::errorCode]
    }
    p
}
