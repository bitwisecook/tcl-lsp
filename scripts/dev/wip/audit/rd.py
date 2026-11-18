import sys
R='crate::retained_dispatch::'
def sub(path, name, value, cite, nth=0):
    s=open(path).read()
    key=f'        name: "{name}",'
    idx=-1
    for _ in range(nth+1):
        idx=s.index(key, idx+1)
    j=s.index('..SubCommand::DEFAULT', idx)
    blk=s[idx:j]
    assert 'retained_dispatch' not in blk, (path,name)
    ind=s.rfind('\n',0,j)+1
    pad=s[ind:j]
    s=s[:j]+f'// {cite}\n{pad}retained_dispatch: {value},\n{pad}'+s[j:]
    open(path,'w').write(s)
def cmd(path, value, cite):
    s=open(path).read()
    i=s.index('pub fn spec() -> CommandSpec {')
    j=s.index('..CommandSpec::DEFAULT', i)
    assert 'retained_dispatch' not in s[i:j]
    ind=s.rfind('\n',0,j)+1; pad=s[ind:j]
    s=s[:j]+f'// {cite}\n{pad}retained_dispatch: {value},\n{pad}'+s[j:]
    open(path,'w').write(s)
