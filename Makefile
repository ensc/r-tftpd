IS_RELEASE ?=
IS_OFFLINE ?=
HAS_PROXY ?= t
RUST166_COMPAT ?=

DEFAULT_FEATURES ?= \
	$(if $(filter-out n,${HAS_PROXY}),proxy) \

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
include mk/grcov.mk

include contrib/Makefile.mk

M4_FLAGS += \
	$(if ${RUST166_COMPAT},-DRUST166_COMPAT=t)

install:	install-fixup

install-fixup:	cargo-install
	${MKDIR_P} ${DESTDIR}${sbindir}
	mv ${DESTDIR}${bindir}/r-tftpd ${DESTDIR}${sbindir}/
	-@rmdir ${DESTDIR}${bindir}

clean:		clean-common

clean-common:
	rm -f ${CLEANFILES}
