CARGO_FILES ?=
CARGO_TARGET_DIR ?=

M4_FLAGS += \
	-DCARGO_TARGET_DIR='${CARGO_TARGET_DIR}'

ifneq (${CARGO_TARGET_DIR},)
.cargo-prepare:	${CARGO_TARGET_DIR}/.dirstamp
endif

.cargo-prepare:	${CARGO_FILES}

.cargo-clean:
	rm -f ${CARGO_FILES}

all:		.cargo-prepare
prepare:	.cargo-prepare
clean:		.cargo-clean
