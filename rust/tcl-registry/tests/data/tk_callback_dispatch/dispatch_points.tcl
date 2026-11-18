# Run under each release's own wish (Tk 8.4.20-9.1.0) with xvfb-run; the outputs
# beside this script are tk<version>.txt captures of that run.
# For each case: register a callback, run the command under test, and report
# whether the callback ran before that command returned.
proc case {name setup action} {
    set ::log {}
    if {[catch {uplevel #0 $setup} err]} { puts "$name: setup-error $err"; return }
    set ::log {}
    if {[catch {uplevel #0 $action} err]} { puts "$name: action-error $err"; return }
    puts "$name: [expr {[llength $::log] ? "runs" : "deferred"}]"
    catch {update}
}
proc vcmd {args} { lappend ::log vcmd; return 1 }
proc hit {args} { lappend ::log hit }

case register.entry.vcmd.validate-all {} {entry .r1 -validate all -validatecommand vcmd}
case register.entry.vcmd.textvariable-validate-none {set ::tv1 x} {entry .r2 -textvariable ::tv1 -validatecommand vcmd}
case register.entry.vcmd.textvariable-validate-all {set ::tv2 x} {entry .r3 -textvariable ::tv2 -validate all -validatecommand vcmd}
case register.entry.vcmd.textvariable-validate-key {set ::tv3 x} {entry .r4 -textvariable ::tv3 -validate key -validatecommand vcmd}
case register.entry.vcmd.textvariable-unset-validate-all {catch {unset ::tv4}} {entry .r5 -textvariable ::tv4 -validate all -validatecommand vcmd}
case register.spinbox.vcmd.textvariable-validate-all {set ::tv5 x} {spinbox .r6 -textvariable ::tv5 -validate all -validatecommand vcmd}
case register.ttk-entry.vcmd.textvariable-validate-all {set ::tv6 x} {ttk::entry .r7 -textvariable ::tv6 -validate all -validatecommand vcmd}
case register.button.command {} {button .r8 -command hit}
case register.after {} {after 0 hit}
case register.after-idle {} {after idle hit}
case register.bind {} {bind all <<Probe>> hit}
case register.trace {} {trace variable ::tw w hit}

case dispatch.update {after 0 hit} {update}
case dispatch.update-idletasks.idle {after idle hit} {update idletasks}
case dispatch.update-idletasks.timer {after 0 hit} {update idletasks}
case dispatch.vwait {after 0 {hit; set ::done 1}} {vwait ::done}
case dispatch.tkwait-variable {after 0 {hit; set ::done2 1}} {tkwait variable ::done2}
case dispatch.after-cancel {set ::aid [after 0 hit]} {after cancel $::aid}
case dispatch.after-info {after 0 hit} {after info}
case dispatch.event-generate {frame .eg; bind .eg <<Probe>> hit} {event generate .eg <<Probe>>}
case dispatch.event-generate-when-tail {frame .eg2; bind .eg2 <<Probe>> hit} {event generate .eg2 <<Probe>> -when tail}
case dispatch.destroy {frame .d; bind .d <Destroy> hit} {destroy .d}
case dispatch.entry-insert.validate-key {entry .ei -validate key -validatecommand vcmd} {.ei insert 0 x}
case dispatch.entry-delete.validate-key {entry .ed -validate none -validatecommand vcmd; .ed insert 0 xy; .ed configure -validate key} {.ed delete 0}
case dispatch.entry-validate {entry .ev -validatecommand vcmd} {.ev validate}
case dispatch.entry-insert.validate-none {entry .en -validatecommand vcmd} {.en insert 0 x}
case dispatch.entry-configure-validate {entry .ec -validatecommand vcmd} {.ec configure -validate all}
case dispatch.button-invoke {button .b -command hit} {.b invoke}
case dispatch.checkbutton-invoke {checkbutton .cb -command hit} {.cb invoke}
case dispatch.checkbutton-variable-write {checkbutton .cb2 -command hit -variable ::cbv} {set ::cbv 1}
case dispatch.scale-set {scale .s -command hit -from 0 -to 10; pack .s; update} {.s set 5}
case dispatch.scrollbar-set {scrollbar .sb -command hit} {.sb set 0 0.5}
case dispatch.listbox-insert.yscrollcommand {listbox .lb -yscrollcommand hit; pack .lb; update} {.lb insert end a b c}
case dispatch.trace-write {trace variable ::tw2 w hit} {set ::tw2 1}
case dispatch.trace-read {set ::tw3 0; trace variable ::tw3 r hit} {set ::tw3}
case dispatch.textvariable-write.validate-all {set ::tv7 x; entry .tv -textvariable ::tv7 -validate all -validatecommand vcmd} {set ::tv7 y}
case dispatch.focus {entry .f; pack .f; update; bind .f <FocusIn> hit} {focus -force .f}
case dispatch.grab {toplevel .g; update; bind .g <Enter> hit; bind all <Leave> hit} {grab .g}
case dispatch.bind-query {frame .bq; bind .bq <<Probe>> hit} {bind .bq}
case dispatch.winfo {after 0 hit} {winfo exists .}
case dispatch.wm-title {after 0 hit} {wm title . x}
puts "TK [package require Tk]"
exit
