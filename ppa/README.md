# StackMuncher PPA

StackMuncher apps are distributed via [https://distro.stackmuncher.com](https://distro.stackmuncher.com).

That domain name is a mirror of *ppa* folder, where this README is located. We do not commit the executables to the repo, though.
They are uploaded straight into an S3 bucket and is accessed via CloudFront.

## Folder contents

* **ubuntu**: everything needed for the PPA to work with `apt` command, except for *.deb* files that are sent straight to S3 to avoid clogging the repo
* **create_ppa.sh**: use it to re-create the PPA or some parts of it, e.g. if the domain or the key change
* **update.sh**: run it from the root folder of the project to create a new package and upload it to S3 
