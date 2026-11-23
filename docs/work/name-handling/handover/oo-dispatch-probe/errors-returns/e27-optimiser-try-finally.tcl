# The finally clause of try always runs, even when the body returns.
proc t {} { try { return 1 } finally { puts fin } }
puts [t]
