from pathlib import Path
import json,hashlib,os,subprocess,datetime
p=Path('/tmp/name-generated-request-word-oracles-20261015');prior=json.loads(Path('/tmp/name-scalar-expression-fixture-oracles-20261015/manifest.json').read_text());h=lambda b:hashlib.sha256(b).hexdigest()
words=[('OLD_SUBSTITUTING','"host[set ::hint 1]"'),('LITERAL_SUBSTITUTION','"host\\[set ::hint 1\\]"'),('QUOTED_DATA','"\\\"host\\\""'),('QUOTED_UNBALANCED_BRACE','"}"'),('ESCAPED_BRACE','\\}'),('LITERAL_SIGIL','"\\$literal"')]
probe='puts [list PATCHLEVEL [info patchlevel]]\n'
for label,word in words:
    probe+='set word [binary format H* '+word.encode().hex()+']\n'
    probe+='set ::hint 0\nset script "list $word"\nset code [catch {eval $script} value]\nset bodyword [format "{%s}" $script]\nset bodycode [catch {eval "list $bodyword"} body]\nset entered -1\nset result {}\nif {!$bodycode} {set entered [catch {eval [lindex $body 0]} result]}\n'
    probe+='puts [list '+label+' WORD_CODE $code VALUE $value BODY_CODE $bodycode ENTERED_CODE $entered RESULT $result ACTIVATED $::hint]\n'
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
m={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'probe_sha256':h(probe.encode()),'scope':'Original fixed Tcl source proposal/hint-word observations before new constructor controls. Standalone quoted substitution vs literal escapes and independently braced script-container behavior. No F5/MCP/Rust/frame/binding/equivalence proof.','rows':rows};(p/'manifest.json').write_text(json.dumps(m,indent=2)+'\n');print(h((p/'manifest.json').read_bytes()))
