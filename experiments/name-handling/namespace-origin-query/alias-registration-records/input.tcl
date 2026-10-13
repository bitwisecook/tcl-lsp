puts [list IDENTITY [info patchlevel]]
proc ::__ar_target {} {return TARGET}
proc ::__ar_observe {label script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    if {$code && [info exists ::errorCode]} {set errorcode $::errorCode}
    puts [list OBSERVATION $label $code $value $errorcode]
}
::__ar_observe first-create {interp alias {} ::original {} ::__ar_target}
::__ar_observe first-rename {rename ::original ::moved}
::__ar_observe original-query {interp alias {} ::original}
::__ar_observe moved-query {interp alias {} ::moved}
::__ar_observe original-target {interp target {} ::original}
::__ar_observe missing-query {interp alias {} ::absent}
::__ar_observe second-create {interp alias {} ::original {} ::__ar_target}
::__ar_observe inventory {lsort [interp aliases {}]}
::__ar_observe original-query-after {interp alias {} ::original}
::__ar_observe prefixed-query {interp alias {} ::::original}
::__ar_observe original-delete {interp alias {} ::original {}}
::__ar_observe moved-after-delete {::moved}
::__ar_observe original-after-delete {::original}
::__ar_observe inventory-after-delete {lsort [interp aliases {}]}

::__ar_observe hidden-create {interp alias {} hiddenRecord {} ::__ar_target}
::__ar_observe hidden-hide {interp hide {} hiddenRecord hiddenSlot}
::__ar_observe hidden-inventory {lsort [interp aliases {}]}
::__ar_observe hidden-query {interp alias {} hiddenRecord}
::__ar_observe hidden-target {interp target {} hiddenRecord}
::__ar_observe hidden-delete {interp alias {} hiddenRecord {}}
::__ar_observe hidden-inventory-after {lsort [interp aliases {}]}

foreach name {a e i m q u y A E I M Q U Y z01 z02 z03 z04 z05 z06 z07 z08 z09 z10 z11 z12 z13 z14 z15 z16 é ö} {
    interp alias {} $name {} ::__ar_target
}
::__ar_observe unsorted-growth-inventory {interp aliases {}}
::__ar_observe relocate-two-records {rename a movedA; rename z01 movedZ; interp aliases {}}
::__ar_observe delete-records {interp alias {} a {}; interp alias {} z01 {}; interp alias {} i {}; interp aliases {}}
::__ar_observe recreate-records {interp alias {} a {} ::__ar_target; interp alias {} z01 {} ::__ar_target; interp aliases {}}
::__ar_observe append-collision-record {rename a movedAgain; interp alias {} a {} ::__ar_target; interp aliases {}}
