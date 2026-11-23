# An uncaught error in file mode: stderr traceback and exit status.
proc inner {} { error "inner failed" }
proc outer {} { inner }
puts before
outer
