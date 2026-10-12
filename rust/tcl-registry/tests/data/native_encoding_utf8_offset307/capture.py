from pathlib import Path
import hashlib,json,os,subprocess,time,shutil
base=Path('/workspace/.proofs');request=base/'native-encoding-utf8-offset307-request';out=base/'native-encoding-utf8-offset307-capture'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(request/'request.json')=='e09732a749b29eae6897bcd6c87433f0844818e367b5d291dd27c6303aaea08a';assert not out.exists();shutil.copytree(request,out)
(out/'capture.py').write_bytes(Path(__file__).read_bytes());req=json.loads((out/'request.json').read_text());probe=out/'probe.c';assert sha(probe)==req['probe_sha256'];summary=[]
for provider in req['providers']:
    version=provider['version'];oldpath=out/provider['original_compile_receipt_file'];assert sha(oldpath)==provider['original_compile_receipt_sha256'];old=json.loads(oldpath.read_text())
    directory=out/version;directory.mkdir(exist_ok=True);exe=directory/'probe.elf';pins=dict(provider['required_sha256']);pins[str(probe)]=req['probe_sha256']
    for p,h in pins.items():assert sha(Path(p))==h,p
    command=[str(probe) if p=='/workspace/.proofs/native-info-original-dispatch292-capture/probe.c' else p for p in old['command']];command[-1]=str(exe)
    start=time.monotonic();result=subprocess.run(command,capture_output=True,timeout=60)
    (directory/'compile.stdout').write_bytes(result.stdout);(directory/'compile.stderr').write_bytes(result.stderr)
    receipt={'command':command,'exit':result.returncode,'seconds':time.monotonic()-start,'required_sha256':pins,'original_compile_receipt_sha256':sha(oldpath),'source_sha256':sha(probe),'stdout_sha256':sha(directory/'compile.stdout'),'stderr_sha256':sha(directory/'compile.stderr'),'request_sha256':sha(out/'request.json')}
    if result.returncode==0:receipt['executable_sha256']=sha(exe)
    (directory/'compile.receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');assert result.returncode==0,(version,result.stderr)
    env=os.environ.copy()
    if version!='jim':env['TCL_LIBRARY']=str(Path('/workspace/tcl-lsp/tmp')/('tcl'+version)/'library')
    for case in req['cases']:
        destination=directory/case['id'];destination.mkdir();command=[str(exe),str(case['index'])];start=time.monotonic();result=subprocess.run(command,env=env,capture_output=True,timeout=30)
        (destination/'stdout').write_bytes(result.stdout);(destination/'stderr').write_bytes(result.stderr)
        rows=result.stdout.decode('ascii').splitlines();receipt={'provider':version,'case':case,'command':command,'exit':result.returncode,'seconds':time.monotonic()-start,'rows':rows,'executable_sha256':sha(exe),'compile_receipt_sha256':sha(directory/'compile.receipt.json'),'source_sha256':sha(probe),'required_sha256':pins,'stdout_sha256':sha(destination/'stdout'),'stderr_sha256':sha(destination/'stderr'),'environment':{key:env[key] for key in ['TCL_LIBRARY'] if key in env},'request_sha256':sha(out/'request.json'),'provider_version_row':rows[0] if rows else None,'channel':req['channel']}
        for p,h in pins.items():assert sha(Path(p))==h,p
        (destination/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
        detail={'provider':version,'case':case['id'],'exit':result.returncode,'stderr_bytes':len(result.stderr),'rows':rows};summary.append(detail);print(json.dumps(detail),flush=True)
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
