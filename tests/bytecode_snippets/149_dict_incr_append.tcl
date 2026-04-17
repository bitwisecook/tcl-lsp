proc test {} {
    set d [dict create count 0 log ""]
    dict incr d count
    dict incr d count 5
    dict append d log "entry1 "
    dict append d log "entry2"
    return $d
}
