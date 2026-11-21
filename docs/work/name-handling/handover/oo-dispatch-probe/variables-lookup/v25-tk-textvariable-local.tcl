proc p {} { set name local; return [info exists ::name]:$name ;# READ
}
puts [p]
