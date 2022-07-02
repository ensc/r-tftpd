%:	%.m4
	@rm -f '$@' '$@.tmp'
	${M4} ${M4_FLAGS} '$<' > '$@.tmp'
	@chmod a-w '$@.tmp'
	@mv '$@.tmp' '$@'
