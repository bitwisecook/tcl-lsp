proc p {} { set n 42; return [format "%05d|%x|%.2f|%s" $n $n 3.14159 abc] }
puts [p]
