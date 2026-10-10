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
		./jim-subcmd.h ./jim-win32compat.h $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/include
	$(INSTALL_DATA) jim-config.h $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/include
	$(INSTALL_DATA_DIR) $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/bin
	$(INSTALL_DATA) build-jim-ext $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/bin
	$(INSTALL_DATA_DIR) $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/lib/pkgconfig
	$(INSTALL_DATA) jimtcl.pc $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/lib/pkgconfig

install-exec: all
	$(INSTALL_DATA_DIR) $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/bin
	$(INSTALL_PROGRAM) $(JIMSH) $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/bin
	$(INSTALL_PROGRAM) ./jimdb $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/bin

uninstall:
	rm -f $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/bin/$(JIMSH)
	rm -f $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/bin/build-jim-ext
	rm -f $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/lib/$(LIBJIM)
	for i in README.extensions  ; do rm -f $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/lib/jim/$$i; done
	rm -f $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/include/jim*.h
	rm -f $(DESTDIR)/workspace/.proofs/native-providers/jimtcl/install/share/doc/jim/Tcl.html

test check: $(JIMSH)
	$(DEF_LD_PATH) $(MAKE) -C tests jimsh="/workspace/.proofs/native-providers/jimtcl/jimsh"

$(OBJS) jimsh.o initjimsh.o: Makefile $(wildcard *.h)

# Generate the unicode case mapping
utf8.o: _unicode_mapping.c

_unicode_mapping.c: ./UnicodeData.txt ./parse-unidata.tcl
	$(ECHO) "	UNIDATA	$@"
	$(Q)/usr/bin/tclsh ./parse-unidata.tcl ./UnicodeData.txt >$@ || ( rm $@; exit 1)

_load-static-exts.c: ./make-load-static-exts.tcl Makefile
	$(ECHO) "	MKLDEXT	$@"
	$(Q)/usr/bin/tclsh ./make-load-static-exts.tcl aio array clock eventloop exec file history interp json load namespace pack package posix readdir regexp signal syslog tclprefix zlib binary ensemble glob jsonencode nshelper oo stdlib tclcompat tree >$@ || ( rm $@; exit 1)

$(LIBJIM): $(OBJS)
	$(ECHO) "	AR	$@"
	$(Q)$(AR) cr $@ $(OBJS)
	$(Q)$(RANLIB) $@



docs: Tcl.html



install-docs:
	$(INSTALL_DATA_DIR) $(DESTDIR)$(docdir)
	@echo "Warning: asciidoc not available - installing Tcl_shipped.html"
	$(INSTALL_DATA) ./Tcl_shipped.html $(DESTDIR)$(docdir)/Tcl.html

Tcl.html: jim_tcl.txt ./make-index
	@echo "asciidoc is not available"; false

coverage:
	@echo "Use ./configure --coverage to enable code coverage"

clean:
	rm -f *.o *.so *.dll *.exe lib*.a $(JIMSH) $(LIBJIM) Tcl.html _*.c libjim.so
	rm -f jsmn/*.o

distclean: clean
	rm -f jimautoconf.h jim-config.h Makefile config.log jimsh0 build-jim-ext
	rm -f jimtcl.pc tests/Makefile examples.api/Makefile

ship: Tcl.html
	cp $< Tcl_shipped.html

# automake compatibility. do nothing for all these targets
EMPTY_AUTOMAKE_TARGETS := dvi pdf ps info html tags ctags mostlyclean maintainer-clean check installcheck installdirs \
 install-pdf install-ps install-info install-html -install-dvi uninstall install-data
.PHONY: $(EMPTY_AUTOMAKE_TARGETS)
$(EMPTY_AUTOMAKE_TARGETS):

# automake compatibility - install sources from the current dir to $(distdir)
distdir_full := $(shell cd $(distdir); pwd)
distdir:
	cd "."; git ls-files | cpio -pdmu $(distdir_full)

reconfig:
	CC='cc' ./configure --prefix=/workspace/.proofs/native-providers/jimtcl/install
