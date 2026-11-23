# | and |& pipelines; |& also routes stderr into the next stage.
puts "pipe=[exec echo a b c | cat | cat]"
puts "pipe-amp=[catch {exec cat no-such-file |& cat} m] [string match *no-such-file* $m]"
puts "trailing-bar=[catch {exec echo a |} m] $m"
puts "leading-bar=[catch {exec | echo a} m] $m"
puts "bar-as-word=[catch {exec echo {a|b}} m] $m"
