proc p {} { return [string length abc] }
rename string _string
proc string {a} { return shadowed:$a }
puts [catch p m]:$m ;# BAD string now takes one argument
rename string {}
rename _string string
