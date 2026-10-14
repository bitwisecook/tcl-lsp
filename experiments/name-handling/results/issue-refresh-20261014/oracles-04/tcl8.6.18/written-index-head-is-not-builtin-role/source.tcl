set a(key) VALUE
rename ::set ::original_set
proc ::set {args} {return key}
puts $a([set k])
