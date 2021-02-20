
# Building from source

## Latest Rust required

`cargo-crev` requires the latest stable version of Rust. If you have installed a Rust package from a slow-moving Linux distro, it's probably outdated and won't work. If you get compilation errors and warnings about unstable features, it means *your Rust version is too old*. Run:

```bash
rustup update
rustup default stable
```

If you don't have `rustup`, uninstall any Rust or Cargo package you may have, and [install Rust via `rustup`](https://rustup.rs/).

## OpenSSL dependency

Currently `cargo-crev` requires a non-Rust dependency to compile, as OpenSSL is required for TLS support.

Though OpenSSL is popular and readily available, it's virtually impossible to cover installing
it on all the available operating systems. We list some examples below. They should have matching commands and similar package names in the Unix-like OS of your choice.

In case of problems, don't hesitate to ask for help.

### Debian and Ubuntu

The following should work on Debian and Debian based distributions such as Ubuntu:

```bash
sudo apt-get install openssl libssl-dev
```

### Arch Linux

On Arch and Arch based distributions such as Manjaro make sure the latest OpenSSL is installed:

```bash
sudo pacman -Syu openssl
```

### RedHat

On RedHat and its derivates Fedora and CentOS the following should work:

```bash
sudo yum install openssl openssl-devel
```

### SuSE

On SuSE Linux the following should work:

```bash
sudo zypper install openssl libopenssl-devel
```

## Compiling

To compile and install latest `cargo-crev` release use `cargo`:

```bash
cargo install cargo-crev
```

In case you'd like to try latest features from the master branch, try:

```bash
cargo install --git https://github.com/crev-dev/cargo-crev/ cargo-crev
```

### Compiling from a local git checkout

```bash
cargo build --release -p cargo-crev
```

It will build `target/release/cargo-crev` executable.

## Support

If you have any trouble compiling, [ask in our gitter channel](https://gitter.im/dpc/crev) or use [pre-built binaries](https://github.com/crev-dev/cargo-crev/releases).
