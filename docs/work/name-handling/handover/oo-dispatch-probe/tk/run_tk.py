import sys, tkinter
root = tkinter.Tk()
script = open(sys.argv[1]).read()
out = []
root.tk.createcommand("emit", lambda *a: out.append(" ".join(a)))
try:
    root.tk.eval("rename puts _puts; proc puts args { emit [lindex $args end] }")
    root.tk.eval(script)
except tkinter.TclError as e:
    out.append("ERROR: " + str(e))
print(" | ".join(out))
