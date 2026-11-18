# Run under each release's own wish (Tk 8.4.20-9.1.0) with xvfb-run; the outputs
# beside this script are tk<version>.txt captures of that run.
proc case {name setup action} {
    set ::log {}
    if {[catch {uplevel #0 $setup} err]} { puts "$name: setup-error $err"; return }
    set ::log {}
    if {[catch {uplevel #0 $action} err]} { puts "$name: action-error $err"; return }
    puts "$name: [expr {[llength $::log] ? "runs" : "deferred"}]"
    catch {update}
}
proc hit {args} { lappend ::log hit }
proc vcmd {args} { lappend ::log vcmd; return 1 }
wm geometry . 200x200+0+0; update
case register.trace-add {} {trace add variable ::tw write hit}
case dispatch.trace-add-write {trace add variable ::tw2 write hit} {set ::tw2 1}
case dispatch.trace-add-read {set ::tw3 0; trace add variable ::tw3 read hit} {set ::tw3}
case dispatch.trace-add-unset {set ::tw4 0; trace add variable ::tw4 unset hit} {unset ::tw4}
case dispatch.event-generate.mapped {frame .eg -width 50 -height 50; pack .eg; update; bind .eg <<Probe>> hit} {event generate .eg <<Probe>>}
case dispatch.event-generate.mapped-when-tail {frame .eg2 -width 50 -height 50; pack .eg2; update; bind .eg2 <<Probe>> hit} {event generate .eg2 <<Probe>> -when tail}
case dispatch.event-generate.mapped-key {entry .eg3; pack .eg3; update; focus -force .eg3; update; bind .eg3 <Key-a> hit} {event generate .eg3 <Key-a>}
case dispatch.event-generate.unmapped {frame .eg4; bind .eg4 <<Probe>> hit} {event generate .eg4 <<Probe>>}
case dispatch.focus.mapped {entry .f; pack .f; update; bind .f <FocusIn> hit} {focus -force .f}
case dispatch.tkwait-window {toplevel .tw; after 0 {hit; destroy .tw}} {tkwait window .tw}
case dispatch.spinbox-invoke {spinbox .sp -command hit -from 0 -to 5} {.sp invoke buttonup}
case dispatch.spinbox-insert.validate-key {spinbox .sv -validate key -validatecommand vcmd} {.sv insert 0 1}
case dispatch.ttk-button-invoke {ttk::button .tb -command hit} {.tb invoke}
case dispatch.ttk-entry-insert.validate-key {ttk::entry .te -validate key -validatecommand vcmd} {.te insert 0 x}
case dispatch.ttk-entry-textvariable-write.validate-all {set ::tv8 x; ttk::entry .te2 -textvariable ::tv8 -validate all -validatecommand vcmd} {set ::tv8 y}
case dispatch.menu-invoke {menu .m; .m add command -label a -command hit} {.m invoke 0}
case dispatch.radiobutton-invoke {radiobutton .rb -command hit -variable ::rbv -value 1} {.rb invoke}
case dispatch.scale-set.then-update {scale .s -command hit -from 0 -to 10; pack .s; update} {.s set 5; update idletasks}
case dispatch.listbox-yscroll.then-update-idletasks {listbox .lb -yscrollcommand hit; pack .lb; update} {.lb insert end a b c; update idletasks}
case dispatch.uplevel-eval {after 0 hit} {eval {set x 1}}
puts "TK [package require Tk]"
exit
