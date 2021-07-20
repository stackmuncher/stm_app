use path_absolutize::{self, Absolutize};
use regex::Regex;
use stackmuncher_lib::{config::Config, git::check_git_version, git::get_local_identities, utils::hash_str_sha1};
use std::path::Path;
use std::process::exit;
use crate::help;

pub(crate) const CMD_ARGS: &'static str = "Optional CLI params: [--rules path_to_folder_with_alternative_code_rules], \
[--project path_to_project_to_be_analyzed] defaults to the current directory, \
[--report path_to_reports_folder] defaults to a platform specific location, \
[--emails email1,email2,email3] a list of emails for additional contributor to include in the report, \
[--log error|warn|info|debug|trace] defaults to `error`.";

/// Inits values from ENV vars and the command line arguments
pub(crate) async fn new_config() -> Config {
    // look for the rules in the current working dir if in debug mode
    // otherwise default to a platform-specific location
    // this can be overridden by `--rules` CLI param
    let (code_rules_dir, report_dir, log_level) = if cfg!(debug_assertions) {
        (
            Path::new(Config::RULES_FOLDER_NAME_DEBUG).to_path_buf(),
            Path::new(Config::REPORT_FOLDER_NAME_DEBUG).to_path_buf(),
            tracing::Level::INFO,
        )
    } else if cfg!(target_os = "linux") {
        (
            Path::new(Config::RULES_FOLDER_NAME_LINUX).to_path_buf(),
            Path::new(Config::REPORT_FOLDER_NAME_LINUX).to_path_buf(),
            tracing::Level::ERROR,
        )
    } else if cfg!(target_os = "windows") {
        // the easiest way to store the rules on Win is next to the executable
        let exec_dir = match std::env::current_exe() {
            Err(e) => {
                // in theory, this should never happen
                panic!(
                    "No current exe path: {}. This is a bug. The app should at least see the path to its own executable.",
                    e
                );
            }
            Ok(v) => v
                .parent()
                .expect(&format!(
                    "Cannot determine the location of the exe file from: {}",
                    v.to_string_lossy()
                ))
                .to_path_buf(),
        };

        // apps should store their data in the user profile and the exact location is obtained via an env var
        let local_appdata_dir = std::env::var("LOCALAPPDATA").expect("%LOCALAPPDATA% env variable not found");
        let local_appdata_dir = Path::new(&local_appdata_dir);
        (
            exec_dir.join(Config::RULES_FOLDER_NAME_WIN),
            local_appdata_dir.join(Config::REPORT_FOLDER_NAME_WIN),
            tracing::Level::ERROR,
        )
    } else {
        unimplemented!("Only Linux and Windows are supported at the moment");
    };

    // assume that the project_dir is the current working folder
    let project_dir = std::env::current_dir().expect("Cannot access the current directory.");

    // init the minimal config structure with the default values
    let config = Config {
        log_level,
        code_rules_dir,
        report_dir: Some(report_dir),
        project_dir,
        user_name: String::new(),
        repo_name: String::new(),
        git_remote_url_regex: Regex::new(Config::GIT_REMOTE_URL_REGEX).unwrap(),
        git_identities: Vec::new(),
    };

    // check if there were any arguments passed to override the ENV vars
    let config = read_cli_params(config);

    rules_dir_check(&config);
    project_dir_check(&config);
    
    // check if GIT is installed
    // this check will change to using the git supplied as part of STM package
    if let Err(_e) = check_git_version(&config.project_dir).await {
        eprintln!(
            "STACKMUNCHER CONFIG ERROR: Cannot launch Git from {} folder. Is it installed on this machine?",
            config.project_dir.to_string_lossy()
        );
       help::emit_usage_msg();
        exit(1);
    };

    let config = report_dir_check(config);

    let config = git_identity_check(config).await;

    config
}

/// Converts case insensitive level as String into Enum, defaults to INFO
fn string_to_log_level(s: String) -> tracing::Level {
    match s.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        _ => {
            // the user specified something, but is it not a valid value
            // it may still be better off to complete the job with some extended logging, so defaulting to INFO
            println!("STACKMUNCHER CONFIG ERROR. Invalid tracing level. Use TRACE, DEBUG, WARN or ERROR. Choosing INFO level.");
            return tracing::Level::INFO;
        }
    }
}

/// Shortens a potentially long folder name like home_mx_projects_stm_stm_apps_stm_28642a39
/// to a reasonable length of about 250 bytes, which can be 250 ASCII chars or much fewer for UTF-8.
/// The trimming is done by cutting off segments at the _ from the start.
/// The worst case scenario it will be just the hash left.
fn trim_canonical_project_name(name: String) -> String {
    let mut name = name;

    // windows - 260, linux - 255, mac - 255, but that is in chars
    // it gets tricky with UTF-8 because some chars are multi-byte, so technically it is even shorter
    // having a very long name is kind of pointless - most useful info is at the end,
    // anything over 100 glyphs would be hard to read
    while name.as_bytes().len() > 250 {
        // cut off the first segment_
        // there should be at least one _ under the 255 limit because the hash is at the end and it's only 8 chars long
        let cut_off_idx = name
            .find("_")
            .expect("Failed to trim a canonical project name. It's a bug")
            + 1;
        name = name[cut_off_idx..].to_string();
        // keep cutting until it is within the acceptable limit
        continue;
    }

    name
}

