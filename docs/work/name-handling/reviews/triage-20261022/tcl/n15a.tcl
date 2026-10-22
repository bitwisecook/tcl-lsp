proc a {} {return OLD}
proc longName {} {return NEW}
puts [a]
puts [longName]
