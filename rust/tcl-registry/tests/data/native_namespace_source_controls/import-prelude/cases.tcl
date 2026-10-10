proc row {name script} {
    set code [catch {uplevel 1 $script} result]
    set length [string length $result]
    set units {}
    for {set index 0} {$index < $length} {incr index} {
        scan [string index $result $index] %c unit
        lappend units $unit
    }
    puts [list $name $code $length $units]
}
proc auto_import {pattern} {return -code error PRELOAD91}
namespace eval ::Prelude91 {}
row import-prelude-before-missing-namespace {
    namespace eval ::Prelude91 {namespace import ::Missing91::p}
}
