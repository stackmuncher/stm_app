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
    // get input params
    let config = AppConfig::new().await;

    tracing_subscriber::fmt()
        .with_max_level(config.lib_config.log_level.clone())
        .with_ansi(false)
        //.without_time()
        .init();

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
        _ => {
            eprintln!("STACKMUNCHER ERROR: This command has not been implemented yet.");
            unimplemented!();
        }
    };

    Ok(())
}
