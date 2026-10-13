puts "provider [info patchlevel]"
set x ROOT
namespace eval N {
    set x NAMESPACE
    namespace eval child {set x VALUE}
    puts "relative-qualified $child::x"
    proc p {} {
        set x LOCAL
        puts "current0 [uplevel 0 {set x}]"
        puts "root0 [uplevel #0 {set x}]"
        puts "caller1 [uplevel 1 {set x}]"
    }
    p
    set lambda {{p} {return $p} ::N}
}
if {[llength [info commands apply]]} {
    puts "lambda [apply $::N::lambda VALUE]"
} else {
    puts "lambda unsupported"
}
if {[llength [info commands dict]]} {
    set d {root ROOT}
    namespace eval N {
        set d {local LOCAL}
        dict with d {puts "dictionary-key $local"}
    }
    proc dictionary_local {} {dict with d {set root}}
    puts "dictionary-proc-code [catch dictionary_local result]"
    puts "dictionary-proc-result $result"
} else {
    puts "dictionary unsupported"
}
proc linked {} {global x; return $x}
puts "linked-value [linked]"
set which live
set {targets($which)} FALSE
set targets(live) CORRECT
puts "array-literal [set {targets($which)}]"
puts "array-substituted $targets($which)"
set {scalar(open} SCALAR
puts "braced-scalar ${scalar(open}"
