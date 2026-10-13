puts "provider [info patchlevel]"
set x ROOT
namespace eval N {
    variable x NAMESPACE
    namespace eval child {puts "child-holder [namespace current]"; variable x VALUE}
    puts "child-rooted-exists [info exists ::N::child::x]"
    puts "relative-qualified-code [catch {set child::x} result]"
    puts "relative-qualified-result $result"
    proc p {} {
        set x LOCAL
        puts "current0 [uplevel 0 {set x}]"
        puts "root0 [uplevel #0 {set x}]"
        puts "caller1 [uplevel 1 {set x}]"
    }
    p
    variable lambda {{p} {return $p} ::N}
}
if {[llength [info commands apply]]} {
    puts "lambda-code [catch {apply $::N::lambda VALUE} result]"
    puts "lambda-result $result"
} else {
    puts "lambda unsupported"
}
if {[llength [info commands dict]]} {
    set d {root ROOT}
    namespace eval N {
        variable d {local LOCAL}
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
