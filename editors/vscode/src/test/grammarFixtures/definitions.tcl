# Marked-up grammar fixture. Each `#  ^^^ scope` line asserts that the caret
# span, in the code line above it, carries that TextMate scope. Assertion lines
# are ordinary Tcl comments, so this file is still a valid script.
#
# These hold the contract from #903/#904: the static grammar must agree with the
# semantic-token layer, never contradict it.

proc greet {name {greeting hello}} {
#^^^ storage.type.function.tcl
#    ^^^^^ entity.name.function.tcl
#           ^^^^ variable.parameter.tcl
#                 ^^^^^^^^ variable.parameter.tcl
    puts "$greeting $name"
#   ^^^^ support.function.tcl
}

# A non-local exit is a control keyword, not a library call — `catch` and
# `error` are two halves of one construct and must match (#904).
catch { error boom } msg
#^^^^ keyword.control.tcl
#       ^^^^^ keyword.control.tcl
throw {MY ERR} oops
#^^^^ keyword.control.tcl
return -code error x
#^^^^^ keyword.control.tcl

# A computing builtin stays a function.
lappend acc 1
#^^^^^^ support.function.tcl
string length $s
#^^^^^ support.function.tcl

oo::class create Account {
#^^^^^^^^^^^^^^^ support.class.tcl
#                ^^^^^^^ entity.name.class.tcl
    superclass Base
#   ^^^^^^^^^^ storage.modifier.tcl
#              ^^^^ entity.other.inherited-class.tcl
    constructor {balance} {
#   ^^^^^^^^^^^ storage.type.function.tcl
#                ^^^^^^^ variable.parameter.tcl
        my variable b
    }
    method deposit {amount} {
#   ^^^^^^ storage.type.function.tcl
#          ^^^^^^^ entity.name.function.tcl
#                   ^^^^^^ variable.parameter.tcl
        incr b $amount
#       ^^^^ support.function.tcl
    }
}

# Issue #637: a bareword `proc` used as data must not open a definition, or it
# would swallow the following quote and derail every string after it.
dict set frame proc "x"
#              ^^^^ keyword.other.tcl
