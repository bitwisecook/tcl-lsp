proc ::r2286_global {} {return GLOBAL}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
    set code [catch {info commands r2286* EXTRA} result]
    list $code $result
}
