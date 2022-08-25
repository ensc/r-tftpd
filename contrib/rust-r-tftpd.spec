%bcond_with check

%global rev	f3c71630fd3dbbefa28456228c31563f21ff5077
%global crate	r-tftpd

Name:           rust-%crate
Version:        0.0.7
Release:        1%{?dist}
Summary:        tftp server

License:        GPLv3

Source0:        rust-%crate.tar.xz
Source1:	cargo-vendor.tar.xz

ExclusiveArch:  %rust_arches
BuildRequires:	rust cargo make m4
BuildRequires:	openssl-devel pkg-config
BuildRequires:	systemd
BuildRequires:	systemd-rpm-macros
%{?systemd_requires}

%description
rev %rev

%package     -n %crate
Summary:        %summary.

%description -n %crate
%summary.

%prep
%setup -q
%setup -q -T -D -a 1

## create .local.mk with local setup
cat <<EOF > .local.mk
export RUSTFLAGS = -Copt-level=3 -Cdebuginfo=2 -Clink-arg=-Wl,-z,relro,-z,now

IS_RELEASE = t
IS_OFFLINE = t
prefix = %_prefix
bindir = %_bindir
sbindir = %_sbindir
systemd_system_unitdir = %_unitdir

%if %{?rhel}0 > 0
## TODO: remove when EL8 ships a recent 'rust'
export RUSTC_BOOTSTRAP = 1
CARGO = cargo -Z namespaced-features
%endif

EOF

%build
make prepare
make build

%install
make install DESTDIR=$RPM_BUILD_ROOT

%if %{with check}
%check
make test
%endif

%post -n %crate
%systemd_post  %crate.socket

%preun -n %crate
%systemd_preun %name.socket %crate.service

%postun -n %crate
%systemd_postun_with_restart %crate.socket

%files -n %crate
%_sbindir/r-tftpd
%_unitdir/r-tftpd.*

%changelog
