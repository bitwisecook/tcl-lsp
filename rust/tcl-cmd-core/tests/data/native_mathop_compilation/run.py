import fcntl,hashlib,json,os,pathlib,subprocess,time
root=pathlib.Path(__file__).resolve().parent;root.mkdir(exist_ok=True)
operators=[('~','3'),('!','0'),('+','2 3 4'),('*','2 3 4'),('&','7 3 1'),('|','1 2 4'),('^','1 2 4'),('**','2 3 2'),('<<','3 2'),('>>','12 2'),('%','8 3'),('!=','2 3'),('ne','A B'),('in','B {A B}'),('ni','C {A B}'),('-','10 3 2'),('/','24 3 2'),('<','1 2 3'),('<=','1 1 2'),('>','3 2 1'),('>=','3 3 2'),('==','2 2 2'),('eq','A A A')]
aggregate='proc p {} {list '+ ' '.join('[::tcl::mathop::'+op+' '+args+']' for op,args in operators)+'}; p'
cases=[aggregate,
'proc p {} {list [::tcl::mathop::+] [::tcl::mathop::*] [::tcl::mathop::&] [::tcl::mathop::|] [::tcl::mathop::^] [::tcl::mathop::**]}; p',
'proc p {} {list [::tcl::mathop::+ 5] [::tcl::mathop::* 5] [::tcl::mathop::& 5] [::tcl::mathop::| 5] [::tcl::mathop::^ 5] [::tcl::mathop::** 5] [::tcl::mathop::- 5] [::tcl::mathop::/ 4]}; p',
'set seen {}; proc tick {} {lappend ::seen HIT; return VALUE}; proc p {} {list [::tcl::mathop::<] [::tcl::mathop::< [tick]] [::tcl::mathop::eq [tick]]}; list [p] $seen',
'set seen {}; proc tick {} {lappend ::seen HIT; return VALUE}; list [::tcl::mathop::< [tick]] $seen',
'proc p {} {::tcl::mathop::+ {*}{1 2 3}}; p',
'proc p {values} {::tcl::mathop::+ {*}$values}; p {1 2 3}',
'rename ::tcl::mathop::+ {odd name}; proc p {} {{odd name} 1 2 3}; p',
'rename ::tcl::mathop::! {odd name}; proc p {} {{odd name}}; catch {p} result; set result',
'namespace import ::tcl::mathop::*; proc p {} {list [+ 1 2 3] [eq A A A]}; p',
'proc p {} {list [::tcl::mathop::lt A B C] [::tcl::mathop::le A A B] [::tcl::mathop::gt C B A] [::tcl::mathop::ge C C B]}; p',
'set seen {}; proc tick {v} {lappend ::seen $v; return $v}; proc p {} {::tcl::mathop::+ [tick A] [tick B] [tick C]}; catch {p} result; list $result $seen',
'proc p {} {list [::tcl::mathop::+ 1e16 -1e16 1] [::tcl::mathop::- 1e16 1e16 -1] [::tcl::mathop::/ 100.0 5 2]}; p',
'proc p {} {::tcl::mathop::in VALUE}; catch {p} result; set result']
sources=['if {![llength [info commands ::tcl::mathop::+]]} {set marker UNAVAILABLE} else {'+case+'}' for case in cases]
(root/'cases.json').write_text(json.dumps(sources,indent=2)+'\n')
engines=[('tcl'+v,os.environ['TCL_LSP_TCLSH'+v.replace('.', '')]) for v in ['8.4','8.5','8.6','9.0','9.1']]+[('jim',os.environ['TCL_LSP_JIMSH'])]
captures=[];rows=[]
for engine,path in engines:
 for index,source in enumerate(sources):
  slot=None
  while slot is None:
   for number in range(2):
    candidate=open('/workspace/.proofs/2286-test-slot-'+str(number)+'.lock','a')
    try:fcntl.flock(candidate,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:candidate.close();continue
    slot=candidate;break
   if slot is None:time.sleep(.1)
  wrapper='set code [catch {'+source+'} result]; binary scan $result H* bytes; puts [list $code $bytes]\n'
  try:result=subprocess.run([path],input=wrapper.encode(),capture_output=True,timeout=60)
  finally:slot.close()
  code,data=result.stdout.decode().strip().split(' ',1)
  rows.append('\t'.join([engine,str(index),code,data,source.encode().hex()]))
  captures.append(dict(engine=engine,case=index,source=source,exit=result.returncode,stdout=result.stdout.decode(),stderr=result.stderr.decode(),binary=path,binary_sha256=hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()))
(root/'manifest.json').write_text(json.dumps(dict(captures=captures),indent=2)+'\n');(root/'rows.txt').write_text('\n'.join(rows)+'\n')
print(json.dumps(dict(rows=len(rows),failures=[r for r in captures if r['exit'] or r['stderr']])))
