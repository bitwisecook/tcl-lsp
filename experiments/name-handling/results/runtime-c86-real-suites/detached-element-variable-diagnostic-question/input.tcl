puts [list PROVIDER [info patchlevel]]
proc emit {id script} {
    catch {namespace delete ::test_ns_var}
    set ::errorCode {NH UNSET}
    set code [catch $script result]
    puts [list ROW $id $code $result $::errorCode]
}
emit original-var17 {
    namespace eval test_ns_var {
        variable result
        proc p {} {
            array set x {1 2 3 4}
            upvar 0 x(1) foo
            lappend result [catch {set foo} msg] $msg
            unset x
            puts {MARK original-var17 before-write}
            lappend result [catch {set foo 3} msg] $msg
            return [list $result $::errorCode]
        }
        set result [p]
        namespace delete [namespace current]
        set result
    }
}
emit root-unset-head {
    proc p {} {
        array set x {1 2 3 4}
        upvar 0 x(1) foo
        set live [set foo]
        unset x
        set head set
        puts {MARK root-unset-head before-write}
        set code [catch {$head foo 3} msg]
        return [list $live $code $msg $::errorCode]
    }
    p
}
emit element-unset {
    proc p {} {
        array set x {1 2 3 4}
        upvar 0 x(1) foo
        set live [set foo]
        unset x(1)
        puts {MARK element-unset before-write}
        set code [catch {set foo 3} msg]
        return [list $live $code $msg [set x(1)] $::errorCode]
    }
    p
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
