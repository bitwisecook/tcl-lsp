set code [catch {proc target args {return OLD}; alias a target PREFIX; rename target original; proc target args {return NEW}; a ARG} result]
binary scan $result H* hex
puts [list $code $hex]
