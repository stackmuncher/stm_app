use crate::config::AppConfig;
use path_absolutize::{self, Absolutize};
use tracing::info;

mod app_args;
mod cmd_munch;
mod config;
mod help;
mod signing;
mod submission;

#[tokio::main]
async fn main() -> Result<(), ()> {
    // generate the app config from a combo of default, cached and CLI params
    // and initialize the logging with either default or user-requested level
    let config = AppConfig::new().await;

    info!(
        "StackMuncher started in {} from {}",
        config.lib_config.project_dir.to_string_lossy(),
        std::env::current_exe()
            .expect("Cannot get path to stackmuncher executable. It's a bug.")
            .to_string_lossy()
    );

    info!(
        "Report folder: {}",
        config
            .lib_config
            .report_dir
            .as_ref()
            .expect("Cannot unwrap config.report_dir. It's a bug.")
            .absolutize()
            .expect("Cannot convert config.report_dir to absolute path. It's a bug.")
            .to_string_lossy()
    );

    info!(
        "Code rules folder: {}",
        config
            .lib_config
            .code_rules_dir
            .absolutize()
            .expect("Cannot convert config.code_rules_dir to absolute path. It's a bug.")
            .to_string_lossy()
    );

    #[cfg(debug_assertions)]
    info!("Running in debug mode");

    match config.command {
        app_args::AppArgCommands::Munch => {
            cmd_munch::run(config).await?;
        }
        app_args::AppArgCommands::DeleteProfile => {
            delete_profile();
        }
        app_args::AppArgCommands::MakeAnon => {
            make_anon();
        }
        app_args::AppArgCommands::ViewConfig => {
            view_config(config);
        }
        _ => {
            eprintln!("STACKMUNCHER ERROR: This command has not been implemented yet.");
            unimplemented!();
        }
    };

    Ok(())
}

/// A temporary stub for `delete_profile` command.
fn delete_profile() {
    println!("DELETE PROFILE: not implemented yet.");
    println!();
    println!("    Please, email info@stackmuncher.com and we'll delete it manually.");
}

/// A temporary stub for `make_anon` command.
fn make_anon() {
    let exe_suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
    println!("MAKE ANON: not implemented yet.");
    println!();
    println!("    Run `stackmuncher{} --public_name \"\" --public_contact \"\"` to remove your public details and make your profile anonymous.", exe_suffix);
}

/// A temporary stub for `view_config` command.
fn view_config(config: AppConfig) {
    println!(
        "Configuration file: {}",
        config
            .config_file_path
            .absolutize()
            .expect("Cannot convert config.config_file_path to absolute path. It's a bug.")
            .to_string_lossy()
    );
    println!();
    println!("    Primary email: {}", config.primary_email.as_ref().unwrap_or(&"not set".to_owned()));
    println!();
    println!("    Public name: {}", config.public_name.as_ref().unwrap_or(&"not set".to_owned()));
    println!();
    println!("    Public contact: {}", config.public_contact.as_ref().unwrap_or(&"not set".to_owned()));
    println!();
    println!(
        "    Local stack reports: {}",
        config
            .lib_config
            .report_dir
            .as_ref()
            .expect("config.report_dir is not set. It's a bug.")
            .absolutize()
            .expect("Cannot convert config.report_dir to absolute path. It's a bug.")
            .to_string_lossy()
    );
    println!();
    println!(
        "    Code analysis rules: {}",
        config
            .lib_config
            .code_rules_dir
            .absolutize()
            .expect("Cannot convert config.code_rules_dir to absolute path. It's a bug.")
            .to_string_lossy()
    );
    println!();
    println!("    Commit emails: {}", config.lib_config.git_identities.join(", "));
    println!();
    println!("Edit the config file manually as needed. The file is read anew on every run.");
}
