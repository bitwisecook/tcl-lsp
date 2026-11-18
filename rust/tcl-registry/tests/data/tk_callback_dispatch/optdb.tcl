proc vcmd {args} { lappend ::log vcmd; return 1 }
set v x
option add *Entry.textVariable v
option add *Entry.validate all
set ::log {}
entry .e -validatecommand vcmd
puts "optiondb-both: [expr {[llength $::log] ? "runs" : "deferred"}] tv=[.e cget -textvariable] validate=[.e cget -validate]"
option clear
option add *Spinbox.textVariable v
set ::log {}
spinbox .s -validate all -validatecommand vcmd
puts "optiondb-textvariable-only spinbox: [expr {[llength $::log] ? "runs" : "deferred"}]"
set ::log {}
entry .e2 -textvar v -validate al -validatecommand vcmd
puts "abbreviated -textvar/-validate al: [expr {[llength $::log] ? "runs" : "deferred"}]"
set ::log {}
entry .e3 -textvariable v -validate none -validatecommand vcmd
puts "explicit none: [expr {[llength $::log] ? "runs" : "deferred"}]"
puts "TK [package require Tk]"
exit
