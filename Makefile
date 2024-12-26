# See https://www.gnu.org/software/make/manual/make.html

prefix ?= /usr/local
exec_prefix ?= $(prefix)
datarootdir ?= $(prefix)/share
bindir ?= $(exec_prefix)/bin
mandir ?= $(datarootdir)/man
man1dir ?= $(mandir)/man1
man5dir ?= $(mandir)/man5
INSTALL ?= install
INSTALL_PROGRAM ?= $(INSTALL)
INSTALL_DATA ?= $(INSTALL)

.PHONY: all clean clean-man default \
		fsidx doc fsidx.1 fsidx.toml.5 \
		view.doc view.fsidx view.fsidx.toml \
		target/release/fsidx \
		install man test uninstall

all: fsidx test doc man

fsidx: target/release/fsidx

target/release/fsidx:
	cargo build --features="cli" --release

test:
	cargo test --features="cli"

doc:
	cargo doc --features="cli"

install: fsidx test man
	$(INSTALL_PROGRAM) target/release/fsidx $(DESTDIR)$(bindir)/fsidx
	$(INSTALL_DATA) -d  $(DESTDIR)$(man1dir)
	$(INSTALL_DATA) -d  $(DESTDIR)$(man5dir)
	$(INSTALL_DATA) target/man/man1/fsidx.1 $(DESTDIR)$(man1dir)/fsidx.1
	$(INSTALL_DATA) target/man/man5/fsidx.toml.5 $(DESTDIR)$(man5dir)/fsidx.toml.5

uninstall:
	rm $(DESTDIR)$(bindir)/fsidx
	rm $(DESTDIR)$(man1dir)/fsidx.1
	rm $(DESTDIR)$(man5dir)/fsidx.toml.5

clean: clean-man
	cargo clean

clean-man:
	rm -rf target/man

man: fsidx.1 fsidx.toml.5
fsidx.1: target/man/man1/fsidx.1
fsidx.toml.5: target/man/man5/fsidx.toml.5

target/man/man1/fsidx.1: doc/fsidx.1.md
	mkdir -p $(dir $@)
	pandoc -s -t man -o $@ $<

target/man/man5/fsidx.toml.5: doc/fsidx.toml.5.md
	mkdir -p $(dir $@)
	pandoc -s -t man -o $@ $<

view.fsidx: fsidx.1
	MANPATH=target/man man fsidx

view.fsidx.toml: fsidx.toml.5
	MANPATH=target/man man fsidx.toml

view.doc:
	cargo doc --open
