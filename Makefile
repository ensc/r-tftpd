IS_RELEASE ?=
IS_OFFLINE ?=
HAS_PROXY ?= t
RUST179_COMPAT ?=

DEFAULT_FEATURES ?= \
	$(if $(filter-out n,${HAS_PROXY}),proxy) \
	$(if ${RUST179_COMPAT},legacy_rust_179) \

CARGO_FILES = \
	.cargo/config.toml \
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

PRECISE_PKG_179 =

update_179:	.cargo-update-precise-pre
	$(call cargo_update_precise,${PRECISE_PKG_179})

update-compat:	.cargo-update-precise-pre
update-compat:	$(if ${RUST179_COMPAT},update_179)

install:	install-fixup

install-fixup:	cargo-install
ifeq (${bindir},${sbindir})
	@:
else
	${MKDIR_P} ${DESTDIR}${sbindir}
	mv ${DESTDIR}${bindir}/r-tftpd ${DESTDIR}${sbindir}/
	-@rmdir ${DESTDIR}${bindir}
endif

clean:		clean-common

clean-common:
	rm -f ${CLEANFILES}
