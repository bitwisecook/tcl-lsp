puts [list PROVIDER [info patchlevel]]
proc emit {id script} {
    catch {namespace delete ::test_ns_var}
    set ::errorCode {NH UNSET}
    set code [catch $script result]
    puts [list ROW $id $code $result $::errorCode]
}
emit root-recreated-old-link {
    proc p {} {
        array set x {1 2 3 4}
        upvar 0 x(1) foo
        set live [set foo]
        unset x
        array set x {1 NEW}
        puts {MARK root-recreated-old-link before-write}
        set code [catch {set foo 3} msg]
        return [list $live $code $msg [set x(1)] $::errorCode]
    }
    p
}
