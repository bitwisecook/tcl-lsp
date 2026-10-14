interp alias {} branch {} if 1
interp alias {} chained {} branch
puts [branch {set result FIRST}]
puts [chained {set result SECOND}]
puts [interp alias {} branch]
puts [interp alias {} chained]
