import re,sys
# commands whose body holds a backslash-newline inside a braced word of a row
# (brace depth >= 2 below the `speclib` body, i.e. inside a row word of a
# command body), versus only at row level.
for path in sys.argv[1:]:
    src=open(path).read()
    depth=0; cmd=None; cmd_depth=None; hits=set(); i=0; n=len(src)
    line_start=0
    while i<n:
        c=src[i]
        if c=='\\':
            if i+1<n and src[i+1]=='\n' and cmd and depth>cmd_depth:
                hits.add(cmd)
            i+=2; continue
        if c=='{':
            # detect `command NAME {` / `subcommand NAME {` at this position
            ls=src.rfind('\n',0,i)+1
            m=re.match(r'\s*command\s+(\S+)',src[ls:i])
            if m and cmd is None:
                cmd=m.group(1); cmd_depth=depth+1
            depth+=1
        elif c=='}':
            depth-=1
            if cmd and depth<cmd_depth: cmd=None
        i+=1
    for h in sorted(hits): print(path.split('/')[-1].replace('.tclspec','.snap'), h)
