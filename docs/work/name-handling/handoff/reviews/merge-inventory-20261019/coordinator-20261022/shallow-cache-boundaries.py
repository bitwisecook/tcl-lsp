from pathlib import Path
import json,os,subprocess,sys
before=set(Path('/tmp/naming-shallow-before1455').read_text().splitlines())
path=Path('.git/shallow');original=path.read_text();current=set(original.splitlines())
assert before and before<=current
if '--check-tip' in sys.argv:
    assert '--dry-run' in sys.argv
    current.add(sys.argv[sys.argv.index('--check-tip')+1])
env=dict(os.environ,GIT_NO_LAZY_FETCH='1')
proc=subprocess.Popen(['git','cat-file','--batch'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,env=env)
cache={};missing=set()
def parents(sha):
    if sha in cache:return cache[sha]
    assert len(sha)==40 and all(c in '0123456789abcdef' for c in sha)
    proc.stdin.write((sha+'\n').encode());proc.stdin.flush()
    fields=proc.stdout.readline().split()
    if fields==[sha.encode(),b'missing']:
        missing.add(sha);cache[sha]=None;return None
    assert len(fields)==3 and fields[0]==sha.encode() and fields[1]==b'commit',fields
    size=int(fields[2]);assert 0<size<10000000
    data=proc.stdout.read(size);assert len(data)==size and proc.stdout.read(1)==b'\n'
    result=[line.split()[1].decode('ascii') for line in data.split(b'\n\n',1)[0].splitlines() if line.startswith(b'parent ')]
    cache[sha]=result;return result
def cached_ancestry(start):
    pending=[start];seen=set()
    while pending:
        sha=pending.pop()
        if sha in before or sha in seen:continue
        seen.add(sha)
        result=parents(sha)
        if result is None:return False
        pending.extend(result)
    return True
try:
    removable={sha for sha in current-before if cached_ancestry(sha)}
finally:
    proc.stdin.close();assert proc.wait()==0
remaining=current-removable
assert remaining and before<=remaining and path.read_text()==original
dry='--dry-run' in sys.argv
if removable and not dry:
    temporary=path.with_name('shallow.naming-tmp')
    with temporary.open('x') as dest:dest.write(''.join(sha+'\n' for sha in sorted(remaining)))
    assert path.read_text()==original
    temporary.replace(path)
print(json.dumps({'preserved_existing_boundaries':len(before),'restored_cached_tip_boundaries':len(removable),'missing_commit_objects':sorted(missing),'still_shallow':bool(remaining),'cached_commit_headers':len(cache),'dry_run':dry}))
if missing:raise SystemExit(3)
