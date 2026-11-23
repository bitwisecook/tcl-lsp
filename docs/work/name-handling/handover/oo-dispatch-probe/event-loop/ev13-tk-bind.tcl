# Tk callbacks are deferred bodies run in the global frame (static fixture; needs Tk).
package require Tk
proc handler {w} { set ::clicked [list $w [info level]] }
button .b -text go -command {handler .b}
bind .b <Button-1> {set ::bound %W}
pack .b
after 0 {.b invoke; set ::done 1}
vwait done
puts $clicked
destroy .
