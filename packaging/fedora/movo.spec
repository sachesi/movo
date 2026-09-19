%define _debugsource_template %{nil}
%define debug_package %{nil}

Name:           movo
# The release workflow and Copr set Version to the tag they build.
Version:        0.4.4
Release:        1%{?dist}
Summary:        GTK 4 and Libadwaita client for HDRezka

License:        GPL-3.0-only
URL:            https://github.com/sachesi/movo
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz
# The crates the build needs, from the release, so that it runs without a network.
Source1:        %{url}/releases/download/v%{version}/%{name}-%{version}-vendor.tar.xz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  gettext
BuildRequires:  desktop-file-utils
BuildRequires:  pkgconfig(gtk4) >= 4.22
BuildRequires:  pkgconfig(libadwaita-1) >= 1.9
BuildRequires:  pkgconfig(openssl)

Recommends:     mpv

%description
Movo is an unofficial client for the HDRezka streaming site, written in Rust.
It provides a GTK 4 and Libadwaita desktop application for browsing catalogs,
searching titles, managing favorites and watch history, and playing streams
via an external player such as mpv.

%prep
%autosetup -n %{name}-%{version} -b 1
rm -f rust-toolchain.toml

%build
export CARGO_HOME=$PWD/.cargo-home
export RUSTFLAGS="%{?build_rustflags}"
cargo build --release -p movo --offline --locked

%install
install -Dpm 0755 target/release/movo %{buildroot}%{_bindir}/movo
install -Dpm 0644 crates/movo-desktop/data/io.github.sachesi.Movo.desktop \
  %{buildroot}%{_datadir}/applications/io.github.sachesi.Movo.desktop
install -Dpm 0644 crates/movo-desktop/data/io.github.sachesi.Movo.metainfo.xml \
  %{buildroot}%{_datadir}/metainfo/io.github.sachesi.Movo.metainfo.xml
install -Dpm 0644 crates/movo-desktop/data/icons/hicolor/scalable/apps/io.github.sachesi.Movo.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/io.github.sachesi.Movo.svg
install -Dpm 0644 crates/movo-desktop/data/icons/hicolor/symbolic/apps/io.github.sachesi.Movo-symbolic.svg \
  %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/io.github.sachesi.Movo-symbolic.svg
install -Dpm 0644 crates/movo-desktop/data/icons/hicolor/512x512/apps/io.github.sachesi.Movo.png \
  %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/io.github.sachesi.Movo.png

for po in po/*.po; do
  [ -f "$po" ] || continue
  lang=$(basename "$po" .po)
  install -d "%{buildroot}%{_datadir}/locale/${lang}/LC_MESSAGES"
  msgfmt -o "%{buildroot}%{_datadir}/locale/${lang}/LC_MESSAGES/movo.mo" "$po"
done

%find_lang movo

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/io.github.sachesi.Movo.desktop
test -x %{buildroot}%{_bindir}/movo

%files -f movo.lang
%license LICENSE
%doc README.md
%{_bindir}/movo
%{_datadir}/applications/io.github.sachesi.Movo.desktop
%{_datadir}/metainfo/io.github.sachesi.Movo.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/io.github.sachesi.Movo.svg
%{_datadir}/icons/hicolor/symbolic/apps/io.github.sachesi.Movo-symbolic.svg
%{_datadir}/icons/hicolor/512x512/apps/io.github.sachesi.Movo.png

%changelog
