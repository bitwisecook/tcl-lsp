puts [list IDENTITY [info patchlevel]]
proc when args {return ROOT}
proc :when args {return LONE_COLON}
namespace eval named {proc when args {return INTERIOR}}
foreach head {when ::when :::when ::::when :::::when :when named::when named:::when ::named::when when:: ::when:: ::} {
    set queryCode [catch {namespace which -command $head} query]
    set callCode [catch {uplevel #0 [list $head EVENT {}]} result]
    puts [list GEOMETRY $head $queryCode $query $callCode $result]
}
