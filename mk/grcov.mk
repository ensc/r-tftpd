GRCOV_STYLE ?= llvm

ifeq (${RUN_GRCOV},)
# noop
else ifeq (${GRCOV_STYLE},llvm)
  export RUSTFLAGS	   = -C instrument-coverage
  export LLVM_PROFILE_FILE = ${CARGO_ACTUAL_TARGET_DIR}/debug/cov/sstate_server-%p-%m.profraw
  GRCOV_STYLE_LLVM = t
else
  $(error "Unsupported GRCOV style ${GRCOV_STYLE}")
endif

GRCOV ?=		grcov
GRCOV_FLAGS ?=	\
	--ignore "/*" \
	$(if ${CARGO_HOME},--ignore "$(patsubst $(abspath ${srcdir})/%,%,$(abspath ${CARGO_HOME}))/*") \
	--ignore-not-existing \
	--llvm \
	--branch \
	$(if ${GIT_REV_FULL},--commit-sha '${GIT_REV_FULL}') \
	$(if ${GIT_REV_BRANCH},--vcs-branch '${GIT_REV_BRANCH}') \

GRCOV_OUTPUT_BASE ?=	coverage

GRCOV_CMD = ${GRCOV} ${GRCOV_FLAGS} \
	-t $* -o $@ -s '${srcdir}' \
	--binary-path '${CARGO_ACTUAL_TARGET_DIR}/debug' \
	$(if ${GRCOV_STYLE_LLVM},${CARGO_ACTUAL_TARGET_DIR}/debug/cov) \
	$(if ${GRCOV_STYLE_GCDA},${CARGO_ACTUAL_TARGET_DIR}/debug) \

run-grcov:	${GRCOV_OUTPUT_BASE}.cobertura ${GRCOV_OUTPUT_BASE}.lcov ${GRCOV_OUTPUT_BASE}.html
	${MAKE} run-grcov-post

run-grcov-post:

${GRCOV_OUTPUT_BASE}.%:	FORCE
	rm -rf $@
	${GRCOV_CMD} src
