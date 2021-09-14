# Software Developer Profile Builder

#### StackMuncher app helps developers find work that matches their skills and interests. It analyzes their commits in their local Git repositories and builds a profile in the [Open Directory of Software Developers](https://stackmuncher.com).

The code analysis is non-judgemental. It simply collects the facts such as what languages and frameworks are used, number of lines of code or library use. All that data is assembled into a Developer Profile to help someone looking for your skills to find you. 

## Table of Contents 

  - [Privacy](#privacy)
  - [Quick start](#quick-start)
  - [Adding more commit emails](#adding-more-commit-emails)
  - [Adding more projects to your profile](#adding-more-projects-to-your-profile)
  - [Making your profile public](#making-your-profile-public)
  - [Using StackMuncher app on multiple machines](#using-stackmuncher-app-on-multiple-machines)
  - [Detailed usage instructions](#detailed-usage-instructions)
      - [Processing settings](#processing-settings)
      - [Profile settings](#profile-settings)
      - [Debug settings](#debug-settings)
      - [Additional info](#additional-info)
  - [Limitations](#limitations)
  - [Troubleshooting](#troubleshooting)
  - [Building from source](#building-from-source)
  - [Bug reports and contributions](#bug-reports-and-contributions)

## Privacy

1. All code analysis is done locally. Not a single line of code is leaving your machine.
2. All identifying and sensitive information such as file, project or private library names is stripped.
3. Your developer profile is completely anonymous unless you add your name and contact details to it.

The app creates a sample stack report on the first run over a project without submitting it (dryrun). You can review the report before continuing.

**Examples**

* anonymous profile: https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK
* public profile: https://stackmuncher.com/rimutaka
* sample report: [samples/submission.json](samples/submission.json)

## Quick start

1. Download the latest binary from GitHub
   * Linux (GNU): `sudo curl -o /usr/local/bin/stackmuncher -L https://github.com/stackmuncher/stm_app/releases/download/v0.1.6/stackmuncher-x86_64-unknown-linux-gnu && sudo chmod 755 /usr/local/bin/stackmuncher`
   * Linux (MUSL): `sudo curl -o /usr/local/bin/stackmuncher -L https://github.com/stackmuncher/stm_app/releases/download/v0.1.6/stackmuncher-x86_64-unknown-linux-musl && sudo chmod 755 /usr/local/bin/stackmuncher`
   * macOS: `sudo curl -o /usr/local/bin/stackmuncher -L https://github.com/stackmuncher/stm_app/releases/download/v0.1.6/stackmuncher-x86_64-apple-darwin && sudo chmod 755 /usr/local/bin/stackmuncher`
   * Windows: `invoke-webrequest -uri https://github.com/stackmuncher/stm_app/releases/download/v0.1.6/stackmuncher-x86_64-pc-windows-msvc.exe -outfile $env:windir\stackmuncher.exe`
2. Change the current directory to one of your projects with a Git repository (has _.git_ subfolder) and run:
   * Linux: `stackmuncher`
   * Windows PowerShell: `stackmuncher`

The app will access the local Git repository for the current directory and create a stack report, but will NOT submit any data to the Directory to let you review the stack report first. It will start updating your profile on subsequent runs unless `--dryrun` flag is used.

**Example**

I made commits to `~/rust/xml_to_serde` project recently and now want to test StackMuncher app on it:
```shell
~$ cd rust/xml_to_serde
~/rust/xml_to_serde$ stackmuncher
   Summary (LoC/libs):  Rust 1265/6, Markdown 187, Bash 16
   Stack reports:       /home/mx/stackmuncher/reports/home_ubuntu_rust_xml_to_serde_git_9a32520d
   Project added to:    https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK

```

## Adding more commit emails

We often commit to the same repo using different `user.email` Git setting. Run `git shortlog -s -e --all` to check if you made commits under any other email addresses.

**Example**

Find out what email addresses were used in commits to `xml_to_serde` repo:

```shell
~/rust/xml_to_serde$ git shortlog -s -e --all
     8  Alex Trump <...>
     5  Martin Crump <...>
     3  alex trump <...>
     1  mx <max@onebro.me>
    31  rimutaka <rimutaka@onebro.me>

```
_mx_ and _rimutaka_ are the same person. Let's add both emails to StackMuncher config using `--email` parameter:

```shell
~/$ stackmuncher config --emails "max@onebro.me, rimutaka@onebro.me"

    Primary email: max@onebro.me
    Commit emails: max@onebro.me, rimutaka@onebro.me

    Anonymous profile: https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK
    Public profile: disabled
    GitHub validation: not set

    Stack reports: /home/mx/stackmuncher/reports/
    Config folder: /home/mx/stackmuncher/config/
    Executable:    /usr/local/bin/stackmuncher
```
The app stored two emails from `--emails` param in its config file and printed its full config info (`config` command). From now on it will look for commits from _max@onebro.me_ and _rimutaka@onebro.me_.

##  Adding more projects to your profile

Adding more of your projects to your Directory Profile builds a more complete picture of your skills. StackMuncher can be configured to keep your profile current as you write and commit more code:

1. Make sure `stackmuncher` executable is placed in a folder included in `PATH` environment variable or add its folder to `PATH`.
2. Check if you have Git hooks already configured: `git config --get-all init.templatedir`
   * _if the query returned a value_ - edit your post-commit templates manually
   * _if the query returned nothing_ - add a [post-commit  Git hook](https://git-scm.com/docs/githooks#_post_commit):

    #### Shell
    ```bash
    git config --global --add init.templatedir '~/.git-templates'
    mkdir -p ~/.git-templates/hooks
    echo 'stackmuncher --log info 2>&1 >> ~/.stm.log' >> ~/.git-templates/hooks/post-commit
    chmod a+x ~/.git-templates/hooks/post-commit
    ```
    #### PowerShell
    ```powershell
    git config --global --add init.templatedir '~/.git-templates'
    mkdir -p ~/.git-templates/hooks
    echo '#!/bin/sh' >> ~/.git-templates/hooks/post-commit
    echo 'stackmuncher --log info 2>&1 >> ~/.stm.log' >> ~/.git-templates/hooks/post-commit
    ```

3. Run `git init` on your existing repos to add the hook from the template. 
    * Any new repos or clones will get the hook added by default.
    * Repos with an existing `hooks/post-commit` file can have the hook added with `echo 'stackmuncher --log info 2>&1 >> ~/.stm.log' >> .git/hooks/post-commit`. Run it from the root of the project folder.

Git will invoke the app every time you make a commit to a repo with the post-commit hook to generate a report, log its progress in `~/.stm.log` and update your Directory Profile.

You can skip adding the Git hook and run `stackmuncher` from the root of any of your projects. No additional params are required.

## Making your profile public

**Anonymous profiles** are identified by a public key from the key-pair generated by the app on the first run. E.g. https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK
The profile can be viewed by anyone with the link, but it cannot be discovered otherwise.

**Public profiles** are searchable by employers looking for software developers. Your public profile will be created with the same login and personal details as your GitHub profile.

E.g. https://stackmuncher.com/rimutaka has _contact details_ and _public projects_ from https://github.com/rimutaka as well as _private projects_ from https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK.

1. Run `stackmuncher github`
2. Use the signature it printed for a private Gist to confirm your GitHub account ownership
3. Run `stackmuncher config --gist [URL of the Gist]`

## Using StackMuncher app on multiple machines

1. Download `stackmuncher` executable to a 2nd machine
2. Run `stackmuncher config` on both machines to see the location of config folders
3. Copy-paste the contents of `.stm_config/config.json` and `.stm_config/key.txt` from the 1st to the 2nd machine

The 2nd machine will be connected to the same Developer Profile as the first one for as long as they share the same _key.txt_ and _config.json_ files. If you loose the key file the app will generate a new one and create a new Developer Profile. Contact us on info@stackmuncher.com to merge the old profile into the new one.

## Detailed usage instructions

Running `stackmuncher` without any additional params generates a report for the project in the current working directory and updates your Developer Profile.

Some of the app's settings are cached in a local config file and only need to be set once. You can set, change or unset them via CLI params or by editing _config.json_ file directly.

#### Processing settings

* `--emails "me@example.com,me@google.com"` : a list of your email addresses used in commits to to be analyzed. Defaults to `git config user.email`. Run `git shortlog -s -e --all` to check if you made commits under other email addresses. _Set once._
* `--project "path_to_project_to_be_analyzed"`: an optional relative or absolute path to the project/repo to analyze, defaults to the current working directory.
* `--dryrun`: tells the app to generate a report, save it locally, but not upload anything to the Directory.

Example:
```shell
~$ stackmuncher --project "~/rust/stm_server" --emails "max@onebro.me, rimutaka@onebro.me" --dryrun

   Summary (LoC/libs):  Rust 12656/26, Markdown 587, PowerShell 169
   Stack reports:       /home/mx/stackmuncher/reports/home_ubuntu_rust_stm_server_a8ff58d9
   Profile update:      skipped with `--dryrun` flag
```

#### Profile settings

* `--primary_email "me@example.com"`: an optional email address for Directory notifications only. Defaults to `git config user.email`. _Set once._

Example:
```shell
~$ stackmuncher config --primary_email "max+jobs@onebro.me"

    Primary email: max+jobs@onebro.me
    Commit emails: max@onebro.me, rimutaka@onebro.me

    Anonymous profile: https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK
    Public profile: https://stackmuncher.com/rimutaka
    GitHub validation: https://gist.github.com/rimutaka/fb8fc0f87ee78231f064131022c8154a

    Stack reports: /home/mx/stackmuncher/reports/
    Config folder: /home/mx/stackmuncher/config/
    Executable:    /usr/local/bin/stackmuncher
```

#### Debug settings

* `--log error|warn|info|debug|trace`: the log is written to _stdout_. Defaults to `error` for least verbose output. Redirect the output to a file or _null device_ to completely silence it. E.g. `stackmuncher --log debug >> ~/stm_trace.log`
* `--reports "path to reports folder"`: a path to an alternative location for saving stack reports. The path can be relative or absolute. Defaults to a platform-specific user-data location. Set once.
* `--config "path to config folder"`: a path to an alternative location of the config folder. The path can be relative or absolute. Defaults to a platform-specific user-data location.

#### Additional info

* `stackmuncher help`: displays usage info.
* `stackmuncher config`: display the contents of the config file and its location. The config file can be edited manually.

## Limitations

_The current version of the app is at alpha-stage and should be used for testing purposes only._

1. Only a small number of computer languages are recognized.
2. The app may unintentionally include private library names in the report - do not use it on commercially-sensitive projects.
3. The only way to delete a profile is to email info@stackmuncher.com.
4. It may take up to 2 minutes for a profile to be created/updated after a report submission.
5. Very large reports (over 50MB) are likely to be rejected.
6. Repositories with more than 10,000 files are not processed.

## Troubleshooting

We want to hear about as many issues users run into as possible. Copy-paste the log and error messages printed by the app into https://github.com/stackmuncher/stm_app/issues/new/choose and let us help you.

**Pre-requisites:**

* Git is installed and its `user.email` setting was configured
* the project to be analyzed has commits from the same author/committer as in `user.email` setting

**If the app did something, but no report was submitted:**

* look through the log it printed for clues
* run `stackmuncher config` and check the output in `reports` folder - there should be at least 4 files:
    * _project_report.json_: includes all contributors 
    * _combined_report.json_: a combined report for authors/committers from Git's `user.email` setting or from `--emails` param
    * _submission.json_: a sanitized version of the combined report exactly as it is submitted to the Directory
    * _contributor_xxxxxxxx.json_: cached reports for individual contributors

## Building from source

Assuming that you have Git and a [Rust toolchain](https://www.rust-lang.org/tools/install) installed, just clone the repo and run the app:

```bash
git clone https://github.com/stackmuncher/stm_app.git
cd stm_app
cargo run -- --log error --project "path_to_any_of_your_local_projects"
```

## Bug reports and contributions

File an issue via https://github.com/stackmuncher/stm_app/issues or email the maintainer on info@stackmuncher.com.