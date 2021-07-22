use stackmuncher_lib::config::Config;

/// Prints out a standard multi-line message on how to use the app and where to find more info
pub(crate) fn emit_usage_msg() {
    println!("Launch StackMuncher app from the root folder of your project with a Git repository in .git subfolder.");
    println!("Add --help for more info");
    println!("");
    println!("");
    emit_support_msg();
}

/// Prints out a standard multi-line message on where to find more info
pub(crate) fn emit_support_msg() {
    println!("Source code and usage instructions: https://github.com/stackmuncher/stm");
    println!("Bug reports and questions: https://github.com/stackmuncher/stm/issues or mailto:info@stackmuncher.com");
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
    println!("To specify a different location use `--rules` param followed by a relative or absolute path to the rules folder.");
    println!("The latest copy of the rules can be downloaded from https://github.com/stackmuncher/stm repo or https://distro.stackmuncher.com/stm_rules.zip");
    println!("");
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
    println!("To specify a different location use `--report` param followed by a relative or absolute path to the reports folder.");
    println!("");
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
    This app generates technology stack reports for your projects and adds them to your profile on stackmuncher.com, a Global Directory of Software Developers.
    Use --no_update flag to NOT submit any data to the Directory.
    
    YOUR DIRECTORY PROFILE
    
        An anonymous profile is created on stackmuncher.com the first time you run this app. You can add more details...
        * to tell employers who you are: `stackmuncher{exe_suffix} --public_name \"My Full Name or Nickname\" --public_contact \"Email, website, twitter\"`
        * to become anonymous again: `stackmuncher{exe_suffix} --public_name \"\" --public_contact \"\"`
    
    CODE PRIVACY:
        All code analysis is done locally. Not a single line of code is leaving your machine. Run `stackmuncher view_reports` to see the reports.
    
    USAGE:
        stackmuncher{exe_suffix}                        analyzes the project in the current folder and updates your profile, uses `git config user.email` setting to find your commits
    
        stackmuncher{exe_suffix} automate               add a git-commit hook to update your Directory profile automatically
    
        stackmuncher{exe_suffix} [command] [OPTIONS]    modify the default behavior of the app
    
    
    OPTIONS:
        --no_update                                     do not update my Directory profile
        
        --primary_email \"me@mydomain.com\"             for Directory notifications, kept private, defaults to the address in `git config user.email` setting
        
        --emails \"me@gmail.com,me@other.com\"          list of emails used in your commits, defaults to the address in `git config user.email` setting
    
        --public_name \"My Full Name or Nickname\"      name of your profile or leave it blank to remain anonymous, only need to set once
    
        --public_contact \"Email, website, twitter\"    contact details in your profile or leave it blank to remove them, only need to set once
    
        --project \"path to project to be analyzed\"    can be relative or absolute, defaults to the current working directory
    
        --rules \"path to code analysis rules\"         can be relative or absolute, defaults to platform-specific application folder
    
        --reports \"path to reports folder\"            can be relative or absolute, defaults to platform-specific application folder
    
        --log error|warn|info|debug|trace               defaults to `error` for least verbose output
    
    ADDITIONAL COMMANDS:
        help, view_reports, make_anon, delete_profile
    
    MORE INFO:
        https://stackmuncher.com/about      about the Directory
        https://github.com/stackmuncher     source code, issues and more
    ",exe_suffix=exe_suffix);
}
