proc p {{a b} {$tag} é unused} {return "${a b}${$tag}${é}"}
puts [info args p]
puts [info default p a d]:$d
