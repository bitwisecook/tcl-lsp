# Which special globals exist (and are arrays) before user code, in a script run.
foreach n {argc argv argv0 auto_path auto_index auto_execs auto_noexec auto_noload
           auto_oldpath env errorInfo errorCode tcl_version tcl_patchLevel tcl_library
           tcl_libPath tclDefaultLibrary tcl_pkgPath tcl_platform tcl_precision
           tcl_rcFileName tcl_interactive tcl_wordchars tcl_nonwordchars
           tcl_traceCompile tcl_traceExec tcl_prompt1 tcl_prompt2 tcl_shlibPath} {
    set e [info exists ::$n]
    set a [array exists ::$n]
    set l [lsearch -exact [info globals] $n]
    puts "$n exists=$e array=$a listed=[expr {$l >= 0}]"
}
