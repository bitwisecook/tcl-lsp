# The error codes each release sets for a bad index and an expression that
# does not parse, and the message the expression's parse error carries.
proc show {script} {
    set ::errorCode NONE
    set code [catch {uplevel 1 $script} result]
    puts [string map [list \n { | }] "$code <$result> <$::errorCode>"]
}
show {lindex {a b} x}
show {string index abc x}
show {lrange {a b} x y}
show {set l {a b}; lindex $l x}
show {string range abc 0 x}
show {expr {1 +}}
show {expr {(1}}
show {expr {1 ? 2}}
show {expr {nosuch}}
show {expr {0x}}
show {set e {1 +}; expr $e}
show {expr {1 ** 2}}
proc p {} {
    show {lindex {a b} x}
    show {string range abc x 1}
    show {expr {1 +}}
    show {expr {(1}}
}
p
