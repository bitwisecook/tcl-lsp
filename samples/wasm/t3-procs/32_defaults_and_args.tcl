# T3: formal-parameter grammar: defaults, args, wrong-arity error text.
proc greet {name {greeting hello} args} {
    set extra [llength $args]
    return "$greeting $name ($extra extra)"
}
puts [greet bob]
puts [greet bob hi]
puts [greet bob hi 1 2 3]
catch {greet} msg
puts $msg
proc sum {args} { set s 0; foreach a $args { incr s $a }; return $s }
puts [sum 1 2 3 4]
puts [sum {*}{5 6 7}]
