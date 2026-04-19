#!/usr/bin/env tclsh
# Benchmark: individual set vs lassign vs foreach {break}
#
# Usage:
#   tclsh8.5 scripts/benchmark_set_packing.tcl
#   tclsh8.6 scripts/benchmark_set_packing.tcl
#   tclsh9.0 scripts/benchmark_set_packing.tcl
#
# Measures per-call cost (microseconds) for three strategies of
# assigning N variables to literal values.  Use the output to
# decide the minimum N at which lassign or foreach becomes faster
# than individual set commands.

puts "Tcl [info patchlevel]"
puts [string repeat - 90]
puts [format "%-6s  %12s  %12s  %12s  %12s  %12s" \
    "N" "set (us)" "lassign (us)" "foreach (us)" "lassign/set" "foreach/set"]
puts [string repeat - 90]

foreach n {2 3 4 5 8 10 20 50} {
    # build the three command strings

    set set_body ""
    set vars {}
    set vals {}
    for {set i 0} {$i < $n} {incr i} {
        append set_body "set v$i $i\n"
        lappend vars "v$i"
        lappend vals $i
    }
    set vars_str [join $vars " "]
    set vals_str [join $vals " "]

    set lassign_cmd "lassign {$vals_str} $vars_str"
    set foreach_cmd "foreach {$vars_str} {$vals_str} {break}"

    # warm up
    eval $set_body
    eval $lassign_cmd
    eval $foreach_cmd

    # measure (100 000 iterations)
    set iters 100000

    set set_time [lindex [time {
        for {set rep 0} {$rep < $iters} {incr rep} {
            eval $set_body
        }
    }] 0]

    set lassign_time [lindex [time {
        for {set rep 0} {$rep < $iters} {incr rep} {
            eval $lassign_cmd
        }
    }] 0]

    set foreach_time [lindex [time {
        for {set rep 0} {$rep < $iters} {incr rep} {
            eval $foreach_cmd
        }
    }] 0]

    set la_ratio [expr {$lassign_time / double($set_time)}]
    set fe_ratio [expr {$foreach_time / double($set_time)}]

    puts [format "N=%-4d  %10.1f    %10.1f    %10.1f    %10.2fx   %10.2fx" \
        $n $set_time $lassign_time $foreach_time $la_ratio $fe_ratio]
}

puts [string repeat - 90]
puts "Ratios < 1.0 mean the alternative is faster than individual set commands."
