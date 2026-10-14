proc hex {s} {binary scan $s H* result; return $result}
foreach reason {invalid-token hide-non-global expose-non-global missing-command missing-hidden already-hidden command-exists invoke-hidden-missing} {
    interp create kid
    kid eval {proc kept {} {return kept}; proc other {} {return other}; namespace eval N {proc p {} {return NP}}}
    if {$reason == "already-hidden" || $reason == "command-exists"} {interp hide kid kept held}
    set ::errorCode NONE
    set code [catch {
        switch -- $reason {
            invalid-token {interp hide kid kept N::held}
            hide-non-global {interp hide kid N::p held}
            expose-non-global {interp expose kid absent ::destination}
            missing-command {interp hide kid missing held}
            missing-hidden {interp expose kid N::missing allowed}
            already-hidden {interp hide kid other held}
            command-exists {interp expose kid held other}
            invoke-hidden-missing {interp invokehidden kid N::missing}
        }
    } result]
    puts [list $reason $code [hex $result] [hex $::errorCode]]
    interp delete kid
}
