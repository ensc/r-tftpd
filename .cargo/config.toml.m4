[build]
m4_ifelse(CARGO_TARGET_DIR,`',`',`m4_dnl
target-dir = "CARGO_TARGET_DIR"')
m4_dnl

m4_divert(-1)
m4_syscmd(`for i in .cargo/config.d/*.yml; do test ! -e "$i" || cat "$i"; done')
