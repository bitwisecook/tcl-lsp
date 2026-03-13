expr {1 ? 2 ? 3 : 4 : 5}
expr {0 ? 42 : 1 ? 99 : 0}
set a "no"
expr {1 ? $a : -54}
