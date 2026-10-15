foreach c {upv orig whic del insc cod} {puts [list $c [catch [list namespace $c 84] r] $r]}
proc {namespace upv} {args} {return SHORT}
puts [namespace upv 84]
