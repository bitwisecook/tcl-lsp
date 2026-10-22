set z x
puts [list [string length "{}$z"] [string length "$z{}"] [string length "a$z"] [string length "{}"] [string length "{$z}"] [string length "a{}$z"] [string length "\{}$z"] [string length "\{\}$z"]]
