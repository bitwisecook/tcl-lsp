proc ::r2286_global {} {return GLOBAL}
proc {::r2286 spaced} {} {return SPACE}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
}
namespace eval ::r2286_holder {
    set abbreviated [catch {::info -nons commands -al r2286*} abbreviated_result]
    set extra [catch {::info -nons commands -all r2286* EXTRA} extra_result]
    list $abbreviated $abbreviated_result $extra $extra_result
}
