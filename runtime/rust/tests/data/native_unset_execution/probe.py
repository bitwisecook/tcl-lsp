from pathlib import Path
import subprocess,json,hashlib,fcntl,time
out=Path('/workspace/.proofs/2286-runtime-native-unset-traces');out.mkdir(exist_ok=True)
cases=[
 ('undefined-trace','set events {}; proc cb {n i op} {lappend ::events [list $n $i $op]; return -code error TRACE}; trace variable ghost u cb; set c [catch {unset ghost} m]; list $c $m $events [trace vinfo ghost]'),
 ('undefined-trace-quiet','set events {}; proc cb {n i op} {lappend ::events [list $n $i $op]; return -code error TRACE}; trace variable ghost u cb; set c [catch {unset -nocomplain ghost} m]; list $c $m $events [trace vinfo ghost]'),
 ('quiet-namespace','set errorCode ORIGINAL; set c [catch {unset -nocomplain ::missing::v} m]; list $c $m $errorCode'),
 ('quiet-array-namespace','set errorCode ORIGINAL; set c [catch {unset -nocomplain ::missing::a(k)} m]; list $c $m $errorCode'),
 ('sequential-quiet','set ::first 1; set ::second 1; set ::events {}; proc name {which} {lappend ::events [list $which [info exists ::first]]; return {}}; proc p {} {unset -nocomplain ::first[name ONE] ::second[name TWO]}; p; list $events [info exists ::first] [info exists ::second]'),
 ('element-order','set ::seen BEFORE; proc idx {value} {set ::seen $value; return $value}; proc p {} {unset a([idx FIRST]) a([idx SECOND])}; set c [catch {p} m]; list $c $m $::seen'),
]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
(out/'cases.tsv').write_text(''.join(n+'\t'+s.encode().hex()+'\n' for n,s in cases))
rows=[]
engines=[(v,Path('/tmp/2286-oracles/bin')/('tclsh'+v)) for v in ['8.4','8.5','8.6','9.0','9.1']]+[('jim',Path('/tmp/2286-oracles/jimtcl/jimsh'))]
for engine,binary in engines:
 result_rows=[]
 for name,body in cases:
  if engine!='8.4':
   body=body.replace('trace variable ghost u cb','trace add variable ghost unset cb').replace('trace vinfo ghost','trace info variable ghost')
  script=out/(engine+'-'+name+'.tcl')
  rows_source=body.encode().hex()
  script.write_text('set code [catch {'+body+'} result]\nbinary scan $result H* hex\nputs "$code\\t$hex"\n')
  slot=None
  while slot is None:
   for i in range(2):
    candidate=open(f'/workspace/.proofs/2286-test-slot-{i}.lock','a')
    try:fcntl.flock(candidate,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:candidate.close();continue
    slot=candidate;break
   if slot is None:time.sleep(.1)
  start=time.monotonic()
  try:r=subprocess.run([str(binary),str(script)],capture_output=True,timeout=60)
  finally:slot.close()
  assert r.returncode==0 and not r.stderr,(engine,name,r.stderr)
  output=r.stdout.decode().strip();assert len(output.split('\t'))==2,(engine,name,output)
  result_rows.append(name+'\t'+rows_source+'\t'+output+'\n')
  rows.append(dict(engine=engine,case=name,command=[str(binary),str(script)],source_sha256=sha(script),binary_sha256=sha(binary),exit=r.returncode,stderr_bytes=len(r.stderr),stdout=output,seconds=time.monotonic()-start,budget=60))
 (out/(engine+'.tsv')).write_text(''.join(result_rows))
 print(engine,len(result_rows),flush=True)
(out/'manifest.json').write_text(json.dumps(dict(scope='Actual undefined-unset traces, quiet unknown namespaces, and sequential compiler operand evaluation across all six native engines; completion/result/trace event observations only',cases_sha256=sha(out/'cases.tsv'),rows=rows),indent=2)+'\n')
