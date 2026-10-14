proc b {} {return k}
set i k
set a(k) V
puts [subst -nocommands {$a([b])}]
puts [subst -nobackslashes {$a(\x6b)}]
puts [subst -nocommands {$a($i)}]
