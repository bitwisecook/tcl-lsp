# BIG-IP cross-context naming: environment

## Question

Which Tcl version, namespace and command capabilities are reported in the measured BIG-IP context, so that other naming observations can be scoped correctly?

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

```tcl-visible
set rows {}
foreach item {patchlevel tclversion} {
    set rc [catch {info $item} result]
    lappend rows [list info $item $rc $result]
}
set rc [catch {package provide Tcl} result]
lappend rows [list package_Tcl $rc $result]
foreach command {array binary catch encoding eval format info interp namespace package proc rename set trace unset uplevel upvar} {
    set rc [catch {info commands $command} result]
    lappend rows [list command $command $rc [llength $result] $result]
}
set rc [catch {lsort [info commands]} result]
lappend rows [list all_commands $rc $result]
set rows
```

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{info patchlevel 0 8.4.6} {info tclversion 0 8.4} {package_Tcl 0 8.4} {command array 0 1 array} {command binary 0 1 binary} {command catch 0 1 catch} {command encoding 0 0 {}} {command eval 0 1 eval} {command format 0 1 format} {command info 0 1 info} {command interp 0 1 interp} {command namespace 0 1 namespace} {command package 0 1 package} {command proc 0 1 proc} {command rename 0 1 rename} {command set 0 1 set} {command trace 0 1 trace} {command unset 0 1 unset} {command uplevel 0 1 uplevel} {command upvar 0 1 upvar} {all_commands 0 {active_members active_nodes after append array b64decode b64encode binary break call case catch class client_addr client_port clientside clock clone close concat connect continue cpu crc32 decode_uri discard domain drop eof error eval event expr fasthash fblocked fcopy fileevent findclass findstr flush for foreach format forward getfield gets global htonl htons http_close http_cookie http_header http_host http_method http_uri http_version if ifile incr info interp ip_addr ip_protocol ip_tos ip_ttl join lappend lasthop lindex link_qos linsert list listen llength llookup log lrange lreplace lsearch lset lsort matchclass mcget md5 members namespace nexthop node nodes ntohl ntohs package peer pem_dtos persist pid pool proc puts rateclass read recv redirect regexp regsub reject relate_client relate_server rename return rmd160 scan scope_exists seek send server_addr server_port serverside session set sha1 sha256 sha384 sha512 sharedvar snat snatpool split string subst substr switch table tcpdump tell time trace traffic_group translate unset update uplevel upvar urlcatblindquery urlcatquery use variable virtual vlan_id vwait whereis while}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### IAppImplementation; ICallScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: IAppImplementation; ICallScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{info patchlevel 0 8.4.6} {info tclversion 0 8.4} {package_Tcl 0 8.4} {command array 0 1 array} {command binary 0 1 binary} {command catch 0 1 catch} {command encoding 0 1 encoding} {command eval 0 1 eval} {command format 0 1 format} {command info 0 1 info} {command interp 0 1 interp} {command namespace 0 1 namespace} {command package 0 1 package} {command proc 0 1 proc} {command rename 0 1 rename} {command set 0 1 set} {command trace 0 1 trace} {command unset 0 1 unset} {command uplevel 0 1 uplevel} {command upvar 0 1 upvar} {all_commands 0 {_OnProcDefine _OnProcExecEnter _OnProcExecLeave __expand_macro_slave_interp__ _proc after append array auto_execok auto_import auto_load auto_load_index auto_qualify binary break case catch cd clock close concat continue encoding eof error eval exec exit expr fblocked fconfigure fcopy file fileevent flush for foreach format gets glob global if incr info interp join lappend lindex linsert list llength load lrange lreplace lsearch lset lsort namespace open package pid pkg_compareExtension pkg_mkIndex pkgload proc puts pwd read regexp regsub rename return scan seek set socket source split string subst switch tclLog tclPkgSetup tclPkgUnknown tell time trace unknown unset update uplevel upvar variable vwait while}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.

### TmshCliScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TmshCliScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{info patchlevel 0 8.4.6} {info tclversion 0 8.4} {package_Tcl 0 8.4} {command array 0 1 array} {command binary 0 1 binary} {command catch 0 1 catch} {command encoding 0 1 encoding} {command eval 0 1 eval} {command format 0 1 format} {command info 0 1 info} {command interp 0 1 interp} {command namespace 0 1 namespace} {command package 0 1 package} {command proc 0 1 proc} {command rename 0 1 rename} {command set 0 1 set} {command trace 0 1 trace} {command unset 0 1 unset} {command uplevel 0 1 uplevel} {command upvar 0 1 upvar} {all_commands 0 {_OnProcDefine _OnProcExecEnter _OnProcExecLeave __expand_macro_slave_interp__ _proc after append array auto_execok auto_import auto_load auto_load_index auto_qualify binary break case catch cd clock close concat continue encoding eof error eval exec exit expr fblocked fconfigure fcopy file fileevent flush for foreach format gets glob global if incr info interp join lappend lindex linsert list llength load lrange lreplace lsearch lset lsort namespace open package pid pkg_compareExtension pkg_mkIndex proc puts pwd read regexp regsub rename return run scan seek set socket source split string subst switch tclLog tclPkgSetup tclPkgUnknown tell time trace unknown unset update uplevel upvar variable vwait while}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
