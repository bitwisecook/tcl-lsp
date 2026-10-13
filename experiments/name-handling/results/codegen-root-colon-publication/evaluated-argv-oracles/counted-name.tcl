set calls 0
proc next_name {} {incr ::calls; return :audit}
proc [next_name] {} {return COUNTED}
puts [list [:audit] $calls [info commands audit]]
