"""Full Tcl/Jim/C/SDK provider inventory; read-only, no compilation."""
from pathlib import Path
import hashlib,json,os,subprocess,sys
BASE=Path(__file__).resolve().parent
out=Path(sys.argv[1]);setup_path=Path('/tmp/naming-oracle-setup.json');setup=json.loads(setup_path.read_text());paths={setup_path,Path('/tmp/tcl-lsp-naming-env.sh')};links={}
def add_tree(root):
 root=Path(root);assert root.is_dir(),'missing provider tree '+str(root)
 for p in root.rglob('*'):
  if p.is_symlink():links[str(p)]={'target':os.readlink(p),'realpath':str(p.resolve())}
  if p.is_file():paths.add(p)
for row in setup['tcl']:
 tree=Path(row['source_tree']);paths.update(Path(row[k])for k in ['interpreter','wrapper']);add_tree(tree/'library');paths.update(p for p in (tree/'unix').glob('libtcl*')if p.is_file())
 paths.add(Path(os.environ['TCL_LSP_TCLSH'+row['patchlevel'].split('.')[0]+row['patchlevel'].split('.')[1]]))
sdk=next(x for x in setup['tcl']if x['patchlevel']=='9.0.4');archive=Path(sdk['source_archive']);assert hashlib.sha256(archive.read_bytes()).hexdigest()==sdk['source_archive_sha256']=='8c01bac92a9a8ce271b63d3aa28f09cde2c6762d6719ca671cdc8aee25391234';paths.add(archive)
for root in ['/workspace/naming-toolchains/tmp/tcl9.0.4/libtommath','/usr/include','/usr/lib/gcc',os.environ['WASI_SDK_PATH']]:add_tree(root)
paths.add(Path(setup['jim']['interpreter']));paths.add(Path(setup['jim']['source_tree'])/'jim-config.h')
assert hashlib.sha256(Path(setup['jim']['interpreter']).read_bytes()).hexdigest()=='4c52ccf801a46df9ea5aa062c50e5d4b7e7b09c5d523d464fe8b6e682744b12e'
files={}
for p in sorted(paths):
 assert p.is_file(),'missing provider file '+str(p)
 with p.open('rb')as f:digest=hashlib.file_digest(f,'sha256').hexdigest()
 files[str(p)]={'sha256':digest,'realpath':str(p.resolve()),'symlink':os.readlink(p)if p.is_symlink()else None,'size':p.stat().st_size}
chain=out.with_name(out.name+'.chain.json');subprocess.run([sys.executable,str(BASE/'provider-chain.py'),str(chain)],check=True)
out.write_text(json.dumps({'files':files,'symlinks':links,'recursive_C_Rust_ELF_proxies':json.loads(chain.read_text())},sort_keys=True,indent=2)+'\n')
