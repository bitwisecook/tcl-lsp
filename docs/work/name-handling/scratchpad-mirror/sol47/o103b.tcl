proc p {} {return P}
proc q {} {return Q}
interp alias {} r {} list
r p oldp
r q p
puts [p]
