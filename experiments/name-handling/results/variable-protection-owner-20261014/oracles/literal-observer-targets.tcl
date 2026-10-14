if {[catch {info patchlevel} version]} {catch {info version} version}
puts [list version $version]
set {} empty-value
set {$counter} sigil-value
set {brace{name}} brace-value
puts [list names [set {}] [set {$counter}] [set {brace{name}}]]
set calls 0
proc note args {global calls; incr calls}
foreach name [list {} {$counter} {brace{name}} ::left::x] {
    namespace eval ::left {}
    set calls 0
    set code [catch {
        trace add variable $name write note
        set $name observed-value
        trace remove variable $name write note
    } result]
    puts [list trace $name $code $calls $result]
}
