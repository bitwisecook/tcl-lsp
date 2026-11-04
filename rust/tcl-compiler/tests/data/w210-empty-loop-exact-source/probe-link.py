import hashlib,json,os,subprocess,time
from pathlib import Path
manifest=json.loads(Path('/tmp/lsp-2376-2426-joint-production-rlibs.json').read_text())
rows=manifest['rlibs']
assert all(hashlib.sha256(Path(row['path']).read_bytes()).hexdigest()==row['sha256'] for row in rows.values())
source=Path('/tmp/lsp-2441-source-probe.rs')
command=['rustc','--edition=2024','-C','debuginfo=0',str(source),'-o','/tmp/lsp-2441-source-probe','-L','dependency=/workspace/tcl-lsp/tmp/lsp-bindings/target/debug/deps','--extern','tcl_compiler='+rows['tcl_compiler']['path'],'--extern','tcl_registry='+rows['tcl_registry']['path']]
start=time.monotonic()
with open('/tmp/lsp-2441-source-probe-link.log','w') as log:
    run=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
receipt={'command':command,'exit':run.returncode,'seconds':time.monotonic()-start,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'product_head':manifest['head'],'product_patch_sha256':manifest['patch_sha256'],'rlibs_unchanged':all(hashlib.sha256(Path(row['path']).read_bytes()).hexdigest()==row['sha256'] for row in rows.values())}
if run.returncode==0:receipt['binary_sha256']=hashlib.sha256(Path('/tmp/lsp-2441-source-probe').read_bytes()).hexdigest()
Path('/tmp/lsp-2441-source-probe-link.receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt))
raise SystemExit(run.returncode)
