from pathlib import Path
import subprocess,json,hashlib,fcntl,time
out=Path('/workspace/.proofs/2286-native-unset-options');out.mkdir(exist_ok=True)
cases=[
 ('none','unset'),
 ('unknown','unset -bad x'),
 ('defined-unknown','set -bad BAD;set x X;unset -bad x;list [info exists -bad] [info exists x]'),
 ('repeated','set -nocomplain KEEP;set x X;unset -nocomplain -nocomplain x;list [info exists -nocomplain] [info exists x]'),
 ('only-quiet','unset -nocomplain'),
 ('only-end','unset --'),
 ('quiet-end','set -nocomplain KEEP;unset -nocomplain -- -nocomplain;info exists -nocomplain'),
 ('end-quiet','set -nocomplain KEEP;unset -- -nocomplain missing'),
 ('unknown-prefix','unset -nocomp missing'),
 ('nul-quiet','set flag [binary format H* 2d6e6f636f6d706c61696e007461696c];unset $flag missing'),
 ('nul-end','set flag [binary format H* 2d2d007461696c];set -nocomplain KEEP;unset $flag -nocomplain;info exists -nocomplain'),
]
rows=[];sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
for engine,binary in [(v,Path('/tmp/2286-oracles/bin')/('tclsh'+v)) for v in ['8.4','8.5','8.6','9.0','9.1']]+[('jim',Path('/tmp/2286-oracles/jimtcl/jimsh'))]:
 results=[]
 for label,source in cases:
  script=out/f'{engine}-{label}.tcl';script.write_text('set c [catch {'+source+'} m]\nbinary scan $m H* h\nputs "$c\\t$h"\n')
  slot=None
  while slot is None:
   for i in range(2):
    lock=open(f'/workspace/.proofs/2286-test-slot-{i}.lock','a')
    try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:lock.close();continue
    slot=lock;break
   if slot is None:time.sleep(.1)
  start=time.monotonic()
  try:r=subprocess.run([str(binary),str(script)],capture_output=True,timeout=60)
  finally:slot.close()
  assert r.returncode==0 and not r.stderr,(engine,label,r.stderr)
  output=r.stdout.decode().rstrip('\n');assert len(output.split('\t'))==2,(engine,label,output)
  results.append(label+'\t'+source.encode().hex()+'\t'+output+'\n')
  rows.append(dict(engine=engine,case=label,command=[str(binary),str(script)],source_sha256=sha(script),binary_sha256=sha(binary),stdout=output,exit_code=r.returncode,stderr_bytes=len(r.stderr),budget=60,seconds=time.monotonic()-start))
 (out/f'{engine}.tsv').write_text(''.join(results));print(engine,len(results),flush=True)
(out/'manifest.json').write_text(json.dumps(dict(scope='Original unset command flags/arity/unknown prefixes and dynamic raw-NUL options; completion/result only',rows=rows),indent=2)+'\n')
