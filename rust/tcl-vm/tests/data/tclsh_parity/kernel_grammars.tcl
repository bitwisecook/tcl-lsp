# Each release's `scan` integer width, `clock` surface and `binary` field set
# (docs/design/registry/model/370-dialect-grammars.md), at top level and
# compiled in a procedure.
proc show {script} {
    set ::errorCode NONE
    set code [catch {uplevel 1 $script} result]
    puts "$code <$result> <$::errorCode>"
}
# A packed result as hex, so the output stays text.
proc showbin {script} {
    set ::errorCode NONE
    if {[set code [catch {uplevel 1 $script} result]] == 0} {
        binary scan $result H* result
    }
    puts "$code <$result> <$::errorCode>"
}
foreach {input conversion} {
    ffffffff %x 1ffffffff %x 7fffffff %x 80000000 %x 100000000 %x -ffffffff %x
    3000000000 %u 4294967295 %u 4294967296 %u -1 %u -3000000000 %u
    4294967295 %d -2147483649 %d 8589934591 %d 99999999999999999999 %d
    37777777777 %o 0xffffffff %i ffffffff %hx
    ffffffffffffffff %x -8000000000000001 %x -10000000000000000 %x
    ffffffff %lx -1 %lu 3000000000 %lu 99999999999999999999 %ld
} {
    show [list scan $input $conversion]
}
show {clock add 0 1 day -gmt 1}
show {clock add 0 1 d -gmt 1}
show {clock milliseconds x}
show {clock clicks -x}
show {clock seconds x}
show {clock scan 1970-01-02 -format %Y-%m-%d -gmt 1}
show {clock scan 1970 -base 0 -gmt 1 -format %Y}
show {clock format 0 -locale C -gmt 1}
show {clock format 0 -g 1 -f %Y}
foreach conversion {
    %Ez %EY %Ey %EC %Ex %EX %Ec %EE %Ea %Oy %Om %Od %OH %Ou %Ow %Oe %Ok
    %Ol %Oz %OY %OC %D %x %X %c %r %T %%
} {
    show [list clock format 86400000 -format <$conversion> -gmt 1]
}
foreach field {a w c m t n q Q r R cu au a*u} {
    showbin [list binary format $field 1]
}
show {binary scan [binary format w 1] m x}
show {binary scan ab au x}
proc compiled {} {
    show {clock add 0 1 day -gmt 1}
    show {clock format 0 -format %Ez -gmt 1}
    show {clock scan 1970-01-02 -format %Y-%m-%d -gmt 1}
    show {clock clicks -x}
    show {scan ffffffff %x}
    showbin {binary format m 1}
    puts [list [catch {clock add 0 1 day -gmt 1} r] $r]
}
compiled
puts [list [catch {clock add 0 1 day -gmt 1} r] $r]
