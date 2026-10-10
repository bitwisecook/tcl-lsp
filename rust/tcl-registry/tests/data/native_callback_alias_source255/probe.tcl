puts [list version [info patchlevel] alias_command [info commands alias]]
proc create_alias {name target captured} {
    if {[llength [info commands alias]] != 0} {
        eval [concat [list alias $name $target] $captured]
    } else {
        eval [concat [list interp alias {} $name {} $target] $captured]
    }
}
proc target args {
    lappend ::observations $args
    return 0
}
set ::observations {}
set code [catch {create_alias cb target {FIXED}; lsort -command cb {2 1}} result]
puts [list single $code $result $::observations]
set ::observations {}
set code [catch {lsort -command {cb BAKED} {2 1}} result]
puts [list baked $code $result $::observations]
set ::observations {}
set code [catch {create_alias inner target {INNER}; create_alias outer inner {OUTER}; lsort -command {outer BAKED} {2 1}} result]
puts [list nested $code $result $::observations]
set ::observations {}
set code [catch {rename cb moved_cb; lsort -command moved_cb {2 1}} result]
puts [list moved_alias $code $result $::observations]
set ::observations {}
set code [catch {rename target moved_target; lsort -command moved_cb {2 1}} result]
puts [list moved_target $code $result $::observations]
proc target {a b c d} {return 0}
set code [catch {lsort -command moved_cb {2 1}} result]
puts [list replacement_four $code $result]
set code [catch {lsort -command {moved_cb BAKED} {2 1}} result]
puts [list replacement_baked_four $code $result]
