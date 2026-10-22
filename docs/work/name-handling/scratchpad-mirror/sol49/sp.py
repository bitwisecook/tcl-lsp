import gdb
f = gdb.newest_frame()
rows=[]
i=0
while f is not None and i < 400:
    name = f.name() or "??"
    try:
        sp = int(f.read_register("sp"))
    except Exception:
        sp = 0
    rows.append((i, name, sp))
    f = f.older(); i+=1
prev=None
for i,name,sp in rows:
    if prev is not None:
        print("%4d %8d %s" % (i, sp-prev, name[:110]))
    prev=sp
