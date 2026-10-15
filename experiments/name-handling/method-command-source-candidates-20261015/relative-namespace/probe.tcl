catch {fconfigure stdout -buffering none}
puts [list version [info patchlevel]]
if {[llength [info commands ::oo::class]] == 0} {
    puts {TCLOO unavailable}
} else {
    proc ::helper {} {return GLOBAL}
    ::oo::class create RootClass {method use {} {helper}}
    RootClass create rootObject
    set code [catch {rootObject use} value]
    puts [list bare-root-before-shadow $code $value]
    set object_namespace [info object namespace rootObject]
    proc ${object_namespace}::helper {} {return OBJECT}
    set code [catch {rootObject use} value]
    puts [list actual-object-shadow $code $value]
    namespace eval Outer {
        proc helper {} {return OUTER}
        ::oo::class create C {method use {} {helper}}
    }
    ::Outer::C create outerObject
    set code [catch {outerObject use} value]
    puts [list outer-declaration-namespace $code $value]
    set code [catch {::Outer::helper} value]
    puts [list separate-outer-proc $code $value]
}
