CARGO ?= cargo
CARGO_TOOLCHAIN ?=
CARGO_FILES ?=
CARGO_TARGET_DIR ?=

CARGO_ACTUAL_TARGET_DIR ?= $(shell ${__cargo_target_dir})

M4_FLAGS += \
	-DCARGO_TARGET_DIR='${CARGO_TARGET_DIR}'

__cargo_target_dir = ${CARGO} ${CARGO_TOOLCHAIN} metadata \
	--offline --no-deps --format-version 1 | \
	sed 's!.*"target_directory":"\([^"]\+\)",.*!\1!'

ifneq (${CARGO_TARGET_DIR},)
.cargo-prepare:	${CARGO_TARGET_DIR}/.dirstamp
endif

.cargo-prepare:	${CARGO_FILES}

.cargo-clean:
	rm -f ${CARGO_FILES}

all:		.cargo-prepare
prepare:	.cargo-prepare
clean:		.cargo-clean

CARGO_BUILD_FLAGS ?=
AM_CARGO_BUILD_FLAGS = \
	$(if ${IS_RELEASE},--release) \
	$(if ${IS_OFFLINE},--frozen --offline) \

CARGO_TEST_FLAGS ?=
AM_CARGO_TEST_FLAGS = \
	$(if ${PKG},--package ${PKG},--workspace) \
	$(if ${IS_RELEASE},--release) \
	--frozen --offline \

CARGO_CHECK_FLAGS ?=	${CARGO_TEST_FLAGS}
AM_CARGO_CHECK_FLAGS = \
	${AM_CARGO_TEST_FLAGS} \
	--tests \

CARGO_DOC_FLAGS ?=
AM_CARGO_DOC_FLAGS = \
	$(if ${IS_RELEASE},--release) \
	--frozen --offline \

CARGO_INSTALL_FLAGS ?=
AM_CARGO_INSTALL_FLAGS = \
	$(if ${IS_RELEASE},,--debug) \
	--force --frozen --offline \

__cargo_op = ${CARGO} ${CARGO_TOOLCHAIN} $1 $2

_cargo_build = $(call __cargo_op,$1,build) \
	$(if ${PKG},--package ${PKG}) \
	${AM_CARGO_BUILD_FLAGS} \
	${CARGO_BUILD_FLAGS} \
	$2

_cargo_test = $(call __cargo_op,$1,test) \
	${AM_CARGO_TEST_FLAGS} \
	${CARGO_TEST_FLAGS} \
	$2 --

_cargo_check = $(call __cargo_op,$1,check) \
	${AM_CARGO_CHECK_FLAGS} \
	${CARGO_CHECK_FLAGS} \
	$2

_cargo_doc = $(call __cargo_op,$1,doc) \
	${AM_CARGO_DOC_FLAGS} \
	${CARGO_DOC_FLAGS} \
	$2

_cargo_install = $(call __cargo_op,$1,install) \
	${AM_CARGO_INSTALL_FLAGS} \
	${CARGO_INSTALL_FLAGS} \
	--path '${srcdir}' \
	--root '${DESTDIR}/${prefix}' \
	$2

lint:		cargo-clippy
build:		cargo-build
install:	cargo-install
mrproper:	cargo-clean
test:		cargo-test
check:		cargo-check
version-info:	cargo-version-info

cargo-update:	FORCE
	$(call __cargo_op,$1,update)

cargo-build:	FORCE
	$(call _cargo_build,,)

cargo-test:	export RUST_BACKTRACE=1
cargo-test:
	$(call _cargo_test,,)

cargo-check:
	$(call _cargo_check,,)

cargo-clippy:	FORCE
	$(call __cargo_op,$1,clippy ${AM_CARGO_FLAGS} $(if ${PKG},--package ${PKG},--workspace) --tests)

cargo-clean:
	$(call __cargo_op,,clean)

cargo-doc:
	$(call _cargo_doc,,)

cargo-install:	FORCE
	$(call _cargo_install,,)
	@rm -f ${DESTDIR}${prefix}/.crates.toml
	@rm -f ${DESTDIR}${prefix}/.crates2.json

cargo-run:	FORCE
	$(call _cargo_run,,$P)

cargo-version-info:	FORCE
	@echo "============ CARGO ============"
	@$(call __cargo_op,,tree --workspace --depth 1)
	@echo
