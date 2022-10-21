# About

r-tftpd is a tftp server with RFC 7440 "windowsize" support and it can
relay tftp requests to http servers.

It allows `RRQ` (read) requests; `WRQ` support is incomplete and
exists only for testing purposes.

# Implemented standards

 - [RFC 1350 "THE TFTP PROTOCOL"](https://www.rfc-editor.org/rfc/rfc1350):

   - `RRQ`: yes
   - `WRQ`: only for testing purposes; e.g. accepts only the data but does not store it.  No window size support either
   - implements only the "octet" ("binary") transfer mode; "netascii" and "mail" are **not** supported
   - block ids will wrap around from 65535 to 0

 - [RFC 2347 "TFTP Option Extension"](https://www.rfc-editor.org/rfc/rfc2347.html):

   - can be disabled (for testing purposes) by the `--no-rfc2374` flag

 - [RFC 2348 "TFTP Blocksize Option"](https://datatracker.ietf.org/doc/html/rfc2348)

 - [RFC 2349 "TFTP Timeout Interval and Transfer Size Options"](https://datatracker.ietf.org/doc/html/rfc2349)

 - [RFC 7440 "TFTP Windowsize Option"](https://www.rfc-editor.org/rfc/rfc7440)
   - only for `RRQ`, but **not** for `WRQ`

# Usage

```
Usage: r-tftpd [OPTIONS]

Options:
  -s, --systemd                use systemd fd propagation
  -p, --port <PORT>            port to listen on [default: 69]
  -l, --listen <IP>            ip address to listen on [default: ::]
  -m, --max-connections <NUM>  maximum number of connections [default: 64]
  -t, --timeout <TIMEOUT>      timeout in seconds during tftp transfers [default: 3]
  -f, --fallback <URI>         fallback uri
  -L, --log-format <FMT>       log format [default: default] [possible values: default, compact, full, json]
  -C, --cache-dir <DIR>        directory used for cache files
      --no-rfc2374             disable RFC 2373 (OACK) support; only useful for testing some clients
      --wrq-devnull            accept WRQ but throw it away; only useful for testing some clients
      --disable-proxy          disable proxy support
  -h, --help                   Print help information
  -V, --version                Print version information
```

## build

```
make
cargo build
```

see [r-tftp.spec](file://contrib/rust-r-tftpd.spec) for ways how to
customize it by using makefile variables.

## standalone

```
cd /var/lib/tftpboot && r-tftpd --port 1234
```

Listening on privileged ports (e.g. the standard 69 one) requires the
`CAP_NET_BIND_SERVICE` capability (see `man 7 capabilities`).


## systemd socket activation

see contrib/

# Proxy mode

"r-tftpd" supports relaying of tftp requests to other servers.  It
allows pseudo virtual hosting by creating (dead) symlinks pointing to
an url.

## supported uris

- `http://` + `https://`

Schemes accept the following, "plus" sign separated modifiers:

- `nocache`: downloaded resources will not be cached; by default usual
  http caching mechanisms (`Cache-Control`, `Etag`, ...)  are applied
  and resources are kept locally.  They are not accessible on disk but
  created by `O_TMPFILE`.

  The cache is cleared periodically

-  `nocompress`: resources are requested with `identity` encoding; by
  default, compression is enabled.  When compression is enabled, the
  whole file must be downloaded when starting the transaction because
  its size can not be determined else.

  Without compression, `Content-Length` information are used and tftp
  upload and http download happen in parallel, This helps to avoid
  tftp timeouts

## examples

```
$ tree
.
├── domain1 -> http+nocache://domain1.example.org/
├── domain2 -> http+nocompress://domain2.example.org/
├── existing
├── remote-file -> http+nocompress+nocache://domain3.example.org/some-file
└── subdir
    └── file

$ r-tftp --fallback http://fallback.example.org/
```

 | requested path | returned resource                                | flags                                |
 |----------------|--------------------------------------------------|--------------------------------------|
 | `existing`     | local `existing file`                            |                                      |
 | `subdir/file`  | local `subdir/file`                              |                                      |
 | `domain1/foo`  | remote `http://domain1.example.org/foo`          | without caching                      |
 | `domain2/bar`  | remote `http://domain2.example.org/bar`          | without http compression             |
 | `remote-file`  | remote `http://fallback.example.org/remote-file` | without http compression nor caching |
 | `not-here`     | remote `http://fallback.example.org/not-here`    |                                      |

# License

GPL-3.0 or later
