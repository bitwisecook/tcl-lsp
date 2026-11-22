import sys, tkinter
t = tkinter.Tk()
t.tk.eval('set argv [list %s %s only]' % (sys.argv[1], sys.argv[2]))
t.tk.eval(open(sys.argv[3]).read())
