import subprocess,pathlib,re,json,hashlib,time,shutil
root=pathlib.Path('/workspace/tcl-lsp')
out=pathlib.Path('/workspace/naming-validation/cli-mcp-server-889')
out.mkdir(exist_ok=True,parents=True)
frozen=out/'frozen-binaries';frozen.mkdir(exist_ok=True)
build=pathlib.Path('/tmp/naming-cli-mcp-server-889-build-retry2.log').read_text()
targets=re.findall(r'Executable (.*?) \(target/debug/deps/([^)]*)\)',build)
bins={}
for label,name in targets:
    dest=frozen/name;shutil.copy2(root/'target/debug/deps'/name,dest);bins[label+':'+name.split('-')[0]]=dest
diff=subprocess.check_output(['git','diff','HEAD','--','rust','runtime/rust'],cwd=root)
(out/'source.diff').write_bytes(diff)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
prod=root/'target/debug/f5-query'
binding={'source_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'source_diff_sha256':hashlib.sha256(diff).hexdigest(),'ordinary':True,'binaries':{k:{'name':v.name,'sha256':sha(v)} for k,v in bins.items()},'production_f5_query':{'path':str(prod),'sha256':sha(prod)},'build':'retry2 PASS87sec zero warnings; two earlier E0275 failures retained. Server binary/test use same256 compiler recursion limit as existing library. No runtime test budget changed.'}
(out/'source-binding.json').write_text(json.dumps(binding,indent=2)+'\n')
print('BINARY_FREEZE_COMPLETE',flush=True)
spec=[('tests/cli.rs:cli',''),('tests/grep.rs:grep',''),('tests/query_input.rs:query_input',''),('tests/tmsh.rs:tmsh',''),('unittests src/main.rs:tcl_mcp','tools::'),('tests/preview_tickets_e2e.rs:preview_tickets_e2e','')]
records=[]
for key,selector in spec:
    binary=bins[key];log=out/(binary.name.split('-')[0]+'.log');start=time.monotonic()
    assert sha(prod)==binding['production_f5_query']['sha256'],'production CLI changed during integration tests'
    args=[str(binary)]+([selector] if selector else [])+['--nocapture','--test-threads=1']
    with log.open('w') as f:p=subprocess.run(args,cwd=root,stdout=f,stderr=subprocess.STDOUT)
    assert sha(prod)==binding['production_f5_query']['sha256'],'production CLI changed during integration tests'
    s=log.read_text();m=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;',s)
    row={'target':key,'selector':selector,'binary_sha256':sha(binary),'source_head':binding['source_head'],'source_diff_sha256':binding['source_diff_sha256'],'exit':p.returncode,'seconds':time.monotonic()-start,'log':log.name,'log_sha256':sha(log),'status':'UNACCEPTED'}
    if m:
        row.update(passed=int(m[2]),failed=int(m[3]),ignored=int(m[4]));row['status']='NO_TESTS' if int(m[2])+int(m[3])==0 else 'PASS' if p.returncode==0 and int(m[3])==0 else 'FAIL'
    records.append(row);(out/'records.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(row),flush=True)
