proc b6 {} {global g; return $g}
set g 6
set r [b6]; puts "<$r>"
set g 123
set r [b6]; puts "<$r>"
unset g
