import pathlib,json,hashlib,subprocess,fcntl,time
root=pathlib.Path('/workspace/tcl-lsp');out=pathlib.Path(__file__).parent
sha=lambda p:hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
records=[]
for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim0.84']:
 jim=v.startswith('jim'); tree=pathlib.Path('/tmp/2286-oracles/jimtcl') if jim else root/'tmp'/('tcl'+v); source=out/('jim-probe.c' if jim else 'probe.c');lib=tree/('libjim.a' if jim else 'unix/libtcl'+v[:3]+'.a'); binary=out/('probe-'+v)
 cmd=['cc','-DSTDC_HEADERS=1','-DHAVE_UNISTD_H=1','-I'+str(tree if jim else tree/'generic')]+([] if jim else ['-I'+str(tree/'unix')])+[str(source),str(lib),'-lm','-ldl','-lpthread','-lz','-o',str(binary)]
 comp=subprocess.run(cmd,capture_output=True,text=True,timeout=60)
 record=dict(version=v,compile=cmd,compile_exit=comp.returncode,compile_stderr=comp.stderr,source_sha256=sha(source),header_sha256=sha(tree/('jim.h' if jim else 'generic/tclInt.h')),library_sha256=sha(lib),native_parser_sha256=sha(tree/('jim.c' if jim else 'generic/tclParse.c')),native_compiler_sha256=sha(tree/('jim.c' if jim else 'generic/tclCompCmds.c')),original_budget=60)
 if comp.returncode==0:
  slot=None
  while slot is None:
   for n in range(2):
    candidate=open('/workspace/.proofs/2286-test-slot-'+str(n)+'.lock','a')
    try:fcntl.flock(candidate,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:candidate.close();continue
    slot=candidate;break
   if slot is None:time.sleep(.1)
  try:
   start=time.monotonic();run=subprocess.run([str(binary)],capture_output=True,text=True,timeout=60);record.update(exit=run.returncode,seconds=time.monotonic()-start,binary_sha256=sha(binary),stdout=run.stdout,stderr=run.stderr);(out/(v+'.tsv')).write_text(run.stdout)
  finally:slot.close()
 records.append(record);(out/'manifest.json').write_text(json.dumps(records,indent=2)+'\n')
 print(v,'compile',comp.returncode,'run',record.get('exit'),comp.stderr[:180],flush=True)
