#!/bin/bash

# exit on error and echo all commands
set -e
set -v

# remove any pre-existing .deb files
rm target/debian/*

# bump the patch number inside stackmuncher/Cargo.toml
cd ./stackmuncher
cargo semver bump patch

# build and package .deb from the root
cd ..
cargo deb -p stackmuncher

# copy all debs into the distro folder
cp target/debian/stackmuncher*.deb distro/ubuntu/



# Packages & Packages.gz
# Entry 'Filename: /stackmuncher_0.1.0_amd64.deb' causes problems with CloudFront.
# E.g. https://distro.stackmuncher.com/ubuntu/./stackmuncher_0.1.2_amd64.deb  403  Forbidden
# It has to be cleaned up
cd ./distro/ubuntu/
dpkg-scanpackages --multiversion . > Packages
sed -i 's+Filename: ./+Filename: +g' Packages
gzip -k -f Packages

# Release, Release.gpg & InRelease
apt-ftparchive release . > Release
gpg --default-key "info@stackmuncher.com" -abs -o - Release > Release.gpg
gpg --default-key "info@stackmuncher.com" --clearsign -o - Release > InRelease

# upload everything to S3 and invalidate the cloudfront cache
cd ../..
aws s3 cp ./distro/ubuntu/ s3://stm-ppa-7b4g14ydlm/ubuntu/ --recursive

# index update
echo "This domain is used to distribute StackMuncher software packages for different platforms. Learn more from https://github.com/stackmuncher/stm/distro." > distro\index.txt
echo "" >> distro/index.txt
aws s3 ls s3://stm-ppa-7b4g14ydlm/ --recursive --summarize >> distro/index.txt
aws s3 cp distro/index.txt s3://stm-ppa-7b4g14ydlm/index.txt
aws cloudfront create-invalidation --distribution-id E102XVLT2KLJHS --paths "/"