/// Reads CLI params, validates them and stores in Config structure.
/// Does process::exit(1) on error + prints error messages
fn read_cli_params(mut config: Config) -> Config {
  // check if there were any arguments passed to override the ENV vars
  let mut args = std::env::args().peekable();
  loop {
      if let Some(arg) = args.next() {
          match arg.to_lowercase().as_str() {
              "--rules" => {
                  let code_rules_dir = Path::new(
                      args.peek()
                          .expect("`--rules` requires a path to the folder with code rules"),
                  )
                  .to_path_buf();

                  // this checks if the rules dir is present, but not its contents
                  // incomplete, may fall over later
                  let code_rules_dir = code_rules_dir
                      .absolutize()
                      .expect("Cannot convert rules dir path to absolute. Check if it looks valid and try to simplify it.")
                      .to_path_buf();

                  if !code_rules_dir.is_dir() {
                      eprintln!(
                          "STACKMUNCHER CONFIG ERROR: Invalid code rules folder `{}`.",
                          code_rules_dir.to_string_lossy()
                      );
                      help::emit_code_rules_msg();
                      exit(1);
                  }

                  // it's a valid path
                  config.code_rules_dir = code_rules_dir;
              }

              "--project" => {
                  let project_dir = Path::new(
                      args.peek()
                          .expect("--project requires a path to the root of the project to be analyzed"),
                  )
                  .absolutize()
                  .expect(
                      "Cannot convert project dir path to absolute path. Check if it looks valid and try to simplify it.",
                  )
                  .to_path_buf();

                  // this only tests the presence of the project dir, not .git inside it
                  // incomplete, may fall over later
                  if !project_dir.exists() {
                      eprintln!(
                          "STACKMUNCHER CONFIG ERROR: Cannot access the project folder at {}",
                          project_dir.to_string_lossy()
                      );
                      help::emit_usage_msg();
                      exit(1);
                  }

                  if !project_dir.is_dir() {
                      eprintln!(
                          "STACKMUNCHER CONFIG ERROR: The path to project folder is not a folder: {}",
                          project_dir.to_string_lossy()
                      );
                      help::emit_usage_msg();
                      exit(1);
                  }

                  // it's a valid path
                  config.project_dir = project_dir;
              }

              "--report" => {
                  let report_dir = 
                      Path::new(
                          args.peek()
                              .expect("--report requires a path to a writable folder for storing StackMuncher reports"),
                      )
                      .absolutize()
                      .expect(
                          "Cannot convert report dir path to absolute path. Check if it looks valid and try to simplify it.",
                      )
                      .to_path_buf();

                  if !report_dir.exists() {
                          eprintln!(
                              "STACKMUNCHER CONFIG ERROR: Cannot access the report folder at {}",
                              report_dir.to_string_lossy()
                          );
                         help::emit_report_dir_msg();
                          exit(1);
                     }

                  if !report_dir.is_dir() {
                          eprintln!(
                              "STACKMUNCHER CONFIG ERROR: The path to report folder is not a folder: {}",
                              report_dir.to_string_lossy()
                          );
                          help::emit_report_dir_msg();
                          exit(1);
                      }
                      
                      // it's a valid path
                  config.report_dir = Some(report_dir);
                  
              }

              "--emails" => {
                let emails = args.peek().expect("--emails requires one or more comma-separated email addresses").to_owned();
                config.git_identities = emails.split(",").filter_map(|email| {
                    let email = email.trim().to_lowercase();
                    if email.is_empty() {None} else {Some(email)}
                }).collect();
              }

              "--log" => {
                  config.log_level =
                      string_to_log_level(args.peek().expect("--log requires a valid logging level").into())
              }
              _ => { //do nothing
              }
          };
      } else {
          break;
      }
  }

  config
}

/// Validates the value for config.code_rules_dir and does process::exit(1) on error.
/// Prints error messages.
fn rules_dir_check(config: &Config)  {
    // this checks if the rules dir is present, but not its contents
    // incomplete, may fall over later
    if !config.code_rules_dir.exists() {
        eprintln!("STACKMUNCHER CONFIG ERROR: Cannot find StackMuncher code parsing rules.");
       help::emit_code_rules_msg();
        exit(1);
    }

    // check if the sub-folders of stm_rules are present
    let file_type_dir = config.code_rules_dir.join(Config::RULES_SUBFOLDER_FILE_TYPES);
    if !file_type_dir.exists() {
        let file_type_dir = file_type_dir
            .absolutize()
            .expect("Cannot convert rules / file_types dir path to absolute. It's a bug.")
            .to_path_buf();

        eprintln!(
            "STACKMUNCHER CONFIG ERROR: Cannot find file type rules folder {}",
            file_type_dir.to_string_lossy()
        );
        help::emit_code_rules_msg();
        std::process::exit(1);
    }

    // check if the munchers sub-folder is present
    let muncher_dir = config.code_rules_dir.join(Config::RULES_SUBFOLDER_MUNCHERS);
    if !muncher_dir.exists() {
        let muncher_dir = muncher_dir
            .absolutize()
            .expect("Cannot convert rules / munchers dir path to absolute. It's a bug.")
            .to_path_buf();

        eprintln!(
            "STACKMUNCHER CONFIG ERROR: Cannot find rules directory for munchers in {}",
            muncher_dir.to_string_lossy()
        );
        help::emit_code_rules_msg();
        std::process::exit(1);
    }
}

