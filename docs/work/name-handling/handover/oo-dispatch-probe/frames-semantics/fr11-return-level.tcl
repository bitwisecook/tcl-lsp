proc a {} { b; return after }
proc b {} { return -level 2 early }
puts [a]
proc c {} { catch { return -level 1 x } r o; return [dict get $o -code]:[dict get $o -level] }
puts [c]
