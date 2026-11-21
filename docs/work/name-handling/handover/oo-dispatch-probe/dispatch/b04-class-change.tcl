oo::class create A { method a {} { return a } }
oo::class create B { method b {} { return b } }
A create o
oo::objdefine o class B
puts [o b]
puts [catch {o a} m]:$m ;# BAD no longer A
