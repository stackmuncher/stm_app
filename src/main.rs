use std::error::Error;
use tracing::{info};

//  mod config;
//  mod processors;
//  mod report;
mod lib;

fn main() -> Result<(), Box<dyn Error>> {
    // get input params
    let params = lib::Params::new();

    tracing_subscriber::fmt()
        .with_max_level(params.log_level.clone())
        .with_ansi(false)
        //.without_time()
        .init();

    info!("Stack munching started ...");

    let instant = std::time::Instant::now();

    // load code rules
    let mut code_rules = lib::code_rules::CodeRules::new(&params.config_file_path);

    let report = lib::process_project(&mut code_rules, &params.project_dir_path)?;

    report.save_as_local_file(&params.report_file_name);

    info!("Done in {}ms", instant.elapsed().as_millis());

    Ok(())
}
