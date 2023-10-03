IS_RELEASE ?=
IS_OFFLINE ?=
HAS_PROXY ?= t
RUST166_COMPAT ?=

DEFAULT_FEATURES ?= \
	$(if $(filter-out n,${HAS_PROXY}),proxy) \
	$(if ${RUST166_COMPAT},legacy_rust_166) \

CARGO_FILES = \
	.cargo/config \
	Cargo.toml \

srcdir = .

all:

include mk/init.mk

-include ${HOME}/.config/rust/common.mk
-include ${HOME}/.config/rust/r-tftpd.mk
-include .local.mk

include mk/m4.mk
include mk/paths.mk
include mk/tools.mk
include mk/cargo.mk
include mk/cargo-util.mk
include mk/grcov.mk

include contrib/Makefile.mk

PRECISE_PKG_166 = \
	clap@4.3.24 \
	clap_lex@0.5.0 \
	anstyle@1.0.2 \
	anstyle-parse@0.2.1 \

update_166:	.cargo-update-precise-pre
	$(call cargo_update_precise,${PRECISE_PKG_166})

update-compat:	.cargo-update-precise-pre
update-compat:	$(if ${RUST166_COMPAT},update_166)

install:	install-fixup

install-fixup:	cargo-install
	${MKDIR_P} ${DESTDIR}${sbindir}
	mv ${DESTDIR}${bindir}/r-tftpd ${DESTDIR}${sbindir}/
	-@rmdir ${DESTDIR}${bindir}

clean:		clean-common

clean-common:
	rm -f ${CLEANFILES}
