proc helper {} {return up}
proc run {} { uplevel 1 {helper} }
puts [run]
