# Print each classic Tk widget's instance methods, one line per widget:
# `WIDGET METHOD ...`, sorted. The list is the one the widget's own command
# reports for an unknown method ("bad option ...: must be ..."), which
# already applies the per-class filtering a shared command table hides
# (tkButton.c). Run under each release's `wish` (issue #1710).
package require Tk
wm withdraw .
set widgets {
    button canvas checkbutton entry frame label labelframe listbox menu
    menubutton message panedwindow radiobutton scale scrollbar spinbox text
    toplevel
}
foreach widget $widgets {
    if {[info commands $widget] eq ""} continue
    set path .probe_$widget
    $widget $path
    catch {$path __no_such_method__} message
    destroy $path
    if {![regexp {must be (.*)$} $message -> list]} {
        puts stderr "$widget: unexpected message: $message"
        continue
    }
    regsub -all {,? or } $list {, } list
    set methods {}
    foreach method [split $list ,] {
        lappend methods [string trim $method]
    }
    puts [join [concat [list $widget] [lsort $methods]] " "]
}
exit
