#!/bin/sh

# An early draft of a local git hook install script for linux

# Download the executable onto a local folder and add it to PATH
mkdir -p ~/bin/stackmuncher
aws s3 cp s3://$STM_S3_BUCKET_PROD_BOOTSTRAP/apps/stackmuncher ~/bin/stackmuncher/stackmuncher
chmod u+x ~/bin/stackmuncher/stackmuncher

# Enable git templates:
git config --global init.templatedir '~/.git-templates'

# Create a directory to hold the global hooks:
mkdir -p ~/.git-templates/hooks

# Add stm executable to post-commit
touch ~/.git-templates/hooks/post-commit
chmod u+x ~/.git-templates/hooks/post-commit
if ! grep -q "stackmuncher" ~/.git-templates/hooks/post-commit ; then 
  echo "~/bin/stackmuncher/stackmuncher" >> ~/.git-templates/hooks/post-commit 
fi