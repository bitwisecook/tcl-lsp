proc forgetXyce {} {
    uplevel 1 {foreach nameSpc [namespace children ::Foo] {
        namespace forget ${nameSpc}::*
    }}
}

proc bad {} {
    set x 1
    uplevel 1 "puts $x"
}
