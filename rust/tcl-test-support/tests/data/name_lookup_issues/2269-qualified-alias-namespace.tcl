interp alias {} ::vendor::unpack {} lindex
puts [namespace exists ::vendor]
puts [::vendor::unpack {a b c} 1]
puts [interp alias {} ::vendor::unpack]
