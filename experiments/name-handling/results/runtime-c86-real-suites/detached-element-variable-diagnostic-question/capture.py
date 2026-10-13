#!/usr/bin/env python3
"""Original-provider diagnostic observations; no runtime/compiler build."""
import hashlib,json,os,pathlib,subprocess,tempfile,time
ROOT=pathlib.Path('/workspace/tcl-lsp');HERE=pathlib.Path(__file__).resolve().parent;OUT=HERE/'capture-01';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();OUT.mkdir(exist_ok=False)
providers=[(v,ROOT/f'tmp/tcl{v}/unix/tclsh',ROOT/f'tmp/tcl{v}') for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']]+[('jim-pinned',ROOT/'tmp/jimtcl-oracle/jimsh',ROOT/'tmp/jimtcl-oracle')]
env=dict(os.environ)
for key in ['TCL_LIBRARY','TCLLIBPATH','TCL_BACKEND_CONSTRAINTS','TCL_TEST_VERBOSE','TCL_TEST_CONSTRAINTS']:env.pop(key,None)
records=[]
for version,binary,tree in providers:
 dest=OUT/version;dest.mkdir();before=sha(binary);argv=[str(binary),str(HERE/'input.tcl')];start=time.monotonic()
 with tempfile.TemporaryDirectory(prefix='detached-element-original-') as cwd:run=subprocess.run(argv,cwd=cwd,env=env,capture_output=True,timeout=20)
 (dest/'stdout.txt').write_bytes(run.stdout);(dest/'stderr.txt').write_bytes(run.stderr)
 sourcepaths=[tree/'generic/tclVar.c',tree/'tests/var.test'] if version!='jim-pinned' else [tree/'jim.c',tree/'jim-namespace.c']
 record={'provider':version,'argv':argv,'exit_code':run.returncode,'seconds':round(time.monotonic()-start,3),'binary_sha256_before':before,'binary_sha256_after':sha(binary),'source_sha256':{str(p.relative_to(ROOT)):sha(p) for p in sourcepaths if p.is_file()},'input_sha256':sha(HERE/'input.tcl'),'stdout_sha256':sha(dest/'stdout.txt'),'stderr_sha256':sha(dest/'stderr.txt'),'row_count':sum(l.startswith(b'ROW ') for l in run.stdout.splitlines()),'marks':[l.decode('ascii') for l in run.stdout.splitlines() if l.startswith(b'MARK ')]}
 (dest/'record.json').write_text(json.dumps(record,indent=2)+'\n');records.append(record);print(version,run.returncode,record['row_count'],len(record['marks']),flush=True)
(OUT/'provenance.json').write_text(json.dumps({'records':records,'scope':'Five fixed original-source controls. Actual C86 var-1.17 operations provide the first row with added public errorCode/marker observations. Adjacent ordinary variable-head, element-only unset, whole-root recreation with old link and newly acquired link controls preserve completion/message/errorCode and actual live scalar/member results. New bounded oracle probe, not unchanged Tcltest or private original-object/entry evidence.','axes':'Fresh five C releases and pinned Jim; Runtime/Syntax execution and builds not performed; F5 not run.','environment':'Inherited environment not logged or dumped. Only named Tcl library/test/overlay selectors removed; no host/device/security information requested.'},indent=2)+'\n')
