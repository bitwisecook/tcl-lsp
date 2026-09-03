# T1: if/elseif/else with braced conditions on a known-int variable.
set n 7
if {$n < 5} {
    puts small
} elseif {$n < 10} {
    puts medium
} else {
    puts large
}
if {$n % 2 == 0} { puts even } else { puts odd }
