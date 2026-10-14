if {[catch {info patchlevel} version]} {catch {info version} version}
puts [list version $version]
proc namedWriter {name value} {upvar 1 $name target; set target $value}
proc callerWriterProbe {} {set body {return original}; namedWriter body {return replacement}; set body}
proc dynamicWriterProbe {name} {set body {return original}; set $name {return replacement}; set body}
proc literalDollarProbe {} {set body original; set {$body} separate; list $body [set {$body}]}
proc escapedNameProbe {} {set body original; set b\x6fdy replacement; set body}
namespace eval custom {
    proc set {name value} {return ignored}
    proc probe {} {set body {return original}; info exists body}
}
foreach test {callerWriterProbe {dynamicWriterProbe body} literalDollarProbe escapedNameProbe custom::probe} {
    set code [catch {eval $test} result]
    puts [list control $test $code $result]
}
