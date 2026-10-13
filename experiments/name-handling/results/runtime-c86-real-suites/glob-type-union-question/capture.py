#!/usr/bin/env python3
"""Original provider probes on controlled ordinary fixture entries only."""
import hashlib,json,os,pathlib,subprocess,tempfile,time
ROOT=pathlib.Path('/workspace/tcl-lsp');HERE=pathlib.Path(__file__).resolve().parent;OUT=HERE/'capture-01';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
providers=[(v,ROOT/f'tmp/tcl{v}/unix/tclsh',ROOT/f'tmp/tcl{v}') for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']]+[('jim-pinned',ROOT/'tmp/jimtcl-oracle/jimsh',ROOT/'tmp/jimtcl-oracle')]
OUT.mkdir(exist_ok=False)
env=dict(os.environ)
for name in ['TCL_LIBRARY','TCLLIBPATH','TCL_BACKEND_CONSTRAINTS','TCL_TEST_VERBOSE','TCL_TEST_CONSTRAINTS']:env.pop(name,None)
records=[]
with tempfile.TemporaryDirectory(prefix='glob-union-original-') as workspace:
 fixture=pathlib.Path(workspace)/'fixture';fixture.mkdir()
 for name,mode in [('plain.txt',0o600),('executable.txt',0o700),('readonly.txt',0o400)]:
  path=fixture/name;path.write_bytes(b'ordinary fixture\n');path.chmod(mode)
 directory=fixture/'directory';directory.mkdir();directory.chmod(0o700)
 for name,binary,tree in providers:
  dest=OUT/name;dest.mkdir();sourcepaths=[tree/'generic/tclFileName.c',tree/'unix/tclUnixFile.c',tree/'tests/fileName.test'] if name!='jim-pinned' else [tree/'glob.tcl',tree/'jim.c']
  source={str(p.relative_to(ROOT)):sha(p) for p in sourcepaths if p.is_file()};before=sha(binary);argv=[str(binary),str(HERE/'input.tcl'),str(fixture)];start=time.monotonic()
  run=subprocess.run(argv,cwd=workspace,env=env,capture_output=True,timeout=20);(dest/'stdout.txt').write_bytes(run.stdout);(dest/'stderr.txt').write_bytes(run.stderr)
  record={'provider':name,'argv':argv,'exit_code':run.returncode,'seconds':round(time.monotonic()-start,3),'binary_sha256_before':before,'binary_sha256_after':sha(binary),'original_source_sha256':source,'input_sha256':sha(HERE/'input.tcl'),'stdout_sha256':sha(dest/'stdout.txt'),'stderr_sha256':sha(dest/'stderr.txt'),'row_count':sum(line.startswith(b'ROW ') for line in run.stdout.splitlines())};(dest/'record.json').write_text(json.dumps(record,indent=2)+'\n');records.append(record);print(name,run.returncode,record['row_count'],flush=True)
 for path in fixture.iterdir():
  if path.is_file():path.chmod(0o600)
provenance={'records':records,'fixture':'Three new regular files and one new directory in a temporary tree, ordinary synthetic content. Only fixture names/results emitted; no device nodes, physical device or host security data. Synthetic file modes control executable/readonly cases; r/w observations retain actual provider access semantics.','axes':'Fresh five C release providers and pinned Jim. No Runtime build/run/source changes; F5 not run and no hosted/native parity inferred.','environment':'Inherited environment not logged or dumped; named Tcl library/overlay/test switches removed so original providers select their own libraries.'};(OUT/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
