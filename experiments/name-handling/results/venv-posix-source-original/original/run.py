from pathlib import Path
import os,sys,json,hashlib,subprocess,time,platform
root=Path(__file__).resolve().parent;shell=Path('/bin/sh');resolved=shell.resolve()
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def identity():
 st=resolved.stat();return {'invocation_path':str(shell),'resolved_path':str(resolved),'sha256':sha(resolved),'device':st.st_dev,'inode':st.st_ino,'size':st.st_size,'mtime_ns':st.st_mtime_ns}
before=identity();rows=[]
cases=['','plain','a b','quote\'double"','$literal`quote`;$(literal)','back\\slash','line\nnext','café']
for index,value in enumerate(cases):
 # Candidate recipe only. Expected argv bytes come directly from original input.
 literal="'"+value.replace("'","'\\''")+"'"
 source="printf '%s\\000' "+literal+"\n"
 path=root/('case-'+str(index)+'.sh');path.write_bytes(source.encode())
 argv=[str(shell),str(path)];env={b'PATH':b'/usr/bin:/bin',b'LANG':os.fsencode(os.environ.get('LANG','C.UTF-8'))}
 start=time.monotonic();out=subprocess.run(argv,env=env,capture_output=True,timeout=20)
 (root/(str(index)+'.stdout')).write_bytes(out.stdout);(root/(str(index)+'.stderr')).write_bytes(out.stderr)
 rows.append({'case':index,'input':value,'input_utf8_hex':value.encode().hex(),'source_file':path.name,'source_sha256':sha(path),'argv':argv,'environment_hex':{k.decode():v.hex() for k,v in env.items()},'exit_code':out.returncode,'completed':True,'elapsed_seconds':time.monotonic()-start,'stdout_hex':out.stdout.hex(),'stderr_hex':out.stderr.hex(),'exact_argv_check':out.stdout==value.encode()+b'\0' and out.returncode==0 and not out.stderr})
source=b'''export TCLLIBPATH='{new path [literal];$value}'"${TCLLIBPATH:+ $TCLLIBPATH}"
printf '%s\\000' "$TCLLIBPATH"
''';path=root/'suffix.sh';path.write_bytes(source)
for index,suffix in enumerate([None,b'',b'  {old path} \t\n',b'original-\xff']):
 env={b'PATH':b'/usr/bin:/bin'}
 if suffix is not None:env[b'TCLLIBPATH']=suffix
 argv=[str(shell),str(path)];start=time.monotonic();out=subprocess.run(argv,env=env,capture_output=True,timeout=20)
 (root/('suffix-'+str(index)+'.stdout')).write_bytes(out.stdout);(root/('suffix-'+str(index)+'.stderr')).write_bytes(out.stderr)
 expected=b'{new path [literal];$value}'+(b' '+suffix if suffix else b'')+b'\0'
 rows.append({'case':'suffix-'+str(index),'original_suffix_hex':None if suffix is None else suffix.hex(),'source_file':path.name,'source_sha256':sha(path),'argv':argv,'environment_hex':{k.decode():v.hex() for k,v in env.items()},'exit_code':out.returncode,'completed':True,'elapsed_seconds':time.monotonic()-start,'stdout_hex':out.stdout.hex(),'stderr_hex':out.stderr.hex(),'exact_argv_check':out.stdout==expected and out.returncode==0 and not out.stderr})
after=identity();receipt={'question':'Does the actual selected POSIX shell preserve original Unicode literal argv and raw inherited environment suffix bytes under the source-word recipe, without evaluating metacharacters?','platform':platform.platform(),'python':sys.version,'driver_sha256':sha(Path(__file__)),'shell_before':before,'shell_after':after,'shell_unchanged':before==after,'rows':rows,'limitations':'This observes the exact captured /bin/sh binary and source recipes only. No Rust implementation, Tcl interpreter, Fish or Docker grammar was executed. The suffix observation concerns shell OS-byte transport, not Tcl decoding or filesystem selection.'}
(root/'result.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'completed':len(rows),'exact_checks':sum(r['exact_argv_check'] for r in rows),'shell_unchanged':before==after}))
