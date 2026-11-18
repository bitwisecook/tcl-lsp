proc p {{a b} {$tag} é unused} {return "${a b}${$tag}${é}"}
puts [p 1 2 3 4]
puts [info args p]
