puts [list PROVIDER [info patchlevel]]
proc emit {id script} {
    catch {namespace delete ::test_ns_var}
    set ::errorCode {NH UNSET}
    set code [catch $script result]
    puts [list ROW $id $code $result $::errorCode]
}
emit literal-scalar {
    namespace eval test_ns_var {variable foo 2}
    proc p {} {
        variable ::test_ns_var::foo
        set liveCode [catch {set foo} liveResult]
        namespace delete ::test_ns_var
        puts {MARK literal-scalar before-write}
        set code [catch {set foo 3} result]
        return [list $liveCode $liveResult $code $result $::errorCode]
    }
    p
}
emit literal-element {
    namespace eval test_ns_var {variable foo 2}
    proc p {} {
        variable ::test_ns_var::foo
        set liveCode [catch {set foo} liveResult]
        namespace delete ::test_ns_var
        puts {MARK literal-element before-write}
        set code [catch {set foo(3) 3} result]
        return [list $liveCode $liveResult $code $result $::errorCode]
    }
    p
}
emit dynamic-scalar {
    namespace eval test_ns_var {variable foo 2}
    proc p {} {
        variable ::test_ns_var::foo
        set liveCode [catch {set foo} liveResult]
        namespace delete ::test_ns_var
        puts {MARK dynamic-scalar before-write}
        set command [list set foo 3]
        set code [catch $command result]
        return [list $liveCode $liveResult $code $result $::errorCode]
    }
    p
}
emit dynamic-element {
    namespace eval test_ns_var {variable foo 2}
    proc p {} {
        variable ::test_ns_var::foo
        set liveCode [catch {set foo} liveResult]
        namespace delete ::test_ns_var
        puts {MARK dynamic-element before-write}
        set command [list set foo(3) 3]
        set code [catch $command result]
        return [list $liveCode $liveResult $code $result $::errorCode]
    }
    p
}
emit original-sequence {
    namespace eval test_ns_var {variable foo 2}
    proc p {} {
        variable ::test_ns_var::foo
        lappend result [catch {set foo} msg] $msg
        namespace delete ::test_ns_var
        puts {MARK original-sequence before-scalar-write}
        lappend result [catch {set foo 3} msg] $msg
        set scalarCode $::errorCode
        puts {MARK original-sequence before-element-write}
        lappend result [catch {set foo(3) 3} msg] $msg
        return [list $result $scalarCode $::errorCode]
    }
    p
}
