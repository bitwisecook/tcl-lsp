# naming.mathop.original-jim-source-and-helper-context

Kind: `native-observation`

## Problem statement

Which result bytes and catch codes do the unchanged 24 original mathop scripts produce under the pinned current Jim distribution, and what genuine namespace-info helper declaration supplies their absolute command guard?

## Question

Which public codes/results do the unchanged 24 original mathop scripts produce under the pinned Jim distribution, and what independently observed namespace-info helper declaration supplies their absolute availability guard?

## Conclusion

All 24 unchanged original Jim scripts produce catch code 0 and exact result UNAVAILABLE under independently pinned Jim 0.84-9-g5bac7c9 CLI SHA e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806. Ten inputs are the original identity-labelled cases and fourteen are compilation-labelled cases; their public availability guards determine these outcomes, so no operator call or specialised compilation succeeds by this observation. The separate metadata process reports genuine namespace info formals cmd {pattern *}, its complete 909-byte public body SHA 04238aa5931ab38eb21d16f1714af4eb11be0a00cfa01f5f23006c1c90abe53b and an empty info commands ::tcl::mathop::+ result. That public body byte-matches the shared raw helper source; this equality is not private procedure/object or compiled-code identity. All 25 fresh processes exit 0 with empty stderr. Each unchanged public code/result pair also matches the prior recorded Jim row, whose different CLI SHA d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0 has no retained source/library provenance in this record. The new pinned distribution proves its own public outcomes and helper declaration; it cannot donate provenance or table/extension equivalence to the old image. No original argument/result-header cache, specialised opcode, physical frame/lifetime, other-provider or Rust assertion result is established.

The two existing VM comparator definitions explicitly select NamespaceInfo only for their original Jim guarded rows. Their source, expected rows and assertions remain unchanged. The fresh current-Jim record supports exactly24 guarded public code0/UNAVAILABLE outcomes and the independent helper declaration; C result/control records retain their own original scope.

The linked guard comparator reads the unchanged current-Jim original source and measured public code/result fields, then supplies the NamespaceInfo library explicitly to its software fixture. That fixture choice is bounded to the comparison; optional software access diagnostics add no provider evidence.

## Scope

Each original source file is byte-identical to its existing public capture and row hex. The actual observation wrapper reconstructs exact counted ASCII source using binary format H* and catch {eval $source}, then reports result hex. All 24 sources contain no raw NUL or non-ASCII; source escapes and guarded unevaluated branches do not establish binary input, operator execution or native compilation. One independently launched metadata source catches public patchlevel, helper args/body and absolute command-inventory queries. Six exact source windows retain actual CLI static initialisation, namespace/nshelper extension roster, generated helper, original Tcl helper, namespace-aware Info forwarding and selected Makefile context. The current CLI/header/static archive/configuration/source/version inputs are checked before and after launch against 78 required pins. No new compiler is run. Inherited environment and system-library/compiler versions are unrecorded; the explicit overlay is empty. Old original row/manifests and their recorded CLI digest remain unchanged, but the old executable and source/library roster are unavailable and are never reconstructed from successful repeated results. Shared helper source bytes are compared independently without new mutable implementation pins or backend pass claims. C8.4–C9.1 and BIG-IP have no process in this request; their separate original mathop evidence remains independent.

These are linked software definitions, not a successful60/84-window execution report. Each independently pinned Rust receipt retains its actual failures or results, including Host ownership refusals separately from Guest/provider outcomes. The helper fixture grants no global library/bootstrap change, specialised Jim opcode, reached mathop operator, Native object/frame or provenance for the old different CLI.

This definition binding does not add a current assertion outcome or broaden the twenty-four original public completions into compiled opcode, private header, lookup roster or old CLI provenance. Independently pinned software outcomes and the existing original source/helper capture keep their separate exact image and purpose.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No process from this provider answers this current original Jim source/helper question. Its separate original mathop result/control records remain independent.

### jim

Status: `observed`. Channel: 24 exact counted ASCII original source values reconstructed by binary format H* and publicly caught eval; independent original helper args/body/version/absolute inventory queries. Version: jim 0.84-9-g5bac7c9.

