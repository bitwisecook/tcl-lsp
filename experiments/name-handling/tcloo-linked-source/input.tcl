puts [list IDENTITY [info patchlevel]]
set hasClass [expr {[llength [info commands ::oo::class]] != 0}]
set hasLink [expr {[llength [info commands ::oo::Helpers::link]] != 0}]
puts [list AVAILABILITY TclOO $hasClass stockLink $hasLink]
if {$hasClass} {
    oo::class create LinkProbe {
        method target {} {return TARGET}
        method :target {} {return LONE_TARGET}
        method ::target {} {return ABSOLUTE_TARGET_DATA}
        method {} {} {return EMPTY_TARGET}
        method inspect {descriptor called} {
            set linkCode [catch {link $descriptor} linkResult]
            set lookupCode [catch {namespace which -command $called} lookupResult]
            set callCode [catch {uplevel 0 [list $called]} callResult]
            return [list $linkCode $linkResult $lookupCode $lookupResult $callCode $callResult]
        }
    }
    foreach {tag descriptor called} {
        single target target
        pair {shortcut target} shortcut
        lone-colon {:shortcut target} :shortcut
        absolute {::link_probe_root target} ::link_probe_root
        odd-leading {:::link_probe_root target} ::link_probe_root
        interior {nested:::shortcut target} nested::shortcut
        literal-marker {$literal target} {$literal}
        empty-alias {{} target} {}
        literal-target {targetAlias :target} targetAlias
        absolute-target-data {absoluteTargetAlias ::target} absoluteTargetAlias
        empty-target {emptyTargetAlias {}} emptyTargetAlias
        invalid-empty-list {} absent
        invalid-three-items {one two three} absent
    } {
        set object [LinkProbe new]
        set code [catch {$object inspect $descriptor $called} observation]
        puts [list LINK $tag $descriptor $called $code $observation]
        $object destroy
    }
    set outsideCode [catch {::oo::Helpers::link target} outsideResult]
    puts [list OUTSIDE $outsideCode $outsideResult]
}
