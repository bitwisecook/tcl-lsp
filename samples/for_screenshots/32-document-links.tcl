# Every resolvable `source` path is a link — literal or computed — and the
# link is offered on the file name alone, so the surrounding substitution
# keeps its normal syntax colouring.

source lib/util.tcl

source [file join [file dirname [info script]] helpers.tcl]

set dir [file dirname [info script]]
set src [file join $dir src]
source [file join $src parser.tcl]
#                       ^--- cursor

namespace eval ::snit:: {
    variable library [file dirname [info script]]
}
source [file join $::snit::library main1.tcl]

# No link is offered for a path no static reading can place: this one is
# whatever the invoking user typed.
source [file join [lindex $argv 0] plugin.tcl]
