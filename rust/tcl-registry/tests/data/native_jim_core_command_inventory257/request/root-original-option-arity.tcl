proc ::r2286_global {} {return GLOBAL}
proc {::r2286 spaced} {} {return SPACE}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
}
set abbreviated [catch {::info commands -al r2286*} abbreviated_result]
set ordinary_extra [catch {::info commands r2286* EXTRA} ordinary_result]
set all_extra [catch {::info commands -all r2286* EXTRA} all_result]
list $abbreviated $abbreviated_result $ordinary_extra $ordinary_result $all_extra $all_result
