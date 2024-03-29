use crate::config::{self, AppConfig};
use crate::signing::ReportSignature;

/// Prints out a standard multi-line message on how to use the app and where to find more info
pub(crate) fn emit_usage_msg() {
    println!();
    println!("    Run `stackmuncher help` for detailed usage info.");
    println!();
    emit_support_msg();
}

/// Prints out a standard multi-line message on where to find more info
pub(crate) fn emit_support_msg() {
    println!("Source code and support: https://github.com/stackmuncher/stm_app or info@stackmuncher.com");
}

/// Prints out info on where the reports can be saved
pub(crate) fn emit_report_dir_msg() {
    println!("");
    if cfg!(debug_assertions) {
        println!("The default location for StackMuncher reports in DEBUGGING MODE is `{}` sub-folder of the current working directory.", config::REPORT_FOLDER_NAME_DEBUG);
    } else if cfg!(target_os = "linux") {
        println!(
            "The default location for StackMuncher reports on Linux is `{}` folder.",
            config::REPORT_FOLDER_NAME_LINUX
        );
    } else if cfg!(target_os = "windows") {
        println!(
            "The default location for StackMuncher reports on Windows is `{}` folder.",
            config::REPORT_FOLDER_NAME_WIN
        );
    }
    println!();
    println!("    To specify a different location use `--reports` param followed by a relative or absolute path to the reports folder.");
    println!("    The path will be saved and used for any subsequent runs.");
    println!();
    emit_support_msg();
}

/// Prints out info on where the config is located
pub(crate) fn emit_config_dir_msg() {
    println!("");
    if cfg!(debug_assertions) {
        println!("The default location for StackMuncher config in DEBUGGING MODE is `{}` sub-folder of the current working directory.", config::CONFIG_FOLDER_NAME_DEBUG);
    } else if cfg!(target_os = "linux") {
        println!(
            "The default location for StackMuncher config on Linux is `{}` folder.",
            config::CONFIG_FOLDER_NAME_LINUX
        );
    } else if cfg!(target_os = "windows") {
        println!(
            "The default location for StackMuncher config on Windows is `{}` folder.",
            config::CONFIG_FOLDER_NAME_WIN
        );
    }
    println!();
    println!("    To specify a different location use `--config` param followed by a relative or absolute path to the config folder.");
    println!();
    emit_support_msg();
}

/// Prints out either Win or nix/Mac msg with --log info instructions on getting more info
pub(crate) fn emit_detailed_output_msg() {
    eprintln!(
        "To see detailed output run `stackmuncher --log info` from the project root directory (where .git folder is)."
    );
}

/// Prints a message about invalid args and exits with code 1.
pub(crate) fn emit_cli_err_msg() {
    eprintln!("Cannot parse the parameters from the command line. Run `stackmuncher help` for usage details.");
}

/// Prints a message about an invalid private key.
pub(crate) fn emit_key_err_msg(key_file_path: &str) {
    eprintln!();
    eprintln!("    1. Did you manually copied the contents of `key.txt`? It is invalid. Try again.");
    eprintln!();
    eprintln!(
        "    2. If you didn't edit {} you can delete it and the app will generate a new one.",
        key_file_path
    );
    eprintln!("    The side effect of that is that the app will also create a new Directory Profile for you.");
    eprintln!("    If you think you lost your original key, just delete the file and contact us on info@stackmuncher.com to link your existing Directory Profile to the new key.");
    eprintln!("    We apologize for not automating this step yet.");
    eprintln!();
}

/// Prints a message about a the first run over a repo.
pub(crate) fn emit_dryrun_msg(report_file_path: &str) {
    eprintln!();
    eprintln!("A stack report was generated, but NOT submitted.");
    eprintln!();
    eprintln!(
        "    If your project code is commercially sensitive you can check {} file to make sure the stack report is safe to submit.",
        report_file_path
    );
    eprintln!();
    eprintln!("    The app will add this project to your profile at the next run unless `--dryrun` flag is specified.");
    eprintln!("    Run `stackmuncher` again to add it now.");
    eprintln!();
}

