proc p {x} {return p:$x}
rename p q
puts [q 1] ;# CALL q
