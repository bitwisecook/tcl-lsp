puts "provider [info patchlevel]"
set n WRONG
set {$n} RIGHT
set {scalar(open} SCALAR
puts "interpolation ${n}:${$n}:${scalar(open}"
set result [list $n [list ${$n}]]
puts "nested-fold $result"
set pattern {ROOT}
proc regex_local {} {
    set pattern {LOCAL}
    puts "regex-local [regexp $pattern LOCAL]"
}
regex_local
if {[llength [info commands dict]]} {
    set d {alpha A beta B}
    dict with d {puts "dictionary-key $alpha $beta"}
    puts "dictionary-retained $alpha $beta"
} else {
    puts "dictionary unsupported"
}
puts "interpreter-create-code [catch {interp create named} created]"
puts "interpreter-created $created"
if {$created eq "named"} {
    puts "interpreter-eval-code [catch {interp eval $created {set x VALUE}} result]"
    puts "interpreter-eval-result $result"
}
