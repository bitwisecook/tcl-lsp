puts [list IDENTITY [info patchlevel]]
set hasClass [expr {[llength [info commands ::oo::class]] != 0}]
set hasLink [expr {[llength [info commands ::oo::Helpers::link]] != 0}]
puts [list AVAILABILITY TclOO $hasClass stockLink $hasLink]
if {$hasLink} {
    oo::class create LinkPublicationProbe {
        method target {} {return TARGET}
        method inspect {} {
            link {:shortcut target}
            set ordinaryCode [catch {shortcut} ordinaryResult]
            set colonCode [catch {:shortcut} colonResult]
            set zeroCode [catch {link} zeroResult]
            return [list ordinary $ordinaryCode $ordinaryResult loneColon $colonCode $colonResult zeroArguments $zeroCode $zeroResult]
        }
    }
    set object [LinkPublicationProbe new]
    puts [list PUBLICATION [$object inspect]]
    $object destroy
}
