proc run {label script} {
    set code [catch {eval $script} result]
    puts [list $label $code $result]
}
run empty {
    proc preserved {} {return ORIGINAL}
    foreach x {} {rename preserved {}; error BODY}
    list [preserved] [info exists x]
}
run multiple-empty {
    proc preserved {} {return ORIGINAL}
    foreach {x y} {} z {} {rename preserved {}; error BODY}
    list [preserved] [info exists x] [info exists y] [info exists z]
}
run existing-array {
    set x(k) KEEP
    foreach x {} {error BODY}
    list $x(k) [array exists x]
}
run nonempty {
    proc preserved {} {return ORIGINAL}
    foreach x {1} {proc preserved {} {return BODY}}
    list [preserved] $x
}
run list-argument-effect {
    proc preserved {} {return ORIGINAL}
    foreach x [eval {proc preserved {} {return ARGUMENT}; list}] {error BODY}
    list [preserved] [info exists x]
}
run body-argument-effect {
    proc preserved {} {return ORIGINAL}
    foreach x {} [eval {proc preserved {} {return BODY_WORD}; list error BODY}]
    list [preserved] [info exists x]
}
run argument-error {
    set code [catch {foreach x [error ARGUMENT] {error BODY}} result]
    list $code $result [info exists x]
}
run empty-varlist {
    set code [catch {foreach {} {} {error BODY}} result]
    list $code $result
}
run malformed-values {
    set values "\{"
    set code [catch {foreach x $values {error BODY}} result]
    list $code $result [info exists x]
}
run wrong-arity {
    set code [catch {foreach x {} {} extra} result]
    list $code $result
}
run shadowed-handler {
    rename foreach stock_foreach
    proc foreach {args} {proc preserved {} {return SHADOW}; return SHADOW_RESULT}
    set code [catch {foreach x {} {error BODY}} result]
    rename foreach {}
    rename stock_foreach foreach
    list $code $result [preserved] [info exists x]
}
puts [list observer-command trace [info commands trace] xtrace [info commands xtrace]]
