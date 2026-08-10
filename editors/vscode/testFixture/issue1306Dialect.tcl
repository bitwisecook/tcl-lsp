namespace eval ::I1306 {}
oo::class create ::I1306::Mother {
    superclass oo::class
}
proc ::I1306::NSNormalize {namespace qualname} {
    if {![string match ::* $qualname]} {
        set qualname ${namespace}::$qualname
    }
    regsub -all {::+} $qualname "::"
}
proc ::I1306::mkdialect {name} {
    set NSPACE [NSNormalize [uplevel 1 {namespace current}] $name]
    ::I1306::Mother create ${NSPACE}::class {
        superclass ::I1306::Mother
    }
}
::I1306::mkdialect ::I1306::D
