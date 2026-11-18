# Run under each release's own wish (Tk 8.4.20-9.1.0) with xvfb-run; the outputs
# beside this script are tk<version>.txt captures of that run.
# With a timer and an idle callback pending, run one command and report
# whether either pending callback ran before it returned.
wm geometry . 200x200+0+0; update
proc hit {args} { lappend ::log $args }
proc case {name cmd} {
    set ::log {}
    after 0 [list hit timer]
    after idle [list hit idle]
    if {[catch {uplevel #0 $cmd} err]} { puts "$name: error [string range $err 0 60]"; update; return }
    set seen $::log
    update
    puts "$name: [expr {[llength $seen] ? "dispatches $seen" : "none"}]"
}
case after.ms {after 20}
case after.ms-script {after 10 {set ::unused 1}}
case after.idle-script {after idle {set ::unused 1}}
case after.cancel {after cancel [after 1000 {set ::unused 1}]}
case after.info {after info}
case tkwait.visibility {toplevel .v; tkwait visibility .v}
case selection.get-own {selection own .; catch {selection get}}
case clipboard.get-own {clipboard clear; clipboard append x; clipboard get}
case update {update}
case update.idletasks {update idletasks}
puts "TK [package require Tk]"
exit
