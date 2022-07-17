CONTRIB_SYSTEMD_UNITS = \
	contrib/r-tftpd.service \
	contrib/r-tftpd.socket \

CONTRIB_SED_CMD = \
	-e 's!/usr/sbin!${sbindir}!g' \

CLEANFILES += \
	contrib/r-tftpd.service

install:	install-systemd

install-systemd:	${CONTRIB_SYSTEMD_UNITS}
	${MKDIR_P} ${DESTDIR}/${systemd_system_unitdir}
	${INSTALL_DATA} $^ ${DESTDIR}/${systemd_system_unitdir}/

contrib/%:	contrib/%.in
	rm -f $@
	sed ${CONTRIB_SED_CMD} < '$<' > '$@'
	chmod a-w '$@'
