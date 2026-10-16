from pathlib import Path
import hashlib,json,subprocess,os,time,datetime
r=Path('/workspace/tcl-lsp');b=Path('/workspace/naming-validation/jetbrains-explorer-current');g=Path('/workspace/.toolchains/gradle/gradle.properties');g.write_text(g.read_text()+'org.gradle.java.installations.paths=/workspace/.toolchains/jdk-17.0.20.1+1\n')
files=subprocess.check_output(['git','ls-files','-z','--','editors/jetbrains','Makefile','RELEASE_NOTES.md'],cwd=r).decode().split('\0');files=[p for p in files if p and(r/p).is_file()]
def snap():return {p:hashlib.sha256((r/p).read_bytes()).hexdigest()for p in files}
a=snap();(b/'input-before.json').write_text(json.dumps(a,indent=2)+'\n');d={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip(),'command':['make','test-jetbrains'],'status':'running','scope':'Actual original wrapper/Gradle98, originalIntelliJ2025.3SDK and Kotlin JVM17 plugin test gate. No fakeSDK/stubcompile; setup failure is not test outcome.','input_count':len(a),'gradle_distribution_sha256':'bafd5ce9cfaea0fbccfdc8439a1ac42fbd4cd9c89dc9a988228d8a2639a58e6c'};(b/'binding.json').write_text(json.dumps(d,indent=2)+'\n');start=time.monotonic();env=os.environ.copy();env['GRADLE_USER_HOME']='/workspace/.toolchains/gradle'
with (b/'make-test-jetbrains.log').open('w')as log:
 try:result=subprocess.run(d['command'],cwd=r,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=1200);d['exit_code']=result.returncode
 except subprocess.TimeoutExpired:d['exit_code']=None;d['timed_out']=True
d['seconds']=time.monotonic()-start;d['input_drift']=[p for p in files if a[p]!=snap().get(p)];d['completed_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat();d['status']='PASS'if d['exit_code']==0 and not d['input_drift']else'FAILED_OR_BLOCKED_ACTUAL_GATE';(b/'binding.json').write_text(json.dumps(d,indent=2)+'\n');print(json.dumps(d,indent=2))
