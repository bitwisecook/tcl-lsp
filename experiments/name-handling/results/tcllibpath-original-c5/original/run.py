import hashlib,json,os,pathlib,subprocess,time
repo=pathlib.Path('/workspace/tcl-lsp');root=pathlib.Path(__file__).resolve().parent
script=root/'probe.tcl'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
manifest=repo/'rust/tcl-dialect/data/reference-toolchains.tsv'
rows=[line.split('\t') for line in manifest.read_text().splitlines() if line and not line.startswith('#')]
results=[]
questions='Observe original C Tcl list environment ingress and list-prepend semantics. Does one genuinely constructed list element preserve an actual special-character Unicode filesystem path and every original suffix value, while raw path+space changes it? Record release-dependent suffix meanings rather than assuming one escape grammar.'
fixture=root/'fixture'/'café space {braces} [literal];$value\\tail'/'lib';fixture.mkdir(parents=True,exist_ok=True);(fixture/'sentinel').write_bytes(b'actual directory')
cases=[('absent',False,''),('empty',True,''),('whitespace',True,'  {old path} \t\n'),('release-escapes',True,r'old\x4142 {literal\n} trailing\ '),('invalid-brace',True,'{unclosed'),('invalid-junk',True,'{closed}junk')]
for release,patchlevel,tag in rows:
 tree=repo/'tmp'/('tcl'+patchlevel);binary=tree/'unix/tclsh';library=tree/'library'
 provider={str(p.relative_to(tree)):digest(p) for p in tree.rglob('*') if p.is_file() and (p.is_relative_to(library) or p.relative_to(tree).as_posix() in ['unix/tclsh','unix/config.log','unix/config.status','unix/Makefile','generic/tcl.h','generic/tclEnv.c','generic/tclUtil.c','generic/tclParse.c','generic/tclUtf.c'] or (p.parent==tree/'unix' and p.suffix in ['.so','.dylib']))}
 (root/('provider-files-'+release+'.json')).write_text(json.dumps(provider,indent=2,sort_keys=True)+'\n')
 env=os.environ.copy()
 for key in ['TCLLIBPATH','TCL_LIBRARY','LD_LIBRARY_PATH']:env.pop(key,None)
 env.update(TCL_LIBRARY=str(library),LD_LIBRARY_PATH=str(tree/'unix'),EXPECTED_PATCHLEVEL=patchlevel,EXPECTED_LIBRARY=str(library),INPUT_PATH=str(fixture))
 ldd=subprocess.run(['ldd',str(binary)],env=env,capture_output=True,timeout=20)
 (root/('ldd-'+release+'.stdout')).write_bytes(ldd.stdout);(root/('ldd-'+release+'.stderr')).write_bytes(ldd.stderr)
 for name,present,suffix in cases:
  case=root/(release+'-'+name);case.mkdir(exist_ok=True)
  actual=env|{'INPUT_SUFFIX':suffix,'HAS_INPUT_SUFFIX':str(int(present))}
  start=time.monotonic()
  try:
   out=subprocess.run([str(binary),str(script)],env=actual,capture_output=True,timeout=30)
   (case/'stdout').write_bytes(out.stdout);(case/'stderr').write_bytes(out.stderr)
   result={'release':release,'patchlevel':patchlevel,'tag':tag,'case':name,'binary':str(binary),'binary_sha256':digest(binary),'library':str(library),'script_sha256':digest(script),'input_path':str(fixture),'suffix_present':present,'suffix':suffix,'invocation':[str(binary),str(script)],'environment':{key:actual.get(key) for key in ['TCL_LIBRARY','LD_LIBRARY_PATH','LANG','LC_ALL','LC_CTYPE','INPUT_PATH','INPUT_SUFFIX','HAS_INPUT_SUFFIX','EXPECTED_PATCHLEVEL','EXPECTED_LIBRARY']},'exit_code':out.returncode,'completed':True,'elapsed_seconds':time.monotonic()-start,'stdout_sha256':digest(case/'stdout'),'stderr_sha256':digest(case/'stderr'),'setup_failure':not out.stdout.startswith(b'PROVIDER\t'),'semantic_pass':name.startswith('invalid-') or out.stdout.endswith(b'PASS\n')}
  except Exception as error:result={'release':release,'case':name,'completed':False,'setup_failure':str(error)}
  (case/'result.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n');results.append(result)
receipt={'question':questions,'provider_owner':str(manifest),'provider_owner_sha256':digest(manifest),'toolchain_adapter':str(repo/'scripts/dev/tcl-reference-toolchains.sh'),'toolchain_adapter_sha256':digest(repo/'scripts/dev/tcl-reference-toolchains.sh'),'script':str(script),'script_sha256':digest(script),'harness':'Plain upstream interpreter file mode, no tcltest used; exact original init/library path per release and actual info-library check. No Rust implementation or generated candidate output was run.','limitations':'This observes the selected original C5 binaries in the captured environment/encoding. It does not authenticate arbitrary external receiver encodings, scripts/wrappers, installed command identity or Native currentness. Invalid-list rows are observed errors, not normal-completion claims.','results':results}
(root/'result.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'rows':len(results),'completed':sum(r.get('completed',False) for r in results),'setup_failures':[r for r in results if r.get('setup_failure')],'nonzero':[r for r in results if r.get('exit_code')!=0],'semantic_missing':[r for r in results if not r.get('semantic_pass',False)]},ensure_ascii=False))
