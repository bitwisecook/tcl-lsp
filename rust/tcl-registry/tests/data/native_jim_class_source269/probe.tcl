puts [list version [info patchlevel] initial_class [info commands class] alias_command [info commands alias]]
set package_code [catch {package require oo} package_result]
puts [list oo_package $package_code $package_result class_command [info commands class]]
if {[llength [info commands class]] == 0} {
    puts [list jim_class_controls not_applicable no_class_command]
} else {
    proc observe {label script} {
        set code [catch {uplevel 1 $script} result]
        puts [list $label $code $result]
    }
    observe sequential_members {
        class Accum {x 1}
        Accum method first {} {return FIRST}
        Accum method second {} {return SECOND}
        set obj [Accum new]
        list [$obj first] [$obj second] [$obj get x]
    }
    observe two_word_before_after {
        proc {Chrono before} {} {return BEFORE}
        class Chrono {x 1}
        proc {Chrono after} {} {return AFTER}
        set obj [Chrono new]
        list [$obj before] [$obj after]
    }
    observe moved_earlier_two_word {
        proc {MovedProc before} {} {return BEFORE}
        rename {MovedProc before} held_before
        class MovedProc {}
        set obj [MovedProc new]
        $obj before
    }
    observe bases_current {
        class Base {base 11}
        Base method inherited {} {return BASE}
        class Child {Base} {own 22}
        set obj [Child new]
        list [$obj inherited] [$obj get base] [$obj get own]
    }
    observe bases_moved_factory {
        class MoveBase {base 31}
        MoveBase method inherited {} {return MOVE_BASE}
        rename MoveBase HeldBase
        class MoveChild {HeldBase} {own 32}
        set obj [MoveChild new]
        list [$obj inherited] [$obj get base] [$obj get own]
    }
    observe bases_command_alias {
        class AliasBase {base 41}
        AliasBase method inherited {} {return ALIAS_BASE}
        alias AliasName AliasBase
        class AliasChild {AliasName} {own 42}
        set obj [AliasChild new]
        list [$obj inherited] [$obj get base] [$obj get own]
    }
    observe bases_known_deleted {
        class DeletedBase {base 51}
        rename DeletedBase {}
        class DeletedChild {DeletedBase} {}
    }
    observe bases_known_nonclass {
        class ShadowedBase {base 61}
        proc ShadowedBase args {return NON_CLASS}
        class ShadowedChild {ShadowedBase} {}
    }
}
