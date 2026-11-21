oo::object create o
oo::objdefine o {
    variable n
    method set1 {} { set n 1 }
    method get {} { return $n }
}
o set1
puts [o get]
