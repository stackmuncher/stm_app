use serde_json;
use std::error::Error;
use std::fs;
use tracing::{error, info};

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

    // load config
    let conf = fs::File::open(&params.config_file_path).expect("Cannot read config file");
    let mut conf: lib::config::Config = serde_json::from_reader(conf).expect("Cannot parse config file");

    // pre-compile regex rules for file names
    for file_rules in conf.files.iter_mut() {
        file_rules.compile_file_name_regex();
    }

    let report = lib::process_project(&params, &mut conf)?;

    report.save_as_local_file(&params.report_file_name);

    info!("Done in {}ms",instant.elapsed().as_millis());

    Ok(())
}
