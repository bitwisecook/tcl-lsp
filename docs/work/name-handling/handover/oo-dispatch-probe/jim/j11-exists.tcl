# Jim exists
set x 1
proc q {} {}
puts [exists x]:[exists y]:[exists -command puts]:[exists -proc puts]:[exists -proc q]
