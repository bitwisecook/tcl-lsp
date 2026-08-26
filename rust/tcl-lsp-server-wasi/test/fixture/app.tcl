# The only document the e2e opens.
#
# `helper` is declared in lib.tcl, which the session never sends: resolving the
# call below means the server followed the `source` edge and read the sibling
# off the preopened filesystem.

source lib.tcl

proc main {} {
    return [helper 21]
}

main
