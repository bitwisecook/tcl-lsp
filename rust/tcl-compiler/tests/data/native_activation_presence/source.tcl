set ::b PRESENT; proc f {} {list [info exists b] [info exists Params(key)]}; puts [f]
