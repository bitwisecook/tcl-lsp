namespace eval ::a { namespace unknown {apply {{args} {return [list unk {*}$args]}}} ; puts [namespace unknown]; puts [frob 1] }
puts [namespace unknown]
