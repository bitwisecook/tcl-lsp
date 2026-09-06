# T3: the observable surface of a proc whose body is dispatched natively -
# recursion, arity, introspection, scope, traces, and redefinition all answer
# exactly what an interpreted body answers (issue #1774).
#
# Every definition this file wants bound to a compiled body is written before
# the first command that widens the world state, because a `proc` statement
# only lowers to the definition ABI while its own dispatch is still proven.
proc fib {n} {
    if {$n < 2} { return $n }
    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
}
proc greet {name {greeting hello} args} { return "$greeting $name ([llength $args])" }
proc lvl {} { return [list [info level] [info level 0]] }
proc outer {x} { return [lvl] }
proc incrby {name delta} { upvar 1 $name v; incr v $delta; uplevel 1 {set seen 1} }
proc traced {x} { return [expr {$x * 2}] }
proc stepped {x} { set y $x; return [list $y $y] }
proc twice {x} { expr {$x * 2} }
proc pick {} { return first }
# The bodies whose last command leaves no interpreter result of its own: the
# procedure still answers what that command answers.
proc last {x} { set y $x }
proc bumped {x} { incr x }
proc show {x} { puts $x }
proc nothing {} { return }
proc empty {} {}
# ...and the two shapes whose answer the compiled body cannot produce, so the
# definition keeps its source body: `append` does not hand back the cell's new
# value, and a one-armed `if` completes with no result of its own.
proc grow {s} { append s ! }
proc branch {x} { if {$x} { set y yes } }

puts [fib 20]

puts [greet bob]
puts [catch {greet} msg]
puts $msg
puts [catch {greet a b c d e} msg]
puts $msg

puts [outer 7]

set counter 10
incrby counter 5
puts "$counter $seen"

trace add execution traced enter {apply {{cmd op} {puts "enter: $cmd"}}}
trace add execution traced leave {apply {{cmd code result op} {puts "leave: $cmd -> $result"}}}
puts [traced 21]

trace add execution stepped enterstep {apply {{cmd op} {puts "step: $cmd"}}}
puts [stepped ab]

puts [twice 4]
proc twice {x} { expr {$x * 3} }
puts [twice 4]
rename twice thrice
puts [thrice 4]
puts [catch {twice 4} msg]
puts $msg
namespace eval ns { proc inner {} { return in } }
puts [ns::inner]
namespace delete ns
puts [catch {ns::inner} msg]
puts $msg

puts [pick]
proc pick {} { return second }
puts [pick]

puts [last hello]
puts [bumped 41]
puts "[show inner]<"
puts "[nothing]<"
puts "[empty]<"
puts [grow hi]
puts "[branch 1]|[branch 0]<"
