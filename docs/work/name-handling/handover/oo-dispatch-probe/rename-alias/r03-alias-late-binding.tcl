interp alias {} al {} later
puts [catch al m]:$m
proc later {} {return later}
puts [al]
rename later later2
puts [catch al m]:$m
proc later {} {return again}
puts [al]
