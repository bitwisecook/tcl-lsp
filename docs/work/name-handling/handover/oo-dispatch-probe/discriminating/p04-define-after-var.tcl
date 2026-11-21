oo::class create C { method m {} {} }
oo::define C method n {} {}
set o [C new]
$o m
