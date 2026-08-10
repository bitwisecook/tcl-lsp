namespace eval ::I1303Echo {}
oo::class create ::I1303Echo::Meta {
    superclass oo::class
    method unknown {w args} { return $w }
}
::I1303Echo::Meta create ::I1303Echo::Widget {
    method go {} { return went }
}
set w [::I1303Echo::Widget .w1303echo]
puts [$w go]
