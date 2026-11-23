# Commands after return -level 0 run (W210 asks whether y is set).
proc v {} { return -level 0 x ; set y 1 ; return $y }
puts [v]
