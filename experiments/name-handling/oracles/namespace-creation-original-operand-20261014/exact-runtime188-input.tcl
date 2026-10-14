
set ::observations {}
proc during {name1 name2 operation} {
    foreach script {
        {interp invokehidden {} -namespace retiring hiddenNamespace current}
        {interp invokehidden {} hiddenNamespace exists retiring}
        {interp invokehidden {} hiddenNamespace eval retiring {}}
    } {
        set code [catch {uplevel #0 $script} value]
        lappend ::observations [list $code $value]
    }
}
namespace eval retiring {
    variable sentinel HELD
    trace add variable sentinel unset ::during
}
interp hide {} namespace hiddenNamespace
set deleted [catch {interp invokehidden {} hiddenNamespace delete retiring} message]
list $deleted $message $::observations [interp invokehidden {} hiddenNamespace exists retiring]
