use crate::config::AppConfig;
use path_absolutize::{self, Absolutize};
use tracing::info;

mod app_args;
mod cmd_config;
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
        "Analyzing {} from {}",
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
            cmd_config::view_config(config).await;
        }
        app_args::AppArgCommands::Help => {
            help::emit_welcome_msg(config);
        }
        app_args::AppArgCommands::GitGHubConfig => {
            cmd_config::github(config).await;
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
    println!("MAKE ANON: not implemented yet.");
    println!();
    println!("    Run `stackmuncher{} --public_name \"\" --public_contact \"\"` to remove your public details and make your profile anonymous.", std::env::consts::EXE_SUFFIX);
}