All 24 exact original guarded scripts return code 0 and UNAVAILABLE. Separate helper query returns formals cmd {pattern *}, complete 909-byte body and empty absolute + inventory. All 25 host exits 0/empty stderr; public pairs match unchanged old rows but confer no old CLI roster/provenance or specialised compilation.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Exact pinned current Jim original sources; independently reported patchlevel0.84-9-g5bac7c9.`, `/workspace/.proofs/native-providers/jimtcl/jimsh.c`, function `Jim_InitStaticExtensions actual CLI entry`, lines 72–125. Retained reference `native_mathop_jim_original_source-original-inputs-workspace--.proofs--native-providers--jimtcl--jimsh.c`.

```text
    printf("Usage: %s\n", executable_name);
    printf("or   : %s [options] [filename]\n", executable_name);
    printf("\n");
    printf("Without options: Interactive mode\n");
    printf("\n");
    printf("Options:\n");
    printf("      --version  : prints the version string\n");
    printf("      --help     : prints this text\n");
    printf("      -e CMD     : executes command CMD\n");
    printf("                   NOTE: all subsequent options will be passed as arguments to the command\n");
    printf("    [filename|-] : executes the script contained in the named file, or from stdin if \"-\"\n");
    printf("                   NOTE: all subsequent options will be passed to the script\n\n");
}

int main(int argc, char *const argv[])
{
    int retcode;
    Jim_Interp *interp;
    char *const orig_argv0 = argv[0];

    /* Parse initial arguments before interpreter is started */
    if (argc > 1 && strcmp(argv[1], "--version") == 0) {
        printf("%d.%d\n", JIM_VERSION / 100, JIM_VERSION % 100);
        return 0;
    }
    else if (argc > 1 && strcmp(argv[1], "--help") == 0) {
        usage(argv[0]);
        return 0;
    }

    /* Create and initialize the interpreter */
    interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);

    /* Register static extensions */
    if (Jim_InitStaticExtensions(interp) != JIM_OK) {
        JimPrintErrorMessage(interp);
    }

    Jim_SetVariableStrWithStr(interp, "jim::argv0", orig_argv0);
    Jim_SetVariableStrWithStr(interp, JIM_INTERACTIVE, argc == 1 ? "1" : "0");
#ifdef USE_LINENOISE
    Jim_SetVariableStrWithStr(interp, "jim::lineedit", "1");
#else
    Jim_SetVariableStrWithStr(interp, "jim::lineedit", "0");
