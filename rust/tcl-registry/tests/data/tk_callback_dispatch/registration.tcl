# Run under each release's own wish (Tk 8.4.20-9.1.0) with xvfb-run; the outputs
# beside this script are tk<version>.txt captures of that run.
# Does creating a widget run the callback it is given?
proc vcmd {args} { lappend ::log vcmd; return 1 }
proc hit {args} { lappend ::log hit }
proc case {name setup action} {
    set ::log {}
    if {[catch {uplevel #0 $setup} err]} { puts "$name: setup-error [string range $err 0 50]"; return }
    set ::log {}
    if {[catch {uplevel #0 $action} err]} { puts "$name: action-error [string range $err 0 50]"; return }
    puts "$name: [expr {[llength $::log] ? "runs" : "deferred"}]"
}
case scale.command {} {scale .s1 -command hit}
case scale.variable-set.command {set ::sv 3} {scale .s2 -variable ::sv -command hit}
case scale.variable-unset.command {catch {unset ::sv2}} {scale .s3 -variable ::sv2 -command hit}
case ttk-scale.variable-set.command {set ::sv4 3} {ttk::scale .s4 -variable ::sv4 -command hit}
case checkbutton.variable-set.command {set ::cv 1} {checkbutton .c1 -variable ::cv -command hit}
case radiobutton.variable-set.command {set ::rv 1} {radiobutton .r1 -variable ::rv -value 1 -command hit}
case spinbox.command.textvariable {set ::spv 1} {spinbox .sp -textvariable ::spv -command hit -from 0 -to 5}
case listbox.listvariable.yscroll {set ::lv {a b c}} {listbox .lb -listvariable ::lv -yscrollcommand hit}
case entry.textvariable.xscroll {set ::ev abc} {entry .en -textvariable ::ev -xscrollcommand hit}
case entry.vcmd.validate-all {} {entry .e1 -validate all -validatecommand vcmd}
case entry.vcmd.textvariable-validate-all {set ::tv2 x} {entry .e2 -textvariable ::tv2 -validate all -validatecommand vcmd}
case entry.vcmd.textvariable-validate-focus {set ::tv3 x} {entry .e3 -textvariable ::tv3 -validate focus -validatecommand vcmd}
case entry.vcmd.textvariable-validate-none {set ::tv4 x} {entry .e4 -textvariable ::tv4 -validate none -validatecommand vcmd}
case spinbox.vcmd.textvariable-validate-all {set ::tv5 x} {spinbox .p5 -textvariable ::tv5 -validate all -validatecommand vcmd}
case ttk-entry.vcmd.textvariable-validate-all {set ::tv6 x} {ttk::entry .t6 -textvariable ::tv6 -validate all -validatecommand vcmd}
case ttk-combobox.vcmd.textvariable-validate-all {set ::tv7 x} {ttk::combobox .t7 -textvariable ::tv7 -validate all -validatecommand vcmd}
case optiondb.entry {set ::tv8 x; option add *Entry.textVariable ::tv8; option add *Entry.validate all} {entry .e8 -validatecommand vcmd}
case optiondb.entry-explicit-none {} {entry .e9 -validate none -validatecommand vcmd}
case abbreviated.entry {set ::tv10 x; option clear} {entry .e10 -textvar ::tv10 -validate al -validatecommand vcmd}
puts "TK [package require Tk]"
exit
