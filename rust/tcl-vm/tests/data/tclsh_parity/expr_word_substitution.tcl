# A lone substituted `expr` word is evaluated by its value, not its spelling.
set c {1+2}
set x [expr $c]
puts "set: $x"
puts "quoted: [expr "$c*2"]"
puts "braced: [expr {$c}]"
puts "escaped: [set y [expr 1\u002b2]]"
puts "command: [set y [expr [list 4*5]]]"
proc p {c} {
    set x [expr $c]
    lappend l [expr $c] [expr "$c"] [expr {$c}] [expr 7\u002d1]
    puts "proc set: $x"
    puts "proc list: $l"
    return [expr $c]
}
puts "proc return: [p {3*3}]"
proc q {c} {
    set code [catch {expr $c} r]
    list $code $r
}
puts "catch: [q {acos(1)}] [q {1/0}]"
proc r {c} {
    catch {expr 2*3} v
    lappend v [expr 2*$c]
    expr $c+1
}
puts "statement: [r 4]"
catch {expr 5-1} v
puts "top catch: $v"
