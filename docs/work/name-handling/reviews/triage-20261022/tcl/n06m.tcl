oo::class create K {
    method old {} {return OLD}
    method dest {} {return DEST}
}
set k [K new]
puts "[$k old] [$k dest]"
