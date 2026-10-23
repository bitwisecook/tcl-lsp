import hashlib,json,os,pathlib,sys
root=pathlib.Path('/workspace/naming-toolchains');paths=[]
for version,key in [('8.4.20','84'),('8.5.19','85'),('8.6.18','86'),('9.0.4','90'),('9.1.0','91')]:
 tree=root/'tmp'/('tcl'+version)
 paths.extend([pathlib.Path(os.environ['TCL_LSP_TCLSH'+key]),tree/'unix'/'tclsh'])
 paths.extend(p for p in (tree/'library').rglob('*') if p.is_file())
 paths.extend(p for p in (tree/'unix').glob('libtcl*') if p.is_file())
paths.append(pathlib.Path(os.environ['TCL_LSP_JIMSH']))
facts={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in sorted(set(paths))}
pathlib.Path(sys.argv[1]).write_text(json.dumps(facts,sort_keys=True,indent=2)+'\n')
