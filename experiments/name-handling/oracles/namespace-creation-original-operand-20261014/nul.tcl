set ::observations {}
proc during {name1 name2 operation} {
    set status [catch {interp invokehidden {} hiddenNamespace eval $::operand {}} value]
    if {$status} {set tuple $::errorCode} else {set tuple {}}
    set ::observations [list $status $value $tuple]
}
namespace eval retiring {
    variable sentinel HELD
    if {[package vcompare [info tclversion] 8.5] < 0} {
        trace variable sentinel u ::during
    } else {
        trace add variable sentinel unset ::during
    }
}
interp hide {} namespace hiddenNamespace
set ::operand "retiring\0ignored"
set status [catch {interp invokehidden {} hiddenNamespace delete retiring} value]
puts [list OBSERVATIONS $::observations]
puts [list FINAL_EXISTS [interp invokehidden {} hiddenNamespace exists retiring]]
