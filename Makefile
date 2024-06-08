IS_RELEASE ?=
IS_OFFLINE ?=
HAS_PROXY ?= t
RUST175_COMPAT ?=

DEFAULT_FEATURES ?= \
	$(if $(filter-out n,${HAS_PROXY}),proxy) \
	$(if ${RUST175_COMPAT},legacy_rust_175) \

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

PRECISE_PKG_175 =

update_175:	.cargo-update-precise-pre
	$(call cargo_update_precise,${PRECISE_PKG_175})

update-compat:	.cargo-update-precise-pre
update-compat:	$(if ${RUST175_COMPAT},update_175)

install:	install-fixup

install-fixup:	cargo-install
	${MKDIR_P} ${DESTDIR}${sbindir}
	mv ${DESTDIR}${bindir}/r-tftpd ${DESTDIR}${sbindir}/
	-@rmdir ${DESTDIR}${bindir}

clean:		clean-common

clean-common:
	rm -f ${CLEANFILES}
