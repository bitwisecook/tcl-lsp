puts [list IDENTITY [info patchlevel]]
set hasLink [expr {[llength [info commands ::oo::Helpers::link]] != 0}]
puts [list AVAILABILITY stockLink $hasLink]
if {$hasLink} {
    set declarationCode [catch {
        oo::class create LinkDelegateProbe {
            constructor {} {link factory}
            classmethod factory {} {return FACTORY}
            method use {} {return [factory]}
        }
    } declarationResult]
    puts [list DECLARATION $declarationCode $declarationResult]
    if {!$declarationCode} {
        set object [LinkDelegateProbe new]
        puts [list DELEGATE [catch {$object use} result] $result]
        puts [list INSTANCE_METHODS [info class methods LinkDelegateProbe -all]]
        puts [list CLASS_METHODS [info object methods LinkDelegateProbe -all]]
        $object destroy
    }
}
