set f [apply {dir {
    set path [file join $dir lib]
    puts $path
}} /tmp]
