#!/bin/bash

# This script creates a PPA for the first time from scratch.
# Based on https://assafmo.github.io/2019/05/02/ppa-repo-hosted-on-github.html

# Remember to have a gpg key already installed on the machine.

# exit on error and echo all commands
set -e
set -v

cd ./distro/ubuntu

gpg --armor --export "info@stackmuncher.com" > KEY.gpg

# the / should be replaced with the channel at some point
# see https://wiki.debian.org/SourcesList
echo "deb https://distro.stackmuncher.com/ubuntu /" > stackmuncher.list
