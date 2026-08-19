# A6: unset traces during proc-frame and namespace teardown, plus legacy
# registration on an array element.
set ::log {}
proc rec args { lappend ::log [join $args |] }

# Proc frame teardown: locals are unset when the frame pops.
proc p {} {
    set x 1
    trace add variable x unset {rec u1}
    trace add variable x unset {rec u2}
    return done
}
p
puts "frame: $::log"

# Frame teardown with several locals: which order do the variables go in?
set ::log {}
proc q {} {
    set a 1 ; set b 2 ; set c 3
    trace add variable a unset {rec A}
    trace add variable b unset {rec B}
    trace add variable c unset {rec C}
}
q
puts "frame-multi: $::log"

# Namespace teardown.
set ::log {}
namespace eval ::ns {
    variable v 1
    trace add variable v unset {rec n1}
    trace add variable v unset {rec n2}
}
namespace delete ::ns
puts "ns: $::log"

# Unsetting a whole array with both array-level and element traces.
set ::log {}
array set arr {k 1 j 2}
trace add variable arr unset {rec ARR}
trace add variable arr(k) unset {rec K}
trace add variable arr(j) unset {rec J}
unset arr
puts "array-unset: $::log"

# Legacy registration on an array element: vivification + error shape.
proc cb args {}
trace variable y(2) r cb
puts "vivify: [array exists y] [info exists y] [info exists y(2)]"
puts "read: [catch {set y(2)} m]:$m"
puts "vinfo: [trace vinfo y(2)] / [trace info variable y(2)]"
set scalar x
puts "scalar-elem: [catch {trace variable scalar(2) r cb} m]:$m"
puts "missing-ns: [catch {trace variable ::no::such::x r cb} m]:$m"
