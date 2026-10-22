import gdb
f=gdb.newest_frame(); rows=[]
while f is not None:
    try: sp=int(f.read_register('rsp'))
    except Exception: sp=0
    rows.append((sp, f.name() or '?')); f=f.older()
prev=None
for sp,n in rows:
    d = (sp-prev) if prev else 0
    print("%8d %s" % (d, n[:150])); prev=sp
print("TOTAL", rows[-1][0]-rows[0][0], "FRAMES", len(rows))
