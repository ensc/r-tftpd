CARGO_FILES = \
	.cargo/config \
	Cargo.toml \

all:

include mk/init.mk

-include ${HOME}/.config/rust/common.mk
-include ${HOME}/.config/rust/r-tftpd.mk

include mk/m4.mk
include mk/paths.mk
include mk/cargo.mk
