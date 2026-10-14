puts [list VERSION [info patchlevel]]
proc emit {name script} {
    set code [catch {uplevel 1 $script} result]
    puts [list CASE $name CODE $code RESULT $result]
}
emit dictionary-availability {info commands dict}
emit empty-name-and-body {
    set {} {mapped 11}
    dict with {} {}
    list [set {}] [set mapped]
}
emit literal-dollar-name-and-body {
    set {$dict} {literal 12}
    dict with {$dict} {}
    list [set {$dict}] [set literal] [info exists dict]
}
emit comment-only-body {
    set d {commented 13}
    dict with d {# lexical comment only
    }
    list [set d] [set commented]
}
emit trace-api {info commands trace}
set observer_calls 0
proc observer {args} {incr ::observer_calls}
set trace_code [catch {trace add variable observed write observer} trace_result]
puts [list TRACE_SETUP CODE $trace_code RESULT $trace_result]
if {$trace_code == 0} {
    emit empty-body-alias-write-observer {
        set traced_dict {observed 14}
        dict with traced_dict {}
        list [set traced_dict] [set observed] [set observer_calls]
    }
    trace remove variable observed write observer
}
emit body-command-name-mutation {
    set body_dict {body_value 15}
    dict with body_dict {proc newly_defined_name {} {}}
    list [info commands newly_defined_name] [set body_value]
}
emit malformed-dictionary-empty-body {
    set malformed {odd}
    dict with malformed {}
}
