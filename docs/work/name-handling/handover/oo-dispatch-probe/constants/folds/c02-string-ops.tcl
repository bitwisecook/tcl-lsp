proc p {} {
    set a [string toupper abc]
    set b [string repeat ab 3]
    set c [string map {a x} banana]
    set d [string range hello 1 3]
    set e [string length "a b"]
    return [list $a $b $c $d $e]
}
puts [p]
