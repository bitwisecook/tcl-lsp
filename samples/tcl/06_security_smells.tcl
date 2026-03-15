# 06: Intentionally suspicious patterns for security diagnostics
# Do not copy these patterns into real code.

set user_script "puts hacked"
eval $user_script

set template $argv
subst $template

set cmd "ls -la"
open "|$cmd"

set maybe_path $argv0
source $maybe_path

uplevel 1 $user_script
regexp {(a+)+$} $user_script
