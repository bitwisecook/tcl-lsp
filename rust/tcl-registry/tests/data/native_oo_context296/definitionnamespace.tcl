set available [expr {[llength [info commands oo::class]] > 0}]
if {!$available} {list NOT_APPLICABLE stock-TclOO-unavailable} else {
    namespace eval foodef {}
    oo::class create Meta
    set code [catch {oo::define Meta {definitionnamespace foodef}} value]
    set query [catch {info class definitionnamespace Meta} stored]
    list $code $value $query $stored
}
