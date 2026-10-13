puts [list PROVIDER [info patchlevel]]
proc emit {id script} {
    catch {namespace delete ::test_ns_var}
    set ::errorCode {NH UNSET}
    set code [catch $script result]
    puts [list ROW $id $code $result $::errorCode]
}
emit root-recreated-current-link {
    proc p {} {
        array set x {1 2 3 4}
        upvar 0 x(1) foo
        set live [set foo]
        unset x
        array set x {1 NEW}
        upvar 0 x(1) current
        puts {MARK root-recreated-current-link before-write}
        set code [catch {set current 3} msg]
        return [list $live $code $msg [set x(1)] $::errorCode]
    }
    p
}
