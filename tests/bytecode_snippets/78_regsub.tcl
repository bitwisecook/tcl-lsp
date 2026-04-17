set s "Hello World"
regsub {World} $s {Tcl} result
regsub -all {[aeiou]} $s {*} result
