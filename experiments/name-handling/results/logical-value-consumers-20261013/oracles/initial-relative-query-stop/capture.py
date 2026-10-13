import hashlib,json,os,pathlib,subprocess,time
base=pathlib.Path('/workspace/tcl-lsp/tmp');out=pathlib.Path(__file__).parent
sha=lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
probe=out/'probe.tcl'; rows=[]
def seal(binary,library):
    result={str(binary):sha(binary)}
    for p in sorted(library.rglob('*')):
        if p.is_file() and not p.is_symlink(): result[str(p)]=sha(p)
    return result
for version in ('8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim'):
    binary=base/('jimtcl-oracle/jimsh' if version=='jim' else f'tcl{version}/unix/tclsh')
    library=base/('jimtcl-oracle/tclcompat' if version=='jim' else f'tcl{version}/library')
    before=seal(binary,library);env=dict(os.environ)
    if version=='jim': env.pop('TCL_LIBRARY',None)
    else: env['TCL_LIBRARY']=str(library)
    begun=time.time();p=subprocess.run([str(binary),str(probe)],env=env,capture_output=True,timeout=30)
    after=seal(binary,library)
    (out/f'{version}.stdout').write_bytes(p.stdout);(out/f'{version}.stderr').write_bytes(p.stderr)
    (out/f'{version}.inputs-before.json').write_text(json.dumps(before,indent=2)+'\n')
    (out/f'{version}.inputs-after.json').write_text(json.dumps(after,indent=2)+'\n')
    rows.append(dict(provider=version,binary=str(binary),library=str(library),exit_code=p.returncode,seconds=time.time()-begun,input_count=len(before),input_drift=before!=after,stdout_sha256=hashlib.sha256(p.stdout).hexdigest(),stderr_sha256=hashlib.sha256(p.stderr).hexdigest()))
(out/'results.json').write_text(json.dumps(dict(probe_sha256=sha(probe),runner_sha256=sha(__file__),results=rows),indent=2)+'\n')
print(json.dumps(rows,indent=2))
