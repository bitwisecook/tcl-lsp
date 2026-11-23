# The default unknown guards against recursion through unknown_pending.
set auto_index(selfref) {selfref}
puts "selfref: [catch {selfref} m] $m"
