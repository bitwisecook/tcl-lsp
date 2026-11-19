proc peek {n} {upvar 1 $n x; return $x}
proc cell {{name a(1)}} {upvar 1 $name v; incr v}
proc exercise {} {
    set {left right} 7
    set {$sigil} 8
    set a(1) 13
    return [list [peek {left right}] [peek {$sigil}] [cell]]
}
puts [exercise]
