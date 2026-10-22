proc p {} {return P}
proc q {} {return Q}
interp alias {} r {} rename
r p oldp
r q p
set value [p]
puts $value