/// Prints a message about validation Gist troubleshooting.
pub(crate) fn emit_gist_troubleshooting(gist_id: &str, uri: &str) {
    eprintln!();
    eprintln!("Troubleshooting:");
    eprintln!("    1. Try again if it was a networking or some other intermittent problem.");
    eprintln!(
        "    2. Check the response with `curl -i -H 'Accept: application/vnd.github.v3+json' {}",
        uri
    );
    eprintln!(
        "       The Gist ID ({}) was taken from `--gist` CLI param. Check if it is correct.",
        gist_id
    );
    eprintln!();
}

/// Prints a message about validation Gist signature troubleshooting.
pub(crate) fn emit_gist_instructions(gist_content: &String) {
    eprintln!();
    eprintln!("GitHub account validation steps:");
    eprintln!("    1. Navigate to https://gist.github.com to create a new Gist.");
    eprintln!("      * description: \"stm validation\" or any other value");
    eprintln!("      * file name: \"stm.txt\" or any other value");
    eprintln!("      * content: \"{}\"", gist_content);
    eprintln!("    2. Use the URL of the saved Gist to run `stackmuncher config --gist [URL goes here]'");
    eprintln!();
    eprintln!(
        "Example: `stackmuncher config --gist https://gist.github.com/rimutaka/fb8fc0f87ee78231f064131022c8154a`"
    );
    eprintln!();
}

/// Prints out either Win or nix/Mac Welcome msg.
pub(crate) fn emit_welcome_msg(config: AppConfig) {
    // directory profile URL depends on if it's public or not
    let dir_profile_url = match &config.gh_login {
        Some(login) if !login.is_empty() => format!("https://stackmuncher.com/{}", login),
        _ => format!("https://stackmuncher.com/?dev={}", ReportSignature::get_public_key(&config.user_key_pair)),
    };

    let profile_msg = match &config.gh_login {
        Some(login) if !login.is_empty() => String::new(),
        _ => format!(
            "
    An anonymous profile is created in the Directory the first time you run this app.
    Run `stackmuncher github` to make your profile public with details from your GitHub account.
    ",
        ),
    };

    println!("\
StackMuncher app analyzes your technology stack and showcases it in the Global Directory of Software Developers.

USAGE:
    stackmuncher                        analyzes the Git repo in the current folder and creates or updates your Directory Profile
    stackmuncher [command] [OPTIONS]    modifies the default behavior of this app
    
YOUR DIRECTORY PROFILE: 

    {dir_profile_url}
    {profile_msg}
CODE PRIVACY:
    All code analysis is done locally. Not a single line of code is leaving your machine. View the source code at https://github.com/stackmuncher.

OPTIONS:
    --emails \"me@example.com,me@google.com\"       a list of all your commit emails, only need to use it once, defaults to `git config user.email`

    --primary_email \"me@example.com\"              for Directory notifications only, defaults to the address in `git config user.email` setting
    --gist                                         a URL of your GitHub login validation Gist, run `stackmuncher github` for details

    --project \"path to project to be analyzed\"    can be relative or absolute, defaults to the current working directory
    --reports \"path to reports folder\"            can be relative or absolute, defaults to the application folder
    --config \"path to config folder\"              can be relative or absolute, defaults to the application folder

    --log error|warn|info|debug|trace             defaults to `error` for least verbose output
    --dryrun                                      skip updating your Directory Profile (no data leaves your computer)

MORE INFO:

    stackmuncher config                 prints the URL of your Directory Profile and other configuration details
    stackmuncher help                   displays this message

    https://stackmuncher.com/about      about the Directory
    https://github.com/stackmuncher     source code, issues and more
    ", dir_profile_url=dir_profile_url, profile_msg=profile_msg);
}
