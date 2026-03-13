set d [dict create a 1 b 2 c 3]
set sum 0
dict for {k v} $d {
    incr sum $v
}
