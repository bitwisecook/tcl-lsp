# Jim references: ref/getref/setref
set r [ref hello mytag]
puts [string match "<reference.<mytag__>*" $r]
puts [getref $r]
setref $r world
puts [getref $r]
