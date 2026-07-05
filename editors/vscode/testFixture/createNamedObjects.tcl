C create c1 1 out 0 -c 1e-9
L create l1 1 out 0 -l 10e-6
C create c2 2 n002 0 -c 1e-9
foreach elem [list c1 l1 c2] {
    $elem actOnParam -set 1
}
