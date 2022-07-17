# About

r-tftpd is a tftp server with RFC 7440 "windowsize" support.

It allows only `RRQ` (read) requests; `WRQ` is **not** supported and
there are no plans to implement it.

# Usage

## standalone

```
cd /var/lib/tftpboot && r-tftpd --port 1234
```

Listening on privileged ports (e.g. the standard 69 one) requires the
`CAP_NET_BIND_SERVICE` capability (see `man 7 capabilities`).


## systemd socket activation

see contrib/


# TODO

- implementation of transparent proxying of tftp requests:

  - when local file is missing a fallback uri (http(s)) will be tried

  - pseudo virtual hosting: when a subdirectory is a symlink to an uri, file will be requested from there

# License

GPL-3.0 or later
