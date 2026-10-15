set ::observations {}
proc during {name1 name2 operation} {
    foreach script {
        {namespace exists [format "%s%s" retir ing]}
        {namespace eval [format "%s%s" retir ing] {}}
    } {
        set code [catch {uplevel #0 $script} value]
        lappend ::observations [list $code $value]
    }
}
namespace eval retiring {
    variable sentinel HELD
    trace variable sentinel u ::during
}
set code [catch {namespace delete retiring} value]
puts [list delete $code $value observations $::observations final [namespace exists retiring]]
