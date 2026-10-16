const {URI}=require('/workspace/tcl-lsp/editors/vscode/node_modules/vscode-uri');
const metadata=require('/workspace/tcl-lsp/editors/vscode/node_modules/vscode-uri/package.json');
console.log(JSON.stringify({kind:'actual-provider-context',node:process.version,platform:process.platform,arch:process.arch,provider:metadata.name,version:metadata.version}));
for(const raw of ['file://srv/share/a.tcl','file://localhost/share/a.tcl','file://Srv/share//tail/','file://m%C3%BCnchen/share/%CE%BB.tcl','file://ho%40st/share/a.tcl','file://127.0.0.1/share/a.tcl','file://%3F/foo.tcl','file:///tmp/literal%252F%5C%EF%BF%BD.tcl']) {
 const uri=URI.parse(raw);
 console.log(JSON.stringify({kind:'actual-provider-result',raw,authority:uri.authority,path:uri.path,fsPath:uri.fsPath}));
}
