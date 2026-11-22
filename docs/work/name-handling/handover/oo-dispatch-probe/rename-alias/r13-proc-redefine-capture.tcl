proc p {} {return one}
set cb [list p]
proc p {} {return two}
puts [{*}$cb]
interp alias {} al {} p
proc p {} {return three}
puts [al]
