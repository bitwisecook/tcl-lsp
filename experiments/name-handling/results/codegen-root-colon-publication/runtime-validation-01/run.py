import hashlib,json,os,pathlib,shutil,subprocess,time
root=pathlib.Path('/workspace/tcl-lsp')
out=root/'experiments/name-handling/results/codegen-root-colon-publication/runtime-validation-01'
metadata=json.loads(pathlib.Path('/tmp/codegen-original-argv-runtime-metadata.json').read_text())
packages=[p for p in metadata['packages'] if p['manifest_path'].startswith(str(root)+'/')]
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def sources():
 result={}
 for package in packages:
  base=pathlib.Path(package['manifest_path']).parent
  for directory,dirs,files in os.walk(base):
   dirs[:]=[d for d in dirs if d not in ['target','.git','tmp','.claude']]
   for name in files:
    p=pathlib.Path(directory)/name
    if p.is_file() and p.suffix in ['.rs','.toml','.json','.tcl','.tclspec','.txt','.tsv','.c','.h','.inc','.wit','.lock']:
     result[str(p.relative_to(root))]=digest(p)
 for p in [root/'Cargo.toml',root/'Cargo.lock',root/'rust-toolchain.toml',root/'runtime/rust/Cargo.lock',root/'.cargo/config.toml']:
  if p.is_file():result[str(p.relative_to(root))]=digest(p)
 croot=pathlib.Path(os.environ['TCL_TOMMATH_DIR'])
 for p in croot.rglob('*'):
  if p.is_file() and p.suffix in ['.c','.h','.toml','.mk']:
   result[str(p.relative_to(root))]=digest(p)
 return dict(sorted(result.items()))
before=sources();(out/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n')
(out/'packages.json').write_text(json.dumps([{'name':p['name'],'version':p['version'],'manifest_path':p['manifest_path']} for p in packages],indent=2)+'\n')
record={'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'scope':'14 resolved local packages, relevant source/data/manifests/locks and selected libtommath C/header inputs; no claim every retained fixture compiled','environment':{key:os.environ.get(key) for key in ['CARGO_TARGET_DIR','CARGO_INCREMENTAL','CARGO_BUILD_JOBS','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','RUSTFLAGS','TCL_TOMMATH_DIR','CARGO_HOME','RUSTUP_HOME']},'tools':{},'build':{},'tests':[]}
for tool,args in [('rustc',['rustc','--version','--verbose']),('cargo',['cargo','--version']),('cc',['cc','--version'])]:
 binary=pathlib.Path(shutil.which(tool)).resolve();result=subprocess.run(args,capture_output=True,text=True)
 record['tools'][tool]={'path':str(binary),'sha256':digest(binary),'stdout':result.stdout,'stderr':result.stderr,'exit_code':result.returncode}
args=['cargo','test','--offline','--manifest-path','runtime/rust/Cargo.toml','--lib','--no-run','--message-format=json']
started=time.monotonic()
with (out/'build.stdout.jsonl').open('wb') as stdout,(out/'build.stderr').open('wb') as stderr:
 result=subprocess.run(args,cwd=root,stdout=stdout,stderr=stderr)
record['build']={'argv':args,'exit_code':result.returncode,'seconds':time.monotonic()-started}
if result.returncode==0:
 artifacts=[]
 for line in (out/'build.stdout.jsonl').read_text().splitlines():
  item=json.loads(line)
  if item.get('reason')=='compiler-artifact':artifacts.append(item)
 (out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n')
 binaries=[a['executable'] for a in artifacts if a['target']['name']=='tcl_runtime' and a['profile']['test'] and a.get('executable')]
 assert len(binaries)==1,binaries
 binary=pathlib.Path(binaries[0]);binary_hash=digest(binary)
 record['executable']={'path':str(binary),'sha256':binary_hash}
 selectors=['codegen_abi::tests::original_definition_argv','codegen_abi::tests::original_root_name_object_retains_the_installed_software_entry']
 for index,selector in enumerate(selectors):
  args=[str(binary),selector,'--nocapture','--test-threads=1']
  started=time.monotonic()
  with (out/f'test-{index}.stdout').open('wb') as stdout,(out/f'test-{index}.stderr').open('wb') as stderr:
   result=subprocess.run(args,cwd=root,stdout=stdout,stderr=stderr,timeout=90)
  record['tests'].append({'selector':selector,'argv':args,'exit_code':result.returncode,'seconds':time.monotonic()-started,'stdout':f'test-{index}.stdout','stderr':f'test-{index}.stderr'})
  print(selector,'exit',result.returncode,flush=True)
 record['executable']['sha256_after']=digest(binary)
after=sources();(out/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n')
record['inputs_unchanged']=before==after;record['source_count']=len(before);record['changed_sources']=sorted(p for p in set(before)|set(after) if before.get(p)!=after.get(p))
record['finished_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())
(out/'validation.json').write_text(json.dumps(record,indent=2)+'\n')
print('build',record['build']['exit_code'],'sources',len(before),'unchanged',before==after,flush=True)
