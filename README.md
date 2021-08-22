# Software Developer Profile Builder

#### StackMuncher app helps developers find work that matches their skills and interests. It analyzes their commits in their local Git repositories and builds a profile in the [Open Directory of Software Developers](https://stackmuncher.com).

The code analysis is non-judgemental. It simply collects the facts such as what languages and frameworks are used, number of lines of code or library use. All that data is assembled into a Developer Profile to help someone looking for your skills to find you. 

## Table of Contents 

  - [Privacy](#privacy)
  - [Quick start](#quick-start)
  - [Adding more commit emails](#adding-more-commit-emails)
  - [Adding more projects to your profile](#adding-more-projects-to-your-profile)
  - [Detailed usage instructions](#detailed-usage-instructions)
    - [Additional options](#additional-options)
      - [Processing settings](#processing-settings)
      - [Profile settings](#profile-settings)
      - [Debug settings](#debug-settings)
      - [Additional commands](#additional-commands)
  - [Using StackMuncher app on multiple machines](#using-stackmuncher-app-on-multiple-machines)
  - [Limitations](#limitations)
  - [Troubleshooting](#troubleshooting)
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

_This is an alpha release and the only way to run this app is to compile it from the source in Rust._

Assuming that you have Git and a [Rust toolchain](https://www.rust-lang.org/tools/install) installed, just clone and run the app:

```bash
git clone https://github.com/stackmuncher/stm_app.git
cd stm_app
cargo run -- --project "path_to_any_of_your_local_projects"
```

The app will access `.git` folder inside `path_to_any_of_your_local_projects` directory and create an anonymous profile with your first report on _stackmuncher.com_. Add `--noupdate` flag to generate a report without creating a profile or submitting any data to the Directory. Look at the log printed by the app for details to examine the prepared report.

The **default config** of the app assumes that it is run on a development machine from the root folder of a repository you made commits to.

**Example**

I made commits to `~/rust/quickxml_to_serde` project recently and want to test StackMuncher app on it:
```shell
~/rust/stm_app$ cargo run -- --project "~/rust/quickxml_to_serde" --log error
    Finished dev [unoptimized + debuginfo] target(s) in 0.06s
     Running `target/debug/stackmuncher --project '~/rust/quickxml_to_serde' --log error`
   Stack report:         /home/ubuntu/rust/stm_app/reports/home_ubuntu_rust_quickxml_to_serde_git_9a32520d
   Directory profile:    https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK

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
~/rust/stm_app$ cargo run -- view_config --emails "max@onebro.me, rimutaka@onebro.me"

    Primary email: max@onebro.me
    Commit emails: max@onebro.me, rimutaka@onebro.me

    Public name:       not set
    Public contact:    not set
    Directory profile: https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK

    Local stack reports: /home/ubuntu/rust/stm_app/reports/home_ubuntu_rust_stm_app_6213a4b2
    Code analysis rules: /home/ubuntu/rust/stm_app/stm_rules
    Config file: /home/ubuntu/rust/stm_app/.stm_config/config.json
```
The app stored two emails from `--emails` param in its config file and printed its full config info (`view_config` command). From now on it will look for commits from _max@onebro.me_ and _rimutaka@onebro.me_.


##  Adding more projects to your profile

Adding more of your projects to your Directory Profile builds a more complete picture of your skills. StackMuncher can be configured to keep your profile current as you write and commit more code:

1. Build the app with `cargo build --release` from `stm_app` folder.
2. Add the full absolute path of `stm_app/target/release` folder to `PATH` environment variable. E.g. `echo 'export PATH="$HOME/rust/stm_app/target/release:$PATH"' >> ~/.profile` + log off/on or restart.
3. Check if you have Git hooks already configured: `git config --get-all init.templatedir`
   * _the query returned a value_ - edit your post-commit templates manually
   * _the query returned nothing_ - add a [post-commit  Git hook](https://git-scm.com/docs/githooks#_post_commit):
    ```bash
    git config --global --add init.templatedir '~/.git-templates'
    mkdir -p ~/.git-templates/hooks
    echo 'stackmuncher --log info 2>&1 >> ~/.stm.log' >> ~/.git-templates/hooks/post-commit
    chmod a+x ~/.git-templates/hooks/post-commit
    ```
4. Run `git init` on your existing repos to add the hook from the template. 
    * Any new repos or clones will get the hook added by default.
    * Repos with an existing `hooks/post-commit` file can have the hook added with `echo 'stackmuncher --log info 2>&1 >> ~/.stm.log' >> .git/hooks/post-commit`. Run it from the root of the project folder.

Git will invoke the app every time you make a commit to a repo with the post-commit hook to generate a report, log its progress in `~/.stm.log` and update your Directory Profile.

You can skip adding the Git hook and run `stackmuncher` from the root of any of your projects. No additional params are required.

## Detailed usage instructions

Running `stackmuncher` without any additional params generates a report for the project in the current working directory and updates your developer profile.

Anonymous profiles are identified by a public key from the key-pair generated by the app on the first run. E.g. https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK
The key is located in the app's config folder and can be copied to another machine to connect to the same developer profile. Run `stackmuncher view_config` command to see the exact location of the config folder.

### Additional options

Some of the app's settings are cached in a local config file and only need to be set once. You can set, change or unset them via CLI params or by editing the config file directly.

#### Processing settings

* `--emails "me@example.com,me@google.com"` : a list of your email addresses using in commits to include the report. Defaults to `git config user.email`. Run `git shortlog -s -e --all` to check if you made commits under other email addresses. Set once.
* `--project "path_to_project_to_be_analyzed"`: an optional relative or absolute path to the project/repo to generate a report for, defaults to the current working directory.
* `--dryrun`: tells the app to generate a report, save it locally, but not upload anything to the Directory.

Example:
```shell
~$ stackmuncher --project "~/rust/stm_server" --emails "max@onebro.me, rimutaka@onebro.me" --dryrun

   Stack report:         /home/ubuntu/rust/stm_app/reports/home_ubuntu_rust_stm_server_a8ff58d9
   Directory Profile update skipped: `--dryrun` flag.
```

#### Profile settings

* `--primary_email "me@example.com"`: an email address for Directory notifications. Defaults to `git config user.email`. Set once. This setting is optional. No reports are submitted to the directory if this value is unset.
* `--public_name "My Full Name or Nickname"`: an optional public name of your Directory Profile. It is visible to anyone, including search engines. Leave it blank to remain anonymous. Set once.
* `--public_contact "email, website, twitter"`: optional contact details for your Directory Profile. It is visible to anyone, including search engines. Set once.

Example:
```shell
~$ stackmuncher view_config --primary_email "max@onebro.me" --public_name "rimutaka" --public_contact "info@stackmuncher.com or https://rimutaka.me"

    Primary email: max@onebro.me
    Commit emails: max@onebro.me, rimutaka@onebro.me

    Public name:       rimutaka
    Public contact:    info@stackmuncher.com or https://rimutaka.me
    Directory profile: https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK

    Local stack reports: /home/ubuntu/rust/stm_app/reports
    Code analysis rules: /home/ubuntu/rust/stm_app/stm_rules
    Config file: /home/ubuntu/rust/stm_app/.stm_config/config.json
```

#### Debug settings

* `--log error|warn|info|debug|trace`: the log is written to _stdout_. Defaults to `error` for least verbose output. Redirect the output to a file or _null device_ to completely silence it. E.g. `stackmuncher --log debug >> ~/stm_trace.log`
* `--rules "path to code analysis rules"`: a path to an alternative location of code analysis rules. The path can be relative or absolute. Defaults to the application folder.
* `--reports "path to reports folder"`: a path to an alternative location for saving stack reports. The path can be relative or absolute. Defaults to the application folder.
* `--config "path to config folder"`: a path to an alternative location of the config folder. The path can be relative or absolute. Defaults to the application folder.

* `--help`: display usage info

#### Additional commands
* `view_config`: displays the contents of the config file and its location. The config file can be edited manually or copied to another machine together with the key file to connect to the same Developer Profile.

## Using StackMuncher app on multiple machines

1. Clone this repo onto a 2nd machine
2. Run `cargo run -- view_config` to bootstrap the app
3. Copy-paste the contents of `.stm_config/config.json` and `.stm_config/key.txt` from the 1st to the 2nd machine

The 2nd machine will be connected to the same Developer Profile as the first one for as long as they share the same _key.txt_ file. If you loose the key file the app will generate a new one and create a new Developer Profile. Contact us on info@stackmuncher.com to merge the old profile into the new one.

## Limitations

_The current version of the app is at alpha-stage and should be used for testing purposes only._

1. Only a small number of computer languages are recognized.
2. Profiles can be accessed via `/?dev=...` links only.
3. The app may include private library names in the report - do not use it on commercially-sensitive projects.
4. The only way to delete a profile is to email info@stackmuncher.com.
5. Your Github profile may already be included in the Directory, but it cannot be linked to your private profile.
6. It may take up to 2 minutes for a profile to be created/updated after a report submission.
7. Very large reports (over 50MB) are likely to be rejected.

## Troubleshooting

We want to hear about as many issues users run into as possible. Copy-paste the log and error messages printed by the app into https://github.com/stackmuncher/stm_app/issues/new/choose and let us help you.

**Pre-requisites:**

* Git is installed and its `user.email` setting was configured
* the project to be analyzed has commits from the same author/committer as in `user.email` setting

**If the app did something, but no report was submitted:**

* look through the log it printed for clues
* run `stackmuncher view_config` and check the output in `reports` folder - there should be at least 4 files:
    * _project_report.json_: includes all contributors 
    * _combined_report.json_: a combined report for authors/committers from Git's `user.email` setting and from `--emails` param
    * _submitted_report.json_: a sanitized version of the combined report exactly as it was submitted to the Directory
    * _contributor_xxxxxxxx.json_: cached reports for individual contributors

## Bug reports and contributions

File an issue via https://github.com/stackmuncher/stm_app/issues or email the maintainer on info@stackmuncher.com.