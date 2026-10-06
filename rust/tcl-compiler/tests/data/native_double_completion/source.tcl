proc report {id script} {
    set code [catch {uplevel 1 $script} value]
    puts [list $id $code $value]
}
report double21 {expr {double(21)}}
report invalid {expr {double("invalid")}}
report infinity {expr {double(Inf)}}
report negative {expr {double(-21)}}
report arity_zero {expr {double()}}
report arity_two {expr {double(1,2)}}
report argument_return {proc returned {} {expr {double([return EARLY])}}; returned}
report argument_error {expr {double([error ARGUMENT])}}
proc read_error {name index operation} {error READ_ERROR}
set original 21
set trace_kind {}
if {![catch {trace add variable original read read_error}]} {
    set trace_kind modern
} elseif {![catch {trace variable original r read_error}]} {
    set trace_kind legacy
}
if {$trace_kind == {}} {
    puts [list read_trace_unavailable 0 unavailable]
} else {
    report read_error {expr {double($original)}}
    if {$trace_kind == "modern"} {
        trace remove variable original read read_error
    } else {
        trace vdelete original r read_error
    }
}
report replacement {
    if {[llength [info commands ::tcl::mathfunc::double]]} {
        rename ::tcl::mathfunc::double ::saved_double
        proc ::tcl::mathfunc::double {value} {error REPLACED}
    }
    expr {double(21)}
}
