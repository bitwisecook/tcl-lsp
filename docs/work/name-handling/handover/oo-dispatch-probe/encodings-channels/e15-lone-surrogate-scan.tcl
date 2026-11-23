# lone surrogate and NUL through scan %c
puts [scan \uD800 %c]
puts [scan \x00 %c]
puts [llength [split \uD800 ""]]
puts ok
