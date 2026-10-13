import hashlib,json,os,pathlib,re,shlex,subprocess,time
ROOT=pathlib.Path('/workspace/tcl-lsp')
def digest(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while b:=f.read(1024*1024):h.update(b)
 return h.hexdigest()
def key(p):
 p=p.resolve()
 try:return str(p.relative_to(ROOT))
 except ValueError:return str(p)
def local_packages(metadata_paths):
 packages={}
 for path in metadata_paths:
  for p in json.loads(pathlib.Path(path).read_text())['packages']:
   if p['manifest_path'].startswith(str(ROOT)+'/'):
    packages[p['manifest_path']]=p
 return packages
def dep_inputs(targets):
 inputs=set();depfiles={};unresolved=[]
 for target in targets:
  for p in pathlib.Path(target).rglob('*.d'):
   if not p.is_file():continue
   text=p.read_text(errors='replace').replace('\\\n','')
   line=next((line for line in text.splitlines() if ': ' in line),'')
   if not line:continue
   depfiles[key(p)]=digest(p)
   try:tokens=shlex.split(line.split(': ',1)[1])
   except ValueError:
    unresolved.append({'dep_info':key(p),'reason':'cannot parse dependency words'});continue
   for token in tokens:
    q=pathlib.Path(token)
    if not q.is_absolute():q=ROOT/q
    q=q.resolve()
    if q.is_file():inputs.add(q)
    else:unresolved.append({'dep_info':key(p),'input':token,'reason':'not a current file'})
 return inputs,depfiles,unresolved
def static_inputs(packages,targets):
 paths=set()
 for manifest in packages:
  for directory,dirs,files in os.walk(pathlib.Path(manifest).parent):
   dirs[:]=[d for d in dirs if d not in ['target','tmp','.git','node_modules']]
   for name in files:
    p=pathlib.Path(directory)/name
    if p.is_file():paths.add(p.resolve())
 for name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo/config.toml','runtime/rust/Cargo.lock']:
  p=ROOT/name
  if p.is_file():paths.add(p.resolve())
 # External authored include! data, including extensionless fixtures, discovered
 # from literal Rust macros and retained dep-info. Dynamic OUT_DIR is classified.
 for p in list(paths):
  if p.suffix!='.rs':continue
  for name in re.findall(r'include(?:_str|_bytes)?!\s*\(\s*"([^"\n]+)"',p.read_text(errors='replace')):
   q=(p.parent/name).resolve()
   if q.is_file():paths.add(q)
  for tail in re.findall(r'include(?:_str|_bytes)?!\s*\(\s*concat!\s*\(\s*env!\("CARGO_MANIFEST_DIR"\)\s*,\s*"([^"\n]+)"',p.read_text(errors='replace')):
   base=p.parent
   while not (base/'Cargo.toml').exists() and base!=ROOT:base=base.parent
   q=(base/tail.lstrip('/')).resolve()
   if q.is_file():paths.add(q)
 dep,_,_=dep_inputs(targets);paths.update(dep)
 croot=pathlib.Path(os.environ['TCL_TOMMATH_DIR'])
 for p in croot.rglob('*'):
  if p.is_file() and p.suffix in ['.c','.h','.mk']:paths.add(p.resolve())
 return {key(p):digest(p) for p in sorted(paths)}
def coverage(before,after,targets):
 inputs,depfiles,unresolved=dep_inputs(targets)
 covered={};new={};changed={}
 for p in sorted(inputs):
  k=key(p);sha=digest(p)
  if k in before and before[k]==sha and after.get(k)==sha:covered[k]=sha
  elif k in before:changed[k]={'before':before[k],'after':after.get(k),'actual':sha}
  else:new[k]={'sha256':sha,'classification':'generated after capture' if '/build/' in str(p) and '/out/' in str(p) else 'not captured before build'}
 return {'scope':'Current dep-info closure audit; generated new inputs are post-build identities, not pre-build frozen inputs. All targets include pre-existing cached dep-info; select actual built artefact dep-info when reporting check scope.','dep_info':depfiles,'covered_before_and_after':covered,'changed_or_removed':changed,'post_build_only':new,'unresolved':unresolved}
def command_record(args,out,label,timeout=None):
 start=time.monotonic();r=subprocess.run(args,cwd=ROOT,capture_output=True,timeout=timeout)
 (out/(label+'.stdout')).write_bytes(r.stdout);(out/(label+'.stderr')).write_bytes(r.stderr)
 return {'argv':args,'exit_code':r.returncode,'seconds':time.monotonic()-start,'stdout':label+'.stdout','stderr':label+'.stderr'}
