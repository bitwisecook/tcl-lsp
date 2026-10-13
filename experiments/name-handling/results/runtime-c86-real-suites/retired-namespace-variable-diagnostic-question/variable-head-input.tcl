puts [list PROVIDER [info patchlevel]]
proc emit {id script} {
    catch {namespace delete ::test_ns_var}
    set ::errorCode {NH UNSET}
    set code [catch $script result]
    puts [list ROW $id $code $result $::errorCode]
}
emit variable-head-scalar {
    namespace eval test_ns_var {variable foo 2}
    proc p {} {
        variable ::test_ns_var::foo
        set liveCode [catch {set foo} liveResult]
        set head set
        namespace delete ::test_ns_var
        puts {MARK variable-head-scalar before-write}
        set code [catch {$head foo 3} result]
        return [list $liveCode $liveResult $code $result $::errorCode]
    }
    p
}
emit variable-head-element {
    namespace eval test_ns_var {variable foo 2}
    proc p {} {
        variable ::test_ns_var::foo
        set liveCode [catch {set foo} liveResult]
        set head set
        namespace delete ::test_ns_var
        puts {MARK variable-head-element before-write}
        set code [catch {$head foo(3) 3} result]
        return [list $liveCode $liveResult $code $result $::errorCode]
    }
    p
}
