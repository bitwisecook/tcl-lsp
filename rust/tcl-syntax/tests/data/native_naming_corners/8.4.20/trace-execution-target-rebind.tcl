set code [catch {proc before {command operation} {rename leaf original; proc leaf {} {return NEW}}; proc leaf {} {return OLD}; trace add execution leaf enter before; leaf} result]
binary scan $result H* hex
puts [list $code $hex]
