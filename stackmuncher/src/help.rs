use stackmuncher_lib::config::Config;

/// Prints out a standard multi-line message on how to use the app and where to find more info
pub(crate) fn emit_usage_msg() {
    println!("Launch StackMuncher app from the root folder of your project with a Git repository in .git subfolder.");
    println!("The app will analyze the Git repo and produce a report.");
    println!("");
    println!("{}", crate::config::CMD_ARGS);
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
