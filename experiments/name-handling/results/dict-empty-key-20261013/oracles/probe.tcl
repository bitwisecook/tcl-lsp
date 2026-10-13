puts [list version [info patchlevel]]
if {[llength [info commands dict]] == 0} {
 puts [list dict unsupported]
} else {
 set d {{} EMPTY}
 dict with d {puts [list empty-during ${}]}
 puts [list empty-after ${}]
}
