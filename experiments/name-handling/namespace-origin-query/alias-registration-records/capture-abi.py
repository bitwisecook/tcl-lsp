from pathlib import Path
import subprocess,json,hashlib,re,shlex
out=Path('/tmp/runtime-alias-record-oracles-20261013');root=Path('/workspace/tcl-lsp/tmp')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 tree=root/('tcl'+version);d=out/version/'abi';d.mkdir(parents=True,exist_ok=True);config=tree/'unix/tclConfig.sh';text=config.read_text()
 def field(n):return shlex.split(re.search('^'+n+"='([^']*)'",text,re.M).group(1))
 stub=next((tree/'unix').glob('libtclstub*.a'));argv=field('TCL_CC')+field('TCL_EXTRA_CFLAGS')+['-shared','-fPIC','-DUSE_TCL_STUBS','-I'+str(tree/'generic'),str(out/'abi.c'),str(stub),'-o',str(d/'aliasabi.so')]
 before={str(p):sha(p) for p in [config,stub,tree/'generic/tcl.h',tree/'generic/tclDecls.h',out/'abi.c',tree/'unix/tclsh']}
 compile=subprocess.run(argv,capture_output=True);(d/'compile.stdout').write_bytes(compile.stdout);(d/'compile.stderr').write_bytes(compile.stderr)
 if compile.returncode:raise RuntimeError(version+compile.stderr.decode())
 inp=d/'input.tcl';inp.write_text('puts [list IDENTITY [info patchlevel]]\nputs [list ABI [load {'+str(d/'aliasabi.so')+'} Aliasabi]]\n')
 run=subprocess.run([str(tree/'unix/tclsh'),str(inp)],capture_output=True);(d/'stdout.bin').write_bytes(run.stdout);(d/'stderr.bin').write_bytes(run.stderr)
 after={p:sha(Path(p)) for p in before}
 record={'provider':version,'compiler_argv':argv,'compiler_exit_code':compile.returncode,'oracle_argv':[str(tree/'unix/tclsh'),str(inp)],'oracle_exit_code':run.returncode,'before':before,'after':after,'drift':{p:[before[p],after[p]] for p in before if before[p]!=after[p]},'source_sha256':sha(out/'abi.c'),'input_sha256':sha(inp),'module_sha256':sha(d/'aliasabi.so'),'stdout_sha256':sha(d/'stdout.bin'),'stderr_sha256':sha(d/'stderr.bin'),'scope':'Actual matching-header C extension compiled with this oracle build configuration and loaded into the original C interpreter; CHAR_MIN, sizeof(unsigned int), sizeof(size_t)'}
 (d/'record.json').write_text(json.dumps(record,indent=2)+'\n');print(version,run.returncode,run.stdout.decode().strip(),len(record['drift']))
