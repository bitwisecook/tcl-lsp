import re, subprocess, sys, json
rev = sys.argv[1]
files = subprocess.run(['git','ls-tree','-r','--name-only',rev,'--','rust/tcl-registry/src','rust/tcl-dialect/src','rust/tcl-core-types/src','rust/tcl-runtime-api/src'],capture_output=True,text=True,cwd='/home/user/tcl-lsp').stdout.split()
out={}
for f in files:
    if not f.endswith('.rs'): continue
    src = subprocess.run(['git','show',f'{rev}:{f}'],capture_output=True,text=True,cwd='/home/user/tcl-lsp').stdout
    for m in re.finditer(r'\bstruct\s+(\w+)(?:<[^>{]*>)?\s*\{', src):
        name=m.group(1); i=m.end(); depth=1; j=i
        while depth and j < len(src):
            c=src[j]
            if c=='{': depth+=1
            elif c=='}': depth-=1
            j+=1
        body=src[i:j-1]
        # strip nested braces content? fields at depth 0 only
        fields=[]; d=0; line=''
        for ln in body.split('\n'):
            s=ln.strip()
            if d==0:
                fm=re.match(r'(?:pub(?:\([^)]*\))?\s+)?(\w+)\s*:(?!:)', s)
                if fm and not s.startswith('//') and not s.startswith('#'):
                    fields.append(fm.group(1))
            d+=ln.count('{')-ln.count('}')
        out.setdefault(name,[]).append(fields)
json.dump(out,sys.stdout)
