# Adapted from overrides::ResolveCmdName in Dechcaudron/overrides fe18ee3 (MIT, Copyright (c) 2026 Héctor Barreras Almarcha).
proc ResolveCmdName {cmdName {callerNameResLvl 0}} {
    incr callerNameResLvl
    set resolvedName [uplevel $callerNameResLvl info commands \
        [list $cmdName]]
    if {"$resolvedName" eq ""} { error "none" }
    set resolvedNameIsFullyQualified [\
        string match ::* $resolvedName]
    if !$resolvedNameIsFullyQualified {
        set callerNamespace [uplevel $callerNameResLvl \
            namespace current]
        set resolvedName "${callerNamespace}::$resolvedName"
    }
    return $resolvedName
}
puts [ResolveCmdName puts]
