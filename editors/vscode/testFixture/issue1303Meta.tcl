namespace eval ::I1303 {}
oo::class create ::I1303::Meta {
    superclass oo::class
    method unknown {w args} {
        [self] create ::$w {*}$args
        return $w
    }
    unexport new unknown
}
