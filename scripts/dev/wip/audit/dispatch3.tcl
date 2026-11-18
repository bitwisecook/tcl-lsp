proc case {name setup action} {
    set ::log {}
    if {[catch {uplevel #0 $setup} err]} { puts "$name: setup-error $err"; return }
    set ::log {}
    if {[catch {uplevel #0 $action} err]} { puts "$name: action-error $err"; return }
    puts "$name: [expr {[llength $::log] ? "runs" : "deferred"}] $::log"
    catch {update}
}
proc hit {args} { lappend ::log hit }
case dispatch.menu-invoke.after-update {menu .m; .m add command -label a -command hit; update} {.m invoke 0}
case dispatch.menu-invoke.index-a {menu .m2; .m2 add command -label a -command hit; update} {.m2 invoke a}
case dispatch.menu-invoke.then-update {menu .m3; .m3 add command -label a -command hit} {.m3 invoke 0; update idletasks}
case dispatch.menu-invoke.then-update-full {menu .m4; .m4 add command -label a -command hit} {.m4 invoke 0; update}
puts "TK [package require Tk]"
exit
