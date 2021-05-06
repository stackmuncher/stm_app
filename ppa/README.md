# StackMuncher PPA

StackMuncher apps are distributed via [https://distro.stackmuncher.com](https://distro.stackmuncher.com).

That domain name is a mirror of *ppa* folder, where this README is located. We do not commit the executables to the repo, though.
They are uploaded straight into an S3 bucket and is accessed via CloudFront.

## Folder contents

* **ubuntu**: everything needed for the PPA to work with `apt` command, except for *.deb* files that are sent straight to S3 to avoid clogging the repo
* **create_ppa.sh**: use it to re-create the PPA or some parts of it, e.g. if the domain or the key change
* **update.sh**: run it from the root folder of the project to create a new package and upload it to S3 

# Build system for StackMuncher app

## Linux

The app should be deployed to `/usr/bin/stackmuncher` and the rules to `/usr/share/stackmuncher/stm_rules/`. The choice is based on [Linux Filesystem Hierarchy Standard](https://www.pathname.com/fhs/).

### deb package

deb builds rely on [cargo-deb](https://crates.io/crates/cargo-deb) crate. See config inside [stackmuncher/Cargo.toml](stackmuncher/Cargo.toml) `[package.metadata.deb]` table.
* build deb: `cargo deb -p stackmuncher`
* bump the patch #: `cargo semver bump patch`
* package for PPA: `. ppa/update.sh`
* inspect deb: `dpkg-deb -x /home/ubuntu/rust/stackmuncher/target/debian/stackmuncher_0.1.0_amd64.deb .`
* about deb packages: https://blog.packagecloud.io/eng/2015/10/13/inspect-extract-contents-debian-packages/