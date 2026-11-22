set v V
puts [list [subst {$v [string length ab] \t}] [subst -nocommands {$v [x] \t}] [subst -novariables {$v [string length ab]}] [subst -nobackslashes {\t$v}]]
