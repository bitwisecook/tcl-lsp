proc gen {} { yield [info level]; set ::cl [info level]; yield [uplevel 1 {info exists outer}]; return done }
proc driver {} { set outer 1; coroutine co gen; return [co]:[co] }
puts [driver]
puts [list [info level] [catch co]]
