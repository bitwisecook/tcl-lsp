puts [list PROVIDER [info patchlevel]]
proc record {label script} {
    set code [catch {uplevel 1 $script} value]
    puts [list $label $code $value]
}
interp create kid
kid eval {interp hide {} set hiddenSet}
record CHILD_NAMESPACE {kid invokehidden -namespace chosen hiddenSet item CHILD}
record CHILD_NAMESPACE_READ {kid invokehidden hiddenSet ::chosen::item}
record CHILD_ROOT_ABSENT {kid eval {info exists ::item}}
record LAST_GLOBAL {kid invokehidden -namespace chosen -global hiddenSet globalItem GLOBAL}
record GLOBAL_READ {kid invokehidden hiddenSet ::globalItem}
record LAST_NAMESPACE {kid invokehidden -global -namespace :literal: hiddenSet item COLON}
record COLON_READ {kid invokehidden hiddenSet ::\:literal\:::item}
record EMPTY_NAMESPACE {kid invokehidden -namespace {} hiddenSet emptyItem EMPTY}
record EMPTY_NAMESPACE_READ {kid invokehidden hiddenSet ::emptyItem}
record ROOT_RESTORE {kid eval {namespace current}}
interp hide {} namespace hiddenNamespace
record CURRENT_NAMESPACE {interp invokehidden {} hiddenNamespace eval outer {interp invokehidden {} -namespace chosen hiddenNamespace current}}
record CURRENT_ROOT_ABSENT {info exists ::item}
record CURRENT_OUTER_ABSENT {interp invokehidden {} hiddenNamespace eval outer {info exists ::chosen::item}}
interp expose {} hiddenNamespace namespace
interp delete kid
