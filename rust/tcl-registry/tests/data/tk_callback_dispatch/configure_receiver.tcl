# Run under each release's own wish (Tk 8.4.20-9.1.0) with xvfb-run; the outputs
# beside this script are tk<version>.txt captures of that run.
# Does a widget's configure or cget run that widget's own stored callback?
proc vcmd {args} { lappend ::log vcmd; return 1 }
proc hit {args} { lappend ::log hit }
proc case {name setup action} {
    set ::log {}
    if {[catch {uplevel #0 $setup} err]} { puts "$name: setup-error [string range $err 0 50]"; return }
    set ::log {}
    if {[catch {uplevel #0 $action} err]} { puts "$name: action-error [string range $err 0 50]"; return }
    puts "$name: [expr {[llength $::log] ? "runs" : "none"}]"
}
set ::v y
case entry.configure-textvariable {entry .e1 -validate all -validatecommand vcmd} {.e1 configure -textvariable ::v}
case entry.configure-validatecommand {entry .e2 -textvariable ::v -validate none} {.e2 configure -validate all -validatecommand vcmd}
case entry.cget {entry .e3 -textvariable ::v -validate all -validatecommand vcmd} {.e3 cget -validate}
case spinbox.configure-textvariable {spinbox .s1 -validate all -validatecommand vcmd} {.s1 configure -textvariable ::v}
case ttk-entry.configure-textvariable {ttk::entry .t1 -validate all -validatecommand vcmd} {.t1 configure -textvariable ::v}
case checkbutton.configure-variable {checkbutton .c1 -command hit} {.c1 configure -variable ::v}
case scale.configure-variable {scale .sc -command hit} {.sc configure -variable ::v}
case button.configure-command {button .b1} {.b1 configure -command hit}
puts "TK [package require Tk]"
exit
