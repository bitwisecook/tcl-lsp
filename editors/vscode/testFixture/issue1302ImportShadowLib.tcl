# Fixture for issue #1302 (see issue1302ImportShadow.test.ts).
#
# `::Shadow` exports two names: one that a fresh Tcl interpreter already holds
# as a builtin (`set`), and one it does not (`mything`).  Real Tcl refuses to
# import the first and installs the second.
#
# Oracle, tclsh 9.0.4 and 8.6.14, byte-identical:
#
#   namespace import ::Shadow::*
#     -> can't import command "set": already exists
#   namespace origin ::set        -> ::set
#
namespace eval ::Shadow {
    proc set {name value} {
        return SHADOW
    }
    proc mything {} {
        return MINE
    }
    namespace export set mything
}
