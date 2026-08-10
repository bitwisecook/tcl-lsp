# Issue #1332 TN control — nothing here requires Tk and nothing is sourced, so
# the W120 must still fire. Following `source` must not silence W120 globally.
winfo exists .l
