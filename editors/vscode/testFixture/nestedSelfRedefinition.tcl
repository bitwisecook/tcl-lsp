# A nested, cross-namespace proc self-redefinition (issue #923 audit idx 45).
#
# The real corpus shape is nico-robert/ticklecharts: a qualified
# `proc ticklecharts::activate` whose body declares an unqualified
# `proc activate`, redefining the SAME command in the enclosing namespace.
#
# tclsh 9.0.4 and 8.6.16 agree byte-for-byte: the two calls print
#   activating for the first time
#   already activated
# and afterwards `info commands ::activate` is empty while
# `info commands ::ticklecharts::activate` reports the command — so both
# `proc` headers below declare one and the same command, and a reference
# query from either must answer with the same call sites.
#
# Line numbers are load-bearing for nestedSelfRedefinition.test.ts.
namespace eval ticklecharts {}
proc ticklecharts::activate {bool} {
    if {$bool} {
        proc activate {bool} {puts "already activated"}
        puts "activating for the first time"
    }
}
ticklecharts::activate 1
ticklecharts::activate 1
