# A braced after script inside a proc that reads the proc's local: error at run time, reported by bgerror.
proc bgerror {m} { set ::bg $m; set ::done 1 }
proc arm {} { set local 5; after 0 {puts $local} }
arm
after 200 {set ::done timeout}
vwait done
puts $bg
