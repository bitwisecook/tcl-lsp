proc ::__nh_measure {id script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    set errorinfo {}
    if {$code && [info exists ::errorCode]} {set errorcode $::errorCode}
    if {$code && [info exists ::errorInfo]} {set errorinfo $::errorInfo}
    puts [list $id $code $value $errorcode $errorinfo]
}
foreach context [list :: ::__nh_outer] {
    namespace eval $context {
        foreach variant {empty absolute-root nul-suffix} {
            switch $variant {
                empty {set name {}}
                absolute-root {set name ::}
                nul-suffix {set name "[format %c 0]suffix"}
            }
            set identity [list [namespace current] $variant]
            ::__nh_measure [list $identity exists-before] [list namespace exists $name]
            ::__nh_measure [list $identity eval] [list namespace eval $name {namespace current}]
            ::__nh_measure [list $identity exists-after] [list namespace exists $name]
        }
    }
}
