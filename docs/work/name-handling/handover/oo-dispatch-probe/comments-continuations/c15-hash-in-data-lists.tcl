array set a {
    # not-a-comment
    k 1
}
puts [lsort [array names a]]
set d [dict create {*}{
    # x
    k v
}]
puts [dict keys $d]
