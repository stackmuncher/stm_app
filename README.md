# StackMuncher Library and Client App

StackMuncher is a language-agnostic code analysis tool that answers one question: 
> What is my stack and how well do I know it?

This library and a CLI app run on demand or as a GIT hook to analyse the committed code and produce a report with contributor metrics.

The code analysis is un-opinionated. It does not impose any rules, passes a judgement or benchmarks one contributor against the other. Its function is limited to fact collection:
* programming languages used
* language keywords and features
* libraries and dependencies used
* number of lines of code and their types (comments, white space, docs, code)

## Installation

StackMuncher is a single executable file written in Rust. Its only external dependency is `git` that must be installed on the same machine and configuration files in JSON format.

Run StackMuncher client app from the root of your project with a child `.git` folder.
The app will access the contents of the repository, not the working folder. The reports are saved in `.git/stm-reports` folder.

## Ubuntu

```shell
curl -SsL https://stackmuncher.github.io/ppa/ubuntu/KEY.gpg | sudo apt-key add -
sudo curl -SsL -o /etc/apt/sources.list.d/stackmuncher.list https://stackmuncher.github.io/ppa/ubuntu/stackmuncher.list
sudo apt update
sudo apt install stackmuncher
```

### Post-commit Git hook

The best way to run StackMuncher client app is via a global [post-commit](https://git-scm.com/docs/githooks#_post_commit) Git hook to update your stack reports automatically every time you make a new commit.

This script downloads StackMuncher from GitHub and configures it as a global post-commit hook for all your repositories. Pick and choose what lines to run if you already have post-commit hooks configured:

```bash
git config --global init.templatedir '~/.git-templates'
mkdir -p ~/.git-templates/hooks
echo "stackmuncher" >> ~/.git-templates/hooks/post-commit
chmod a+x ~/.git-templates/hooks/post-commit
```

#### Enabling hooks in individual repositories

* **new and cloned repositories**: no action needed, the hook will be installed from `~/.git-templates` folder on creation
* **existing repositories WITH NO post-commit hooks**: run `git init` inside existing repositories to enable the hook
* **existing repositories WITH OTHER post-commit hooks**: manually edit `.git/hooks/post-commit` file to add `stackmuncher` line, probably at the very end to protect your workflow if the app crashes

Running StackMuncher as a post-commit hook is reasonably safe. Even if the app crashes it will not affect the commit or delay your workflow by more than a few milliseconds. It may take up to a few seconds on the very first run with large repositories.

### Manual run

You can run StackMuncher on any repository at any time. It is completely idempotent and will generate reports using the current state of the repository.

This script downloads StackMuncher from GitHub and add it to PATH in .bashrc

```bash
wget https://github.com/users/rimutaka/packages/some-pkg-id -o /usr/bin/stackmuncher
chmod a+x /usr/bin/stackmuncher
echo -e 'export PATH="/usr/bin/stackmuncher:$PATH"' >> ~/.bashrc
```

### CLI parameters

* `--rules [code_rules_dir]` or use `STACK_MUNCHER_CODERULES_DIR` env var: instructs the app where to look for language rules files. The default location is `/usr/bin/stackmuncher/assets` folder.
* `--project [project_path]`: instructs the app to process the project from the specified directory. The default project location is the current working directory.

Running `stackmuncher` without any parameters will use defaults and is the most common scenario.

## Report types

The app produces a number of different report types. They are all stored in `.git/stm-reports` folder and re-used internally for incremental processing of commits. Removing the reports will force full reprocessing on the repo.

### Project report

The project report contains project stats and some contributor info. It only includes files from the current tree.

### Contributor reports

Contributor reports are generated per contributor to isolate the work of each person. They are run on all files touched by the contributor using the latest contributor commit of that file. E.g. you fixed a bug in `src/utils.rs` 3 years ago and have not touched that file since. The app will use the file as it was committed by you then.

### Contributor identity

It is possible that the same person made commits under different `user.name` / `user.email` identities. They can be automatically reconciled by adding the identities to `stm.identity` custom setting in GIT config:

```bash
git config --global --add stm.identity me@example.com
```

Re-run the line above multiple times to add more than one identity. The app will track identity changes after the install and add them to the list automatically. Only identities that were used before the install need to be added manually.

The app creates one report per identity and `contributor_report.json` for all known identities.

## Privacy

1. The app accesses the contents of the repository, not the working folder. If your secrets were committed to the repo there is a tiny chance they may leak into the report, e.g. as a keyword or a name of a library.
2. The code extracted from the repo is analysed in memory and discarded. It is not copied, cached, saved in temp files or submitted anywhere.