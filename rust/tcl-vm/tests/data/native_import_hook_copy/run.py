import pathlib,subprocess,json,hashlib,os,fcntl,time
root=pathlib.Path('/workspace/.proofs/2286-native-import-hook-copy');repo=pathlib.Path('/workspace/tcl-lsp');source=root/'probe.c'
manifest=root/'manifest.json';assert not manifest.exists()
sha=lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
rows=[]
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 tree=repo/'tmp'/('tcl'+version); short='.'.join(version.split('.')[:2]);library=tree/'unix'/('libtcl'+short+'.a');binary=root/version
 command=['cc','-I'+str(tree/'unix'),'-I'+str(tree/'generic'),str(source),str(library),'-lm','-ldl','-lpthread','-lz','-o',str(binary)]
 c=subprocess.run(command,capture_output=True,timeout=60);(root/(version+'-compile.log')).write_bytes(c.stdout+c.stderr)
 row=dict(version=version,compile_command=command,compile_exit=c.returncode,library=str(library),library_sha256=sha(library),header_sha256=sha(tree/'generic'/'tclInt.h'))
 if c.returncode==0:
  slot=None
  while slot is None:
   for n in range(2):
    p=open('/workspace/.proofs/2286-test-slot-'+str(n)+'.lock','a')
    try:fcntl.flock(p,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:p.close();continue
    slot=p;break
   if slot is None:time.sleep(.1)
  try:r=subprocess.run([str(binary)],capture_output=True,timeout=60,env=dict(os.environ,TCL_LIBRARY=str(tree/'library')))
  finally:slot.close()
  (root/(version+'.tsv')).write_bytes(r.stdout);(root/(version+'-stderr.log')).write_bytes(r.stderr)
  row.update(exit=r.returncode,binary_sha256=sha(binary),observations=r.stdout.decode().splitlines(),stderr=r.stderr.decode())
 rows.append(row)
manifest.write_text(json.dumps(dict(source=str(source),source_sha256=sha(source),runs=rows),indent=2)+'\n')
print(json.dumps([dict(version=x['version'],compile=x['compile_exit'],exit=x.get('exit'),observations=len(x.get('observations',[])),stderr=x.get('stderr','')) for x in rows]))
