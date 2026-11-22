proc later {} {return later}
interp alias {} al {} later
rename later later2
puts [catch al m]:$m ;# CALL al
