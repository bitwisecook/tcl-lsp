package require Tk

ttk::frame .main \
    -padding 8
ttk::label .main.message -text {Ready}
ttk::button .main.save -text {Save}
grid .main.message -row 0 -column 0
grid .main.save -row 1 -column 0 -sticky ew
