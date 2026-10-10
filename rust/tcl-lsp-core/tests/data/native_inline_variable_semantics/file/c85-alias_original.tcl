set x 1; proc change {} {upvar #0 x alias; set alias 2}; change; puts $x
