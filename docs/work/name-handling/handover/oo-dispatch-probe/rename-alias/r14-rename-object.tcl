oo::class create C { method m {} {return m} }
C create o
rename o o2
puts [list [o2 m] [catch {o m} e] [info object isa object o2]]
