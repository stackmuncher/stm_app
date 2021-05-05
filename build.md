# Build system for StackMuncher app

## Linux

The app should be deployed to `/usr/bin/stackmuncher` and the rules to `/usr/share/stackmuncher/stm_rules/`. The choice is based on [Linux Filesystem Hierarchy Standard](https://www.pathname.com/fhs/).

### deb package

deb builds rely on [cargo-deb](https://crates.io/crates/cargo-deb) crate. See config inside [stackmuncher/Cargo.toml](stackmuncher/Cargo.toml) `[package.metadata.deb]` table.
* build deb: `cargo deb -p stackmuncher`
* inspect deb: `dpkg-deb -x /home/ubuntu/rust/stackmuncher/target/debian/stackmuncher_0.1.0_amd64.deb .`
* about deb packages: https://blog.packagecloud.io/eng/2015/10/13/inspect-extract-contents-debian-packages/

