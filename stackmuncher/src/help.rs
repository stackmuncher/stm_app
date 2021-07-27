use stackmuncher_lib::config::Config;

/// Prints out a standard multi-line message on how to use the app and where to find more info
pub(crate) fn emit_usage_msg() {
    let exe_suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
    println!();
    println!(
        "    Run `stackmuncher{exe_suffix} --help` for detailed usage info.",
        exe_suffix = exe_suffix
    );
    println!();
    emit_support_msg();
}

/// Prints out a standard multi-line message on where to find more info
pub(crate) fn emit_support_msg() {
    println!("Source code: https://github.com/stackmuncher/stm");
    println!("Support: https://github.com/stackmuncher/stm/issues or info@stackmuncher.com");
}

/// Prints out info on where the rules are expected
pub(crate) fn emit_code_rules_msg() {
    println!("");
    if cfg!(debug_assertions) {
        println!("The default location for StackMuncher code rules in DEBUGGING MODE is `{}` sub-folder of the current working directory.", Config::RULES_FOLDER_NAME_DEBUG);
    } else if cfg!(target_os = "linux") {
        println!(
            "The default location for StackMuncher code rules on Linux is `{}` folder.",
            Config::RULES_FOLDER_NAME_LINUX
        );
    } else if cfg!(target_os = "windows") {
        println!(
            "The default location for StackMuncher code rules on Windows is `{}` folder placed next stackmuncher.exe.",
            Config::RULES_FOLDER_NAME_WIN
        );
    }
    println!();
    println!("    To specify a different location use `--rules` param followed by a relative or absolute path to the rules folder.");
    println!("    The latest copy of the rules can be downloaded from https://github.com/stackmuncher/stm repo or https://distro.stackmuncher.com/stm_rules.zip");
    println!();
    emit_support_msg();
}

/// Prints out info on where the reports can be saved
pub(crate) fn emit_report_dir_msg() {
    println!("");
    if cfg!(debug_assertions) {
        println!("The default location for StackMuncher reports in DEBUGGING MODE is `{}` sub-folder of the current working directory.", Config::REPORT_FOLDER_NAME_DEBUG);
    } else if cfg!(target_os = "linux") {
        println!(
            "The default location for StackMuncher reports on Linux is `{}` folder.",
            Config::REPORT_FOLDER_NAME_LINUX
        );
    } else if cfg!(target_os = "windows") {
        println!(
            "The default location for StackMuncher reports on Windows is `{}` folder.",
            Config::REPORT_FOLDER_NAME_WIN
        );
    }
    println!();
    println!("    To specify a different location use `--report` param followed by a relative or absolute path to the reports folder.");
    println!();
    emit_support_msg();
}

/// Prints out either Win or nix/Mac msg with --log info instructions on getting more info
pub(crate) fn emit_detailed_output_msg() {
    if cfg!(target_os = "windows") {
        eprintln!("To see detailed output run `stackmuncher.exe --log info` from the project root directory (where .git folder is).");
    } else {
        eprintln!("To see detailed output run `stackmuncher --log info` from the project root directory (where .git folder is).");
    }
}

/// Prints a message about invalid args and exits with code 1.
pub(crate) fn emit_cli_err_msg() {
    if cfg!(target_os = "windows") {
        eprintln!(
            "Cannot parse the parameters from the command line. Run `stackmuncher.exe --help` for usage details."
        );
    } else {
        eprintln!("Cannot parse the parameters from the command line. Run `stackmuncher --help` for usage details.");
    }
}

/// Prints out either Win or nix/Mac Welcome msg.
pub(crate) fn emit_welcome_msg() {
    let exe_suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };

    println!("\
StackMuncher app analyzes your technology stack and showcases it in the Global Directory of Software Developers.

USAGE:
    stackmuncher{exe_suffix}                        analyze the Git repo in the current folder and create or update your Directory Profile
    stackmuncher{exe_suffix} automate               add a git-commit hook to update your Directory Profile automatically
    stackmuncher{exe_suffix} [command] [OPTIONS]    modify the default behavior of this app
    
YOUR DIRECTORY PROFILE:
    An anonymous profile is created in the Directory the first time you run this app.
    * Tell employers who you are: `stackmuncher{exe_suffix} --public_name \"Name or Nickname\" --public_contact \"Email, website, twitter\"`
    * Become anonymous again: `stackmuncher{exe_suffix} make_anon`
    * Skip submitting any data to the Directory: use `--no_update` flag

CODE PRIVACY:
    All code analysis is done locally. Not a single line of code is leaving your machine. View the source code at https://github.com/stackmuncher.

OPTIONS:
    --no_update                                   skip updating your Directory Profile
    
    --primary_email \"me@example.com\"              for Directory notifications only, defaults to the address in `git config user.email` setting
    
    --emails \"me@example.com,me@google.com\"       a list of all your commit emails, only need to use it once, defaults to `git config user.email`

    --public_name \"My Full Name or Nickname\"      visible to anyone, leave it blank to remain anonymous, only need to use it once

    --public_contact \"email, website, twitter\"    visible to anyone, leave it blank to remove, only need to use it once

    --project \"path to project to be analyzed\"    can be relative or absolute, defaults to the current working directory

    --rules \"path to code analysis rules\"         can be relative or absolute, defaults to the application folder

    --reports \"path to reports folder\"            can be relative or absolute, defaults to the application folder

    --config \"path to config folder\"              can be relative or absolute, defaults to the application folder

    --log error|warn|info|debug|trace             defaults to `error` for least verbose output

    --help                                        display this message

ADDITIONAL COMMANDS:
    view_config, make_anon, delete_profile

MORE INFO:
    https://stackmuncher.com/about      about the Directory
    https://github.com/stackmuncher     source code, issues and more
    ",exe_suffix=exe_suffix);
}
