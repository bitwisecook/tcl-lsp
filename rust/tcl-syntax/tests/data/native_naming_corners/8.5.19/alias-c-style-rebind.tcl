set code [catch {proc target args {return OLD}; interp alias {} a {} target; rename target original; proc target args {return NEW}; a} result]
binary scan $result H* hex
puts [list $code $hex]
