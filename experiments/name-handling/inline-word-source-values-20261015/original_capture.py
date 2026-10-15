from pathlib import Path
import json,hashlib,os,subprocess,datetime
p=Path('/tmp/name-inline-word-splice-oracles-20261015');prior=json.loads(Path('/tmp/name-scalar-expression-fixture-oracles-20261015/manifest.json').read_text());h=lambda b:hashlib.sha256(b).hexdigest()
cases=[('ORIGINAL_QUOTED_LITERAL','set x {a"b}; set observed "v:$x"'),('RAW_UNESCAPED_SPLICE','set observed "v:a"b"'),('ESCAPED_QUOTED_SPLICE','set observed "v:a\\"b"'),('ORIGINAL_LITERAL_SIGIL','set x {a $b}; set observed "v:$x"'),('RAW_ACTIVATED_SPLICE','set b ALTERED; set observed "v:a $b"'),('NESTED_NAME_QUOTE','set {a{b}"c} VALUE; set observed "${a{b}"c} suffix"')]
probe='puts [list PATCHLEVEL [info patchlevel]]\n'
for label,source in cases:
    probe+='set script [binary format H* '+source.encode().hex()+']\nset complete [info complete $script]\nset code [catch {uplevel #0 $script} result]\nputs [list '+label+' COMPLETE $complete CODE $code RESULT $result]\n'
(p/'probe.tcl').write_text(probe);rows=[]
for old in prior['rows']:
    expected=old['original_files_before'];before={k:h(Path(k).read_bytes()) for k in expected};assert before==expected
    env=dict(os.environ);env.pop('TCL_LIBRARY',None)
    if old.get('selected_original_library'):env['TCL_LIBRARY']=old['selected_original_library']
    argv=[old['argv'][0],str(p/'probe.tcl')];r=subprocess.run(argv,env=env,capture_output=True,timeout=20)
    after={k:h(Path(k).read_bytes()) for k in expected};assert before==after and r.returncode==0
    row={'provider_label':old['provider_label'],'argv':argv,'selected_original_library':old.get('selected_original_library'),'original_files_before':before,'original_files_after':after,'exit_code':r.returncode}
    for ch,b in [('stdout',r.stdout),('stderr',r.stderr)]:fn=old[ch+'_file'];(p/fn).write_bytes(b);row[ch+'_file']=fn;row[ch+'_sha256']=h(b)
    rows.append(row)
m={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'probe_sha256':h(probe.encode()),'scope':'Original fixed inline-source literal/template and configured quoted-name word observations BEFORE new source constructor/geometry controls. Full original code/completeness/result percase retained. No Native read/cell/motion/refactor/edit authority, whole transform/evaluation equivalence or F5 claim.','rows':rows};(p/'manifest.json').write_text(json.dumps(m,indent=2)+'\n');print(h((p/'manifest.json').read_bytes()))
