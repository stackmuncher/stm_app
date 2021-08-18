# Profile Builder for Global Directory of Software Developers

#### StackMuncher app analyses local Git repositories and creates a profile on https://stackmuncher.com, a Global Directory of Software Developers.

The code analysis is non-judgemental. It simply collects the facts such as what languages and frameworks are used, number of lines of code or library use. All that data is assembled into a Developer Profile to help someone looking for your skills find you. 


## Privacy

1. All code analysis is done locally. Not a single line of code is leaving your machine.
2. All identifying and sensitive information like file or private library names is stripped.
3. Your developer profile is completely anonymous unless you add your name and contact details to it.

## Examples

* anonymous profile: https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK
* public profile: https://stackmuncher.com/rimutaka
* sample report (local copy): ...
* sample report (stripped down): ...

## Quick start

_We are testing an alpha release and the only way to run this app for now is to compile it from the source in Rust._

Assuming that you have Git and a [Rust toolchain](https://www.rust-lang.org/tools/install) installed, just clone and run the app:

```bash
git clone https://github.com/stackmuncher/stm_app.git
cd stm_app
cargo run -- --project "path_to_any_of_your_local_projects"
```

The app will access `.git` folder inside `path_to_any_of_your_local_projects` directory and create an anonymous profile with your first report on _stackmuncher.com_. Add `--noupdate` flag to generate a report without creating a profile or submitting any data to the Directory. Look at the log printed by the app for details.

The **default config** of the app assumes that it is run on a development machine from the root folder of a repository you made commits to.

#### Example

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

#### Example

Find out what email addresses were used in commits to `quickxml_to_serde` repo:

```shell
~/rust/stm_app$ cd ~/rust/quickxml_to_serde/
~/rust/quickxml_to_serde$ git shortlog -s -e --all
     8  Alec Troemel <...>
     5  Marius Rumpf <...>
     3  alec troemel <...>
     1  mx <max@onebro.me>
    31  rimutaka <rimutaka@onebro.me>

```
_mx_ and _rimutaka_ are the same person. Add both emails to StackMuncher config using `--email` parameter:

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
The app stored two emails from `--emails` param in its config file and printed its full config info (`view_config` command). From now on it will look for commits from these two email addresses.


##  Adding more projects to your profile

1. Build the app: run `cargo build --release` from `stm_app` folder.
2. Add the full absolute path of `stm_app/target/release` folder to `PATH` environment variable
3. Add a global [post-commit  Git hook](https://git-scm.com/docs/githooks#_post_commit):

    ```bash
    git config --global init.templatedir '~/.git-templates'
    mkdir -p ~/.git-templates/hooks
    echo "stackmuncher >> ~/.stm.log" >> ~/.git-templates/hooks/post-commit
    chmod a+x ~/.git-templates/hooks/post-commit
    ```
4. Run `git init` on your existing repos to add the hook from the template. Any new repos or clones will get the hook added by default.

Git will invoke the app every time you make a commit to a repo with the post-commit hook to generate a report, log its progress in `~/.stm.log` and update your Directory Profile.

You can skip adding the Git hook and run `stackmuncher` from the root of any of your projects. No additional params are required.

## Detailed usage instructions

Running `stackmuncher` without any additional params generates a report for the project in the current working directory and updates your developer profile.

Anonymous profiles are identified by a public key from the key-pair generated by the app on the first run. E.g. https://stackmuncher.com/?dev=9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK
The key is located in the app's config folder and can be copied to another machine to connect to the same developer profile. Run `stackmuncher view_config` to see the exact location of the config folder.

### Additional options

Some settings are cached in a local config file and only need to be set once. You can set, change or unset them via CLI params or by editing the config file manually.

#### Processing settings

* `--emails "me@example.com,me@google.com"` : a list of your email addresses using in commits to include the report. Defaults to `git config user.email`. Run `git shortlog -s -e --all` to check if you made commits under other email addresses. Set once.
* `--project "path_to_project_to_be_analyzed"`: an optional relative or absolute path to the project/repo to generate a report for, defaults to the current working directory.
* `--no_update`: tells the app to generate a report, save it locally, but not upload anything to the Directory.

#### Profile settings

* `--primary_email "me@example.com"`: an email address for Directory notifications. Defaults to `git config user.email`. Set once. This setting is optional. No reports are submitted to the directory if this value is unset.
* `--public_name "My Full Name or Nickname"`: an optional public name of your Directory Profile. It is visible to anyone, including search engines. Leave it blank to remain anonymous. Set once.
* `--public_contact "email, website, twitter"`: optional contact details for your Directory Profile. It is visible to anyone, including search engines. Set once.

#### Debugging settings

* `--log error|warn|info|debug|trace`: the log is written to _stdout_ and _stderror_. Defaults to `error` for least verbose output. Redirect the output to a file or _null device_ to completely silence it. E.g. `stackmuncher --log debug >> ~/stm_trace.log`
* `--rules "path to code analysis rules"`: a path to an alternative location of code analysis rules. The path can be relative or absolute. Defaults to the application folder.
* `--reports "path to reports folder"`: a path to an alternative location for saving stack reports. The path can be relative or absolute. Defaults to the application folder.
* `--config "path to config folder"`: a path to an alternative location of the config folder. The path can be relative or absolute. Defaults to the application folder.

* `--help`: display usage info

#### Additional commands
* `view_config`: displays the contents of the config file and its location. The config file can be edited manually or copied to another machine together with the key file to connect to the same Developer Profile.


## Limitations

_The current version of the app is at alpha-stage and should be used for testing purposes only. _

1. Only a small number of computer languages are recognized.
2. There is no guarantee a profile will come up in a search via the front-end.
3. Profiles can be accessed via `/?dev=...` links only.
4. The app may include private library names in the report - do not use it on sensitive projects.
5. The only way to delete a profile is to email info@stackmuncher.com.
6. Your Github profile may already be included in the Directory, but it cannot be linked to your private profile.
7. It may take up to 2 minutes for a profile to be updated after a report submission.
8. Very large reports (over 50MB) are likely to be rejected.

## Troubleshooting

We want to hear about as many issues users run into as possible. Copy-paste the log and error messages printed by the app into https://github.com/stackmuncher/stm_app/issues/new/choose and let us help you.

#### Pre-requisites:

* Git is installed and its `user.email` setting was configured
* the project to be analyzed has commits from the same author/committer as in `user.email` setting

#### If the app did something, but no report was submitted:

* look through the log it printed for clues
* run `stackmuncher view_config` and check the output in `reports` folder - there should be at least 4 files:
    * **project_report.json**: includes all contributors 
    * **combined_report.json**: a combined report for authors/committers from Git's `user.email` setting and from `--emails` param
    * **submitted_report.json**: a sanitized version of the combined report exactly as it was submitted to the Directory
    * **contributor_xxxxxxxx.json**: cached reports for individual contributors

## Bug reports and contributions

File an issue via https://github.com/stackmuncher/stm_app/issues or email the maintainer on info@stackmuncher.com.