/// Validates config.project_dir
fn project_dir_check(config: &Config) {
    // the project dir at this point is either a tested param from the CLI or the current dir
    // a full-trust app is guaranteed access to the current dir
    // a restricted app would need to test if the dir is actually accessible, but it may fail over even earlier when it tried to get the current dir name

    // check if there is .git subfolder in the project dir
    let git_path = config.project_dir.join(".git");
    if !git_path.is_dir() {
        eprintln!(
            "STACKMUNCHER CONFIG ERROR: No Git repository found in the project folder {}",
            config.project_dir.to_string_lossy()
        );
        help::emit_usage_msg();
        exit(1);
    }
}

/// Validates the value for the reports dir and creates one if needed.
/// Prints error messages and exits on error.
fn report_dir_check(mut config: Config) -> Config {
    // individual project reports are grouped in their own folders - build that path here
    // this can be relative or absolute, which should be converted into absolute in a canonical form as a single folder name
    // e.g. /var/tmp/stackmuncher/reports/home_ubuntu_projects_some_project_name_1_6bdf08b3 were the last part is a canonical project name built
    // out of the absolute project path and its own hash
    // the hash is included in the path for ease of search and matching with the report contents because the report itself does not contain any project or user
    // identifiable info
    let project_dir = &config.project_dir;
    let absolute_project_path = if project_dir.is_absolute() {
        project_dir.to_string_lossy().to_string()
    } else {
        // join the current working folder with the relative path to the project
        std::env::current_dir()
            .expect("Cannot get the current dir. It's a bug.")
            .join(project_dir)
            .to_string_lossy()
            .to_string()
    };

    // convert the absolute project path to its canonical name
    let canonical_project_name = Regex::new(r#"\W+"#)
        .expect("Invalid canonical report path regex. It's a bug.")
        .replace_all(&absolute_project_path, "_")
        .trim_matches('_')
        .to_lowercase();

    // append its own hash at the end
    let canonical_project_name_hash = hash_str_sha1(&canonical_project_name)[0..8].to_string();
    let canonical_project_name = [canonical_project_name, canonical_project_name_hash].join("_");
    let canonical_project_name = trim_canonical_project_name(canonical_project_name);

    // append the project report subfolder name to the reports root folder
    let report_dir = config
        .report_dir
        .as_ref()
        .expect("Cannot unwrap the report dir. It's a bug.")
        .join(canonical_project_name);

    // check if the project report folder exists or create it if possible
    if !report_dir.is_dir() {
        if report_dir.exists() {
            // the path exists as something else
            eprintln!(
                "STACKMUNCHER CONFIG ERROR. The path to report directory exists, but it is not a directory: {}",
                report_dir.to_string_lossy()
            );
        }
        // create it
        if let Err(e) = std::fs::create_dir_all(report_dir.clone()) {
            eprintln!(
                "STACKMUNCHER CONFIG ERROR. Cannot create reports directory at {} due to {}",
                report_dir.to_string_lossy(),
                e
            );
        };
        println!("StackMuncher reports folder: {}", report_dir.to_string_lossy());
    }

    // save the project report path in config as String
    config.report_dir = Some(report_dir);

    config
}

/// Checks if there are any contributor identities and informs the user how to configure them. Does not exit or panic.
async fn git_identity_check(mut config: Config)-> Config {

    // ignore the identities in git config if they were provided via CLI args
    if !config.git_identities.is_empty() {
        match config.git_identities.len() {
            1 => println!("Contributor: {}, taken from CLI arg", config.git_identities[0]),
            _ => println!("Contributors: {}, taken from CLI arg", config.git_identities.join(", ")),
        }
        
        return config;
    }

    // get the list of user identities for processing their contributions individually as the default option
    config.git_identities = match get_local_identities(&config.project_dir).await {
        Ok(v) => {

            match v.len() {
                1 => println!("Contributor: {}, taken from Git config", v[0]),
                _ => println!("Contributors: {}, taken from Git config", v.join(", ")),
            }

            v
        },
        Err(_) => {
            eprintln!(
            "StackMuncher analyses individual contributions within a repo. The app needs to know which contributions are yours. You can:\n \
* Configure your local git with `git config --global user.email you@example.com`
* Add one or more emails as `--emails` argument followed by a comma-separated list of all contributor emails. Put your preferred contact email first.\n \
Only the full project report will be generated."
        );
        return config;

        }
    };

    config

}