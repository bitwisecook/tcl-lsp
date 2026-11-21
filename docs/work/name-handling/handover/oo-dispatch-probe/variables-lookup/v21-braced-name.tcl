proc p {} { set {a b} braced; return ${a b} ;# READ
}
puts [p]
