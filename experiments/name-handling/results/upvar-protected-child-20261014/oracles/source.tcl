proc plain {} {set result [catch {upvar 1 target local}]; set local PLAIN}
proc nested {} {set result [catch {catch {upvar 1 target local}}]; set local NESTED}
proc caller {} {
    set target BEFORE
    plain
    puts [list plain $target]
    nested
    puts [list nested $target]
}
caller
