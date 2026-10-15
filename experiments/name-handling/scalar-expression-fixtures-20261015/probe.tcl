puts [list PATCHLEVEL [info patchlevel]]
set x one
set {a b} one
set {a{b}c} one
set café one
foreach {label expression} {
    OLD_BARE {$x eq one}
    QUOTED {$x eq "one"}
    SPACED_NAME {${a b} eq "one"}
    NESTED_NAME {${a{b}c} eq "one"}
    UNICODE_NAME {$café eq "one"}
    LITERAL_OR {$x eq "one||two" || $x eq "one"}
} {
    set code [catch {expr $expression} result]
    puts [list $label $code $result]
}
