set ::flag 1
if {$::flag} {proc set {args} {return CUSTOM}}
set x 1
puts $x
