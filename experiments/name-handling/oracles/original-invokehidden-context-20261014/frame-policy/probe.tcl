puts [list PROVIDER [info patchlevel]]
interp hide {} info hiddenInfo
proc observe {} {
    list [interp invokehidden {} -global hiddenInfo level] [interp invokehidden {} hiddenInfo level]
}
puts [list CALLER [observe]]
puts [list ROOT [interp invokehidden {} -global hiddenInfo level]]
interp expose {} hiddenInfo info
