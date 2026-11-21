namespace eval :: { set gg 1 }
puts [list [namespace current] [namespace qualifiers ::] [namespace tail ::] $gg [namespace eval ::: {namespace current}]]
