proc BOOM {n1 n2 op} {error BOOM}
array set E {x X}
trace add variable E(x) read BOOM

set rows {}
foreach {target script} [list \
    list {array get E; a inside} \
    try {array get E; a {list inside}} \
    eval {array get E; a {list inside}} \
    switch {array get E; a x x {list inside}}] {
    interp alias {} a {} $target
    set code [catch $script message options]
    lappend rows [list $target $code $message [dict exists $options -errorcode]]
    rename a {}
}

interp create child
child eval {
    proc BOOM {n1 n2 op} {error BOOM}
    array set E {x X}
    trace add variable E(x) read BOOM
}
interp alias child a {} list
set cross [child eval {
    set code [catch {array get E; a inside} message options]
    list $code $message [dict exists $options -errorcode]
}]
proc OPT {} {return -level 0 -code ok -foo BAR value}
interp alias child opt {} OPT
set cross_options [child eval {
    set code [catch {opt} message options]
    list $code $message [dict get $options -foo]
}]
set out [list $rows $cross $cross_options]
