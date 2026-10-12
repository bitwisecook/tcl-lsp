puts [list version [info patchlevel]]
proc observe {label script} {
    set status [catch {uplevel 1 $script} result]
    set code {}
    if {$status} {set code $::errorCode}
    puts [list observation $label status $status result $result errorCode $code]
}
observe capability {namespace ensemble exists ::r2286_probe_unused}
observe create-bare {namespace eval ::r2286_nestedBare {proc x args {return $args}; namespace export x; namespace ensemble create}}
observe invoke-bare-created {::r2286_nestedBare x payload}
    namespace eval ::r2286_nested {}
    observe create-immediate-option {
        namespace eval ::r2286_nested {namespace ensemble create -command ::r2286_plain -map {x ::list}}
        namespace ensemble configure ::r2286_plain -map
    }
    observe create-dash-command-map {
        namespace eval ::r2286_nested {namespace ensemble create -command ::-map -map {x ::list}}
        namespace ensemble configure ::-map -map
    }
    observe configure-command-name-map-data {
        namespace ensemble configure ::-map -map
    }
    observe configure-unqualified-map-data {
        namespace ensemble configure -map -map
    }
    observe configure-command-name-namespace-data {
        namespace eval ::r2286_nested {namespace ensemble create -command ::-namespace -map {x ::list}}
        namespace ensemble configure -namespace -map
    }
    observe configure-option-after-command {
        namespace ensemble configure ::r2286_plain -prefixes 0
        namespace ensemble configure ::r2286_plain -prefixes
    }
    observe configure-invalid-option-after-command {
        namespace ensemble configure ::r2286_plain -command ::r2286_other
    }
    observe exists-dash-map-is-command-name {namespace ensemble exists -map}
    observe exists-dash-namespace-is-command-name {namespace ensemble exists -namespace}
    observe exists-does-not-scan-options {namespace ensemble exists ::r2286_plain -namespace}
    observe create-rejects-configure-only-option {
        namespace eval ::r2286_nested {namespace ensemble create -namespace ::r2286_nested}
    }
    foreach name {::r2286_plain ::-map ::-namespace ::r2286_other} {
        catch {rename $name {}}
    }
    namespace delete ::r2286_nested
catch {namespace delete ::r2286_nestedBare}
