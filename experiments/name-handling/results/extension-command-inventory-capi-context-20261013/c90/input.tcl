load [file join [file dirname [info script]] selected.so] Inventorycontext
proc capture {label script} {
    set code [catch {uplevel #0 $script} result]
    puts [list ROW $label $code $result]
}
namespace eval N: {}
namespace eval N {}
namespace eval N: {::inventoryInstallOriginal}
capture entered-original {namespace inscope N: x}
capture rooted-joined {N:::x}
capture rooted-ordinary {N::x}
capture root-bare {x}
capture scoped-report {namespace inscope N: {namespace which -command x}}
capture joined-report {namespace which -command N:::x}
capture entered-qualified {namespace inscope N: {namespace inscope child y}}
capture root-qualified {N:::child::y}
capture root-child {child::y}
capture scoped-child-report {namespace inscope N: {namespace which -command child::y}}