#endif
    retcode = Jim_initjimshInit(interp);

    if (argc == 1) {
        /* Executable name is the only argument - start interactive prompt */
        if (retcode == JIM_ERR) {
            JimPrintErrorMessage(interp);
        }
        if (retcode != JIM_EXIT) {

```

jim 0.84-9-g5bac7c9, revision `Exact pinned current Jim original sources; independently reported patchlevel0.84-9-g5bac7c9.`, `/workspace/.proofs/native-providers/jimtcl/_load-static-exts.c`, function `Jim_namespaceInit and Jim_nshelperInit`, lines 1–67. Retained reference `native_child_bootstrap_context303-source-_load-static-exts.c`.

```text

/* autogenerated - do not edit */
#include "jim.h"
#include "jimautoconf.h"
int Jim_InitStaticExtensions(Jim_Interp *interp)

{
	extern int Jim_stdlibInit(Jim_Interp *);
	extern int Jim_aioInit(Jim_Interp *);
	extern int Jim_arrayInit(Jim_Interp *);
	extern int Jim_clockInit(Jim_Interp *);
	extern int Jim_ensembleInit(Jim_Interp *);
	extern int Jim_eventloopInit(Jim_Interp *);
	extern int Jim_execInit(Jim_Interp *);
	extern int Jim_fileInit(Jim_Interp *);
	extern int Jim_historyInit(Jim_Interp *);
	extern int Jim_interpInit(Jim_Interp *);
	extern int Jim_jsonInit(Jim_Interp *);
	extern int Jim_jsonencodeInit(Jim_Interp *);
	extern int Jim_loadInit(Jim_Interp *);
	extern int Jim_namespaceInit(Jim_Interp *);
	extern int Jim_nshelperInit(Jim_Interp *);
	extern int Jim_ooInit(Jim_Interp *);
	extern int Jim_packInit(Jim_Interp *);
	extern int Jim_packageInit(Jim_Interp *);
	extern int Jim_posixInit(Jim_Interp *);
	extern int Jim_readdirInit(Jim_Interp *);
	extern int Jim_regexpInit(Jim_Interp *);
	extern int Jim_signalInit(Jim_Interp *);
	extern int Jim_syslogInit(Jim_Interp *);
	extern int Jim_tclcompatInit(Jim_Interp *);
	extern int Jim_tclprefixInit(Jim_Interp *);
	extern int Jim_zlibInit(Jim_Interp *);
	extern int Jim_binaryInit(Jim_Interp *);
	extern int Jim_globInit(Jim_Interp *);
	extern int Jim_treeInit(Jim_Interp *);
	Jim_stdlibInit(interp);
	Jim_aioInit(interp);
	Jim_arrayInit(interp);
	Jim_clockInit(interp);
	Jim_ensembleInit(interp);
	Jim_eventloopInit(interp);
	Jim_execInit(interp);
	Jim_fileInit(interp);
	Jim_historyInit(interp);
	Jim_interpInit(interp);
	Jim_jsonInit(interp);
	Jim_jsonencodeInit(interp);
	Jim_loadInit(interp);
	Jim_namespaceInit(interp);
	Jim_nshelperInit(interp);
	Jim_ooInit(interp);
	Jim_packInit(interp);
	Jim_packageInit(interp);
	Jim_posixInit(interp);
	Jim_readdirInit(interp);
	Jim_regexpInit(interp);
	Jim_signalInit(interp);
	Jim_syslogInit(interp);
	Jim_tclcompatInit(interp);
	Jim_tclprefixInit(interp);
	Jim_zlibInit(interp);
	Jim_binaryInit(interp);
	Jim_globInit(interp);
	Jim_treeInit(interp);
	return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `Exact pinned current Jim original sources; independently reported patchlevel0.84-9-g5bac7c9.`, `/workspace/.proofs/native-providers/jimtcl/_nshelper.c`, function `generated Jim_nshelperInit complete source`, lines 1–150. Retained reference `native_info_commands_literal_original-original-inputs-jim-_nshelper.c`.

```text
/* autogenerated - do not edit */
#include <jim.h>
int Jim_nshelperInit(Jim_Interp *interp)
{
	if (Jim_PackageProvide(interp, "nshelper", "1.0", JIM_ERRMSG)) return JIM_ERR;
	return Jim_EvalSource(interp, "nshelper.tcl", 1, "\n"
"\n"
"\n"
"\n"
"\n"
"\n"
"proc {namespace delete} {args} {\n"
"	foreach name $args {\n"
"		if {$name ni {:: \"\"}} {\n"
"			set name [uplevel 1 [list ::namespace canon $name]]\n"
"			foreach i [info commands ${name}::*] { rename $i \"\" }\n"
"			uplevel #0 [list unset {*}[info globals ${name}::*]]\n"
"		}\n"
"	}\n"
"}\n"
"\n"
"proc {namespace origin} {name} {\n"
"	set nscanon [uplevel 1 [list ::namespace canon $name]]\n"
"	if {[exists -alias $nscanon]} {\n"
"		tailcall {namespace origin} [info alias $nscanon]\n"
"	}\n"
"	if {[exists -command $nscanon]} {\n"
"		return ::$nscanon\n"
"	}\n"
"	if {[exists -command $name]} {\n"
"		return ::$name\n"
"	}\n"
"\n"
"	return -code error \"invalid command name \\\"$name\\\"\"\n"
"}\n"
"\n"
"proc {namespace which} {{type -command} name} {\n"
"	set nsname ::[uplevel 1 [list ::namespace canon $name]]\n"
"	if {$type eq \"-variable\"} {\n"
"		return $nsname\n"
"	}\n"
"	if {$type eq \"-command\"} {\n"
"		if {[exists -command $nsname]} {\n"
"			return $nsname\n"
"		} elseif {[exists -command ::$name]} {\n"
"			return ::$name\n"
"		}\n"
"		return \"\"\n"
"	}\n"
"	return -code error {wrong # args: should be \"namespace which ?-command? ?-variable? name\"}\n"
"}\n"
"\n"
"\n"
"proc {namespace code} {arg} {\n"
"	if {[string first \"::namespace inscope \" $arg] == 0} {\n"
"\n"
"		return $arg\n"
"	}\n"
"	list ::namespace inscope [uplevel 1 ::namespace current] $arg\n"
"}\n"
"\n"
"proc {namespace inscope} {name arg args} {\n"
"	tailcall namespace eval $name $arg $args\n"
"}\n"
"\n"
"proc {namespace import} {args} {\n"
"	set current [uplevel 1 ::namespace canon]\n"
"\n"
"	foreach pattern $args {\n"
"		foreach cmd [info commands [namespace canon $current $pattern]] {\n"
"			if {[namespace qualifiers $cmd] eq $current} {\n"
"				return -code error \"import pattern \\\"$pattern\\\" tries to import from namespace \\\"$current\\\" into itself\"\n"
"			}\n"
"\n"
"\n"
"			set newcmd ${current}::[namespace tail $cmd]\n"
"\n"
"			set alias $cmd\n"
"			while {[exists -alias $alias]} {\n"
"				set alias [info alias $alias]\n"
"				if {$alias eq $newcmd} {\n"
"					return -code error \"import pattern \\\"$pattern\\\" would create a loop\"\n"
"				}\n"
"			}\n"
"\n"
"			alias $newcmd $cmd\n"
"		}\n"
"	}\n"
"}\n"
"\n"
"\n"
"proc {namespace info} {cmd {pattern *}} {\n"
"	set current [uplevel 1 ::namespace canon]\n"
"\n"
"	if {[string first :: $pattern] == 0} {\n"
"		set global 1\n"
"		set prefix ::\n"
"	} else {\n"
"		set global 0\n"
"		set clen [string length $current]\n"
"		incr clen 2\n"
"	}\n"
"	set fqp [namespace canon $current $pattern]\n"
"	switch -glob -- $cmd {\n"
"		co* - p* {\n"
"			if {$global} {\n"
"				set result [info $cmd $fqp]\n"
"			} else {\n"
"\n"
"				set r {}\n"
"				foreach c [info $cmd $fqp] {\n"
"					dict set r [string range $c $clen end] 1\n"
"				}\n"
"				if {[string match co* $cmd]} {\n"
"\n"
"					foreach c [info -nons commands $pattern] {\n"
"						dict set r $c 1\n"
"					}\n"
"				}\n"
"				set result [dict keys $r]\n"
"			}\n"
"		}\n"
"		ch* {\n"
"			set result [info channels $pattern]\n"
"		}\n"
"		v* {\n"
"\n"
"			set result [uplevel #0 info -nons vars $fqp]\n"
"		}\n"
"		g* {\n"
"			set result [info globals $fqp]\n"
"		}\n"
"		l* {\n"
"			set result [uplevel 1 info -nons locals $pattern]\n"
"		}\n"
"	}\n"
"	if {$global} {\n"
"		set result [lmap p $result { string cat $prefix $p }]\n"
"	}\n"
"	return $result\n"
"}\n"
"\n"
"proc {namespace upvar} {ns args} {\n"
"	set nscanon ::[uplevel 1 [list ::namespace canon $ns]]\n"
"	set script [list upvar 0]\n"
"	foreach {other local} $args {\n"
"		lappend script ${nscanon}::$other $local\n"
"	}\n"
"	tailcall {*}$script\n"
"}\n"

```

jim 0.84-9-g5bac7c9, revision `Exact pinned current Jim original sources; independently reported patchlevel0.84-9-g5bac7c9.`, `/workspace/.proofs/native-providers/jimtcl/nshelper.tcl`, function `namespace info scripted helper`, lines 87–142. Retained reference `native_jim_alias_introspection274-nshelper.tcl`.

```text
proc {namespace info} {cmd {pattern *}} {
	set current [uplevel 1 ::namespace canon]
	# Now we may need to strip $pattern
	if {[string first :: $pattern] == 0} {
		set global 1
		set prefix ::
	} else {
		set global 0
		set clen [string length $current]
		incr clen 2
	}
	set fqp [namespace canon $current $pattern]
	switch -glob -- $cmd {
		co* - p* {
			if {$global} {
				set result [info $cmd $fqp]
			} else {
				# Add commands in the current namespace
				set r {}
				foreach c [info $cmd $fqp] {
					dict set r [string range $c $clen end] 1
				}
				if {[string match co* $cmd]} {
					# Now in the global namespace
					foreach c [info -nons commands $pattern] {
						dict set r $c 1
					}
				}
				set result [dict keys $r]
			}
		}
		ch* {
			set result [info channels $pattern]
		}
		v* {
			#puts "uplevel #0 info gvars $fqp"
			set result [uplevel #0 info -nons vars $fqp]
		}
		g* {
			set result [info globals $fqp]
		}
		l* {
			set result [uplevel 1 info -nons locals $pattern]
		}
	}
	if {$global} {
		set result [lmap p $result { string cat $prefix $p }]
	}
	return $result
}

proc {namespace upvar} {ns args} {
	set nscanon ::[uplevel 1 [list ::namespace canon $ns]]
	set script [list upvar 0]
	foreach {other local} $args {
		lappend script ${nscanon}::$other $local

```

jim 0.84-9-g5bac7c9, revision `Exact pinned current Jim original sources; independently reported patchlevel0.84-9-g5bac7c9.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `INFO_COMMANDS genuine namespace-aware helper forwarding`, lines 16070–16150. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
    }
    /* (ct - cmds) is the index into the table */
    int option = ct - cmds;

    switch (option) {
        case INFO_EXISTS:
            Jim_SetResultBool(interp, Jim_GetVariable(interp, argv[2], 0) != NULL);
            return JIM_OK;

        case INFO_ALIAS:{
            Jim_Cmd *cmdPtr;

            if ((cmdPtr = Jim_GetCommand(interp, argv[2], JIM_ERRMSG)) == NULL) {
                return JIM_ERR;
            }
            if ((cmdPtr->flags & JIM_CMD_ISALIAS) == 0) {
                Jim_SetResultFormatted(interp, "command \"%#s\" is not an alias", argv[2]);
                return JIM_ERR;
            }
            Jim_SetResult(interp, (Jim_Obj *)cmdPtr->u.native.privData);
            return JIM_OK;
        }

        case INFO_TAINTED:
            if (argc != 3) {
                Jim_WrongNumArgs(interp, 2, argv, "value");
                return JIM_ERR;
            }
            Jim_SetResultBool(interp, argv[2]->taint != 0);
            return JIM_OK;
        case INFO_CHANNELS:
            mode <<= 1;             /* JIM_CMDLIST_CHANNELS */
#ifndef jim_ext_aio
            Jim_SetResultString(interp, "aio not enabled", -1);
            return JIM_ERR;
#endif
            /* fall through */
        case INFO_ALIASES:
            mode <<= 1;             /* JIM_CMDLIST_ALIASES */
            /* fall through */
        case INFO_PROCS:
            mode <<= 1;             /* JIM_CMDLIST_PROCS */
            /* fall through */
        case INFO_COMMANDS:{
            int n = 0;
            /* mode = 1 => JIM_CMDLIST_COMMANDS */
            if (argc > 2 && Jim_CompareStringImmediate(interp, argv[2], "-all")) {
                mode |= JIM_CMDLIST_ALL;
                n++;
            }
            if (argc < 2 + n || argc > 3 + n) {
                Jim_SetResultFormatted(interp, "wrong # args: should be \"info %#s %s\"", argv[1], cmds[option].args);
                return JIM_ERR;
            }
#ifdef jim_ext_namespace
            if (!nons) {
                /* Called as 'info -nons commands|procs' so respect the current namespace */
                if (Jim_Length(interp->framePtr->nsObj) || (argc == 3 && JimIsGlobalNamespace(argv[2]))) {
                    return Jim_EvalPrefix(interp, "namespace info", argc - 1, argv + 1);
                }
            }
#endif
            Jim_SetResult(interp, JimCommandsList(interp, (argc == 3 + n) ? argv[2 + n] : NULL, mode));
            return JIM_OK;
        }

        case INFO_VARS:
            mode <<= 1;             /* JIM_VARLIST_VARS */
            /* fall through */
        case INFO_LOCALS:
            mode <<= 1;             /* JIM_VARLIST_LOCALS */
            /* fall through */
        case INFO_GLOBALS:
            /* mode = 1 => JIM_VARLIST_GLOBALS */
#ifdef jim_ext_namespace
            if (!nons) {
                if (Jim_Length(interp->framePtr->nsObj) || (argc == 3 && JimIsGlobalNamespace(argv[2]))) {
                    return Jim_EvalPrefix(interp, "namespace info", argc - 1, argv + 1);
                }
            }
#endif

```

jim 0.84-9-g5bac7c9, revision `Exact pinned current Jim original sources; independently reported patchlevel0.84-9-g5bac7c9.`, `/workspace/.proofs/native-providers/jimtcl/Makefile`, function `selected static namespace/nshelper extension roster`, lines 1–80. Retained reference `native_info_commands_literal_original-original-inputs-jim-Makefile`.

```text
# make V=1 for verbose make output
Q := @
ECHO := @echo
ifeq ($(V),1)
ifeq ("$(origin V)", "command line")
Q :=
ECHO := @:
endif
endif

# Tools
CC =  cc
CXX =  c++
RANLIB = ranlib
AR = ar
STRIP = strip

# Configuration

SH_CFLAGS ?= -fPIC
SH_LDFLAGS ?= -shared
SH_LDFLAGS += -Wl,-soname,libjim.so.0.84
SHOBJ_CFLAGS ?= -fPIC
SHOBJ_LDFLAGS ?= -shared

AS_CFLAGS += -I. -fno-unwind-tables -fno-asynchronous-unwind-tables -DUSE_UTF8 -DHAVE_OPENPTY
AS_CXXFLAGS += -I. -fno-unwind-tables -fno-asynchronous-unwind-tables -DUSE_UTF8 -DHAVE_OPENPTY
AS_CPPFLAGS += -O2 -Wall -D_GNU_SOURCE
CFLAGS ?= 
CXXFLAGS ?= 
LDFLAGS = 
LDLIBS += -lm -lssl -lcrypto  -lz 
LIBS += 
exec_prefix ?= /workspace/.proofs/native-providers/jimtcl/install
prefix ?= /workspace/.proofs/native-providers/jimtcl/install
docdir = ${prefix}/docs/jim
srcdir := .

LIBJIM := libjim.a

JIMSH_CC := $(CC) $(AS_FLAGS) $(CFLAGS)

OBJS := _load-static-exts.o jim-subcmd.o jim-interactive.o jim-format.o jim.o utf8.o jimregexp.o jimiocompat.o \
    linenoise.o jim-tty.o openpty.o jsmn/jsmn.o jim-aio.o jim-array.o jim-clock.o jim-eventloop.o jim-exec.o jim-file.o jim-history.o jim-interp.o jim-json.o jim-load.o jim-namespace.o jim-pack.o jim-package.o jim-posix.o jim-readdir.o jim-regexp.o jim-signal.o jim-syslog.o jim-tclprefix.o jim-zlib.o binary.o ensemble.o glob.o jsonencode.o nshelper.o oo.o stdlib.o tclcompat.o tree.o

JIMSH := jimsh

INSTALL_DATA_DIR ?= mkdir -p
INSTALL_DATA ?= cp
INSTALL_PROGRAM ?= cp

all: $(JIMSH) 

# Create C extensions from pure Tcl extensions
.SUFFIXES: .tcl
.tcl.o:
	$(ECHO) " 	TCLEXT	_$*.c"
	$(Q)/usr/bin/tclsh ./make-c-ext.tcl $< >_$*.c
	$(ECHO) " 	CC	$@"
	$(Q)$(CC) $(AS_CFLAGS) $(AS_CPPFLAGS) $(CFLAGS) $(CPPFLAGS) -c -o $@ _$*.c

.c.o:
	$(ECHO) "	CC	$@"
	$(Q)$(CC) $(AS_CFLAGS) $(AS_CPPFLAGS) $(CFLAGS) $(CPPFLAGS) $(TARGET_ARCH) -c $< -o $@

.cpp.o:
	$(ECHO) "	CXX	$@"
	$(Q)$(CXX) $(AS_CXXFLAGS) $(AS_CPPFLAGS) $(CXXFLAGS) $(CPPFLAGS) $(TARGET_ARCH) -c $< -o $@

$(JIMSH): $(LIBJIM) jimsh.o initjimsh.o
	$(ECHO) "	LINK	$@"
	$(Q)$(JIMSH_CC) -Wl,-rpath -Wl,/workspace/.proofs/native-providers/jimtcl/install/lib -rdynamic $(LDFLAGS) -o $@ jimsh.o initjimsh.o $(LIBJIM) $(LDLIBS) $(LIBS)

install: all  install-exec install-docs
	$(INSTALL_DATA_DIR) $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/lib/jim
	$(INSTALL_DATA) $(LIBJIM) $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/lib
	$(INSTALL_DATA) ./README.extensions  $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/lib/jim
	for i in tcltest.tcl ; do $(INSTALL_DATA) ./$$i $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/lib/jim; done
	$(INSTALL_DATA_DIR) $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/include
	$(INSTALL_DATA) ./jim.h ./jim-eventloop.h ./jim-signal.h \

```

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-vm/src/cmd_mathop.rs](../../../../rust/tcl-vm/src/cmd_mathop.rs), `selected_mathop_identity_matches_all_60_native_name_and_argv_controls`: The existing complete comparator explicitly selects NamespaceInfo only for original Jim guarded rows; C observations remain under their own independent proof.
- [rust/tcl-vm/src/cmd_mathop.rs](../../../../rust/tcl-vm/src/cmd_mathop.rs), `original_mathop_compilation_matches_all_84_native_controls`: Preserve every original source/result assertion while supplying the independently observed Jim namespace-info declaration to the Jim fixture only; no global bootstrap or mathop command installation follows.
- [rust/tcl-vm/src/cmd_mathop.rs](../../../../rust/tcl-vm/src/cmd_mathop.rs), `cmd_mathop::tests::selected_mathop_identity_matches_all_60_native_name_and_argv_controls` (linked): Only original Jim guarded public script completion parity has fresh435 context evidence; existing C original controls and all source/result assertions remain exact. This links the software assertion definition only. Independent actual execution receipts retain their own failures and scope; no whole-provider or specialised compilation pass is inferred.
- [rust/tcl-vm/src/cmd_mathop.rs](../../../../rust/tcl-vm/src/cmd_mathop.rs), `cmd_mathop::tests::original_mathop_compilation_matches_all_84_native_controls` (linked): Only original Jim guarded public script completion parity has fresh435 context evidence; existing C original controls and all source/result assertions remain exact. This links the software assertion definition only. Independent actual execution receipts retain their own failures and scope; no whole-provider or specialised compilation pass is inferred.

These source bindings establish no executed assertion result; exact software outcomes belong to the independently pinned Rust validation receipts.

- [rust/tcl-vm/src/native_fixture.rs](../../../../rust/tcl-vm/src/native_fixture.rs), `interpreter_with_scripted_libraries`: Select the actual software engine and explicitly requested original scripted distribution library for a bounded comparator; no global library roster or old differing CLI provenance follows.
- [rust/tcl-vm/src/cmd_mathop.rs](../../../../rust/tcl-vm/src/cmd_mathop.rs), `cmd_mathop::tests::original_jim_namespace_info_guard_retains_its_original_headers` (linked): The software comparator uses the unchanged independently recorded current-Jim identity0 original source and public code/result fields with an explicit NamespaceInfo library fixture. Optional access diagnostics supply no provider evidence or opcode/header conclusion.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_mathop_jim_original_source/inputs](../../../../rust/tcl-registry/tests/data/native_mathop_jim_original_source/inputs)
- [rust/tcl-registry/tests](../../../../rust/tcl-registry/tests)
- [rust/tcl-registry/tests/data/native_mathop_jim_original_source/providers/jim/cases](../../../../rust/tcl-registry/tests/data/native_mathop_jim_original_source/providers/jim/cases)
- [rust/tcl-registry/tests/data/native_mathop_jim_original_source/providers/jim](../../../../rust/tcl-registry/tests/data/native_mathop_jim_original_source/providers/jim)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source)
- [rust/tcl-registry/tests/data/native_mathop_jim_original_source](../../../../rust/tcl-registry/tests/data/native_mathop_jim_original_source)
- [rust/tcl-cmd-core/tests/data](../../../../rust/tcl-cmd-core/tests/data)
- [rust/tcl-registry/tests/data/native_jim_class_source269](../../../../rust/tcl-registry/tests/data/native_jim_class_source269)
- [rust/tcl-registry/tests/data/native_c_alias_holder197](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197)
