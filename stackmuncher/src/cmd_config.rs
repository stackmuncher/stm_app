use crate::config::AppConfig;
use crate::help;
use crate::signing::ReportSignature;
use hyper::{Client, Request};
use hyper_rustls::HttpsConnector;
use path_absolutize::{self, Absolutize};
use ring::signature::{self, Ed25519KeyPair, KeyPair};
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, error, info, warn};

/// A "well-known" string used as the content to be signed for GH verification. The signature is uploaded to a Gist.
const GH_VERIFICATION_STRING_TO_SIGN: &str = "stackmuncher";

/// A stripped-down representation of GH GetGist API response: Owner details.
#[derive(Deserialize)]
pub(crate) struct GistOwner {
    /// GitHub login of the user, e.g. `rimutaka`.
    login: Option<String>,
}

/// A rough top-level representation of GH GetGist API response.
#[derive(Deserialize)]
pub(crate) struct RawGist {
    /// The file name is used as the property name, so it is easier to just get Value and then manually dig into it.
    /// We only need the contents.
    /// ```json
    /// "files": {
    ///   "stm.txt": {
    ///     "content": "MDQ6R2lzdGZiOGZjMGY4N2VlNzgyMzFmMDY0MTMxMDIyYzgxNTRh"
    ///   }
    /// }
    /// ```
    pub files: Option<Value>,
    pub owner: Option<GistOwner>,
}

/// A validated Gist structure, same as Gist, but without Option<>
pub(crate) struct Gist {
    /// GitHub login of the Gist owner, e.g. `rimutaka`
    pub login: String,
    /// The UI URL of the gist, e.g. https://gist.github.com/fb8fc0f87ee78231f064131022c8154a
    pub html_url: String,
}

pub(crate) async fn github(config: AppConfig) {
    // user signature expected in the gist
    let expected_gist_content = generate_gist_content(&config.user_key_pair);

    help::emit_gist_instructions(&expected_gist_content);
}

/// Prints its full current configuration, file locations, profile URL and some usage info.
pub(crate) async fn view_config(config: AppConfig) {
    // prepare values needed in println!() macros to prevent line wrapping in the code
    let pub_key = ReportSignature::get_public_key(&config.user_key_pair);
    let reports = config
        .lib_config
        .report_dir
        .as_ref()
        .expect("config.report_dir is not set. It's a bug.")
        .absolutize()
        .expect("Cannot convert config.report_dir to absolute path. It's a bug.")
        .to_string_lossy()
        .to_string();
    let config_file = config
        .config_file_path
        .absolutize()
        .expect("Cannot convert config.config_file_path to absolute path. It's a bug.")
        .to_string_lossy()
        .to_string();
    let exe_file = match std::env::current_exe() {
        Ok(v) => v.to_string_lossy().to_string(),
        Err(_) => "unknown".to_string(),
    };

    // gh_validation_gist may already be in the config if --gist option was used and it was validated
    // otherwise we need to re-validate it and get the details from github
    let gh_validation_gist = match config.gh_validation_gist {
        Some(v) => Some(v),
        None => get_validated_gist(&config.gh_validation_id, &config.user_key_pair).await,
    };

    // prepare user-friendly GH validation messages
    let (public_profile, github_validation) = match gh_validation_gist {
        Some(v) => (["https://stackmuncher.com/", &v.login].concat(), v.html_url),
        None => ("disabled".to_owned(), "not set".to_owned()),
    };

    println!();
    println!("    Primary email: {}", config.primary_email.as_ref().unwrap_or(&"not set".to_owned()));
    println!("    Commit emails: {}", config.lib_config.git_identities.join(", "));
    println!();
    println!("    Anonymous profile: https://stackmuncher.com/?dev={}", pub_key);
    println!("    Public profile:    {}", public_profile);
    println!("    GitHub validation: {}", github_validation);
    println!();
    println!("    Stack reports: {}", reports);
    println!("    Config folder: {}", config_file);
    println!("    Executable:    {}", exe_file);
    println!();
}

/// Returns gist details, if any for the given Gist ID. Can be tested with this shell command:
/// ```shell
/// curl \
///  -H "Accept: application/vnd.github.v3+json" \
///  https://api.github.com/gists/GIST_ID
/// ```
pub(crate) async fn get_validated_gist(gist_id: &Option<String>, user_key_pair: &Ed25519KeyPair) -> Option<Gist> {
    // check if the gist needs to be retrieved at all
    let gist_id = match gist_id.as_ref() {
        Some(v) => v,

        None => {
            return None;
        }
    };

    // the user requested to unlink from GitHub
    if gist_id.is_empty() {
        return None;
    }

    info!("Getting GitHub validation from Gist #{}", gist_id);

    let uri = ["https://api.github.com/gists/", &gist_id].concat();

    // prepare the HTTP request to GitHub API
    let req = Request::builder()
        .uri(uri.clone())
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "StackMuncher App")
        .method("GET")
        .body(hyper::Body::empty())
        .expect("Cannot create Gist API request");
    debug!("Http rq: {:?}", req);

    // send it out, but it may fail for any number of reasons and we still have to carry on
    let res = match Client::builder()
        .build::<_, hyper::Body>(HttpsConnector::with_native_roots())
        .request(req)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("GitHub API request to {} failed with {}", uri, e);
            help::emit_gist_troubleshooting(&gist_id, &uri);
            return None;
        }
    };

    let status = res.status();
    debug!("GH API response status: {}", status);

    // Concatenate the body stream into a single buffer...
    let buf = hyper::body::to_bytes(res)
        .await
        .expect("Cannot convert GH API response body to bytes. It's a bug.");

    // there should be at least some data returned
    if buf.is_empty() {
        error!("Empty GH API response with status {}", status);
        help::emit_gist_troubleshooting(&gist_id, &uri);
        return None;
    }

    // any status other than 200 is an error
    if !status.is_success() {
        error!("Status {}", status);
        log_http_body(&buf);
        help::emit_gist_troubleshooting(&gist_id, &uri);
        return None;
    }

    // all responses should be JSON. If it's not JSON it's an error.
    let gist = match serde_json::from_slice::<RawGist>(&buf) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to convert GH API response to JSON with {}", e);
            log_http_body(&buf);
            help::emit_gist_troubleshooting(&gist_id, &uri);
            return None;
        }
    };
    info!("GH API response arrived");

    // check that all the data we need is in there
    let github_login = match gist.owner {
        Some(v) => match v.login {
            Some(v) => v,
            None => {
                error!("Invalid GH API response: missing `owner/login` JSON property");
                log_http_body(&buf);
                help::emit_gist_troubleshooting(&gist_id, &uri);
                return None;
            }
        },
        None => {
            error!("Invalid GH API response: missing `owner` JSON property");
            log_http_body(&buf);
            help::emit_gist_troubleshooting(&gist_id, &uri);
            return None;
        }
    };

    // are there any GIST contents at all?
    // expecting something like
    // "files": {"stm.txt": { "content": "MDQ6R2lzdGZiOGZjMGY4N2VlNzgyMzFmMDY0MTMxMDIyYzgxNTRh" } }
    let gist_contents = match gist.files {
        None => {
            error!("Invalid GH API response: missing `file` JSON property");
            log_http_body(&buf);
            help::emit_gist_troubleshooting(&gist_id, &uri);
            return None;
        }
        Some(v) => v,
    };

    // there should be just one property inside "files", but we don't know its name because it is the file name
    // which can be anything
    // the insanely deep check is to make the code more readable - not very efficient, but it's OK, only run once in a while
    if !gist_contents.is_object()
        || gist_contents.as_object().is_none()
        || gist_contents.as_object().unwrap().len() != 1
        || gist_contents.as_object().unwrap().iter().next().is_none()
        || !gist_contents.as_object().unwrap().iter().next().unwrap().1.is_object()
        || gist_contents
            .as_object()
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .1
            .get("content")
            .is_none()
        || !gist_contents
            .as_object()
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .1
            .get("content")
            .unwrap()
            .is_string()
    {
        error!("Invalid GH API response: invalid `file` JSON property");
        log_http_body(&buf);
        help::emit_gist_troubleshooting(&gist_id, &uri);
        return None;
    }

    // this is the actual file, so the property name is "stm.txt" in our example and we can try getting "content"
    let gist_contents = gist_contents
        .as_object()
        .unwrap()
        .iter()
        .next()
        .unwrap()
        .1
        .get("content")
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned();

    // remove possible wrappers and white space around it
    let gist_contents = gist_contents
        .replace("\"", "")
        .replace("'", "")
        .replace("`", "")
        .trim()
        .to_string();

    // We need "https://gist.github.com/rimutaka/fb8fc0f87ee78231f064131022c8154a" with the login in it.
    // The API response doesn't have a URL like that, so it needs to be constructed from parts.
    let gist_html_url = ["https://gist.github.com/", &github_login, "/", &gist_id].concat();

    // user signature expected in the gist
    let expected_gist_content = generate_gist_content(&user_key_pair);

    // convert the signature from base58 into bytes
    let signature = match bs58::decode(gist_contents.clone()).into_vec() {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to decode the contents of the Gist from based58 due to: {}", e,);
            error!("Expected this Gist content: {}", expected_gist_content);
            error!("Found something different: {}", gist_contents);
            help::emit_gist_instructions(&expected_gist_content);
            return None;
        }
    };

    // check if the signature in the gist is valid
    let pub_key = signature::UnparsedPublicKey::new(&signature::ED25519, user_key_pair.public_key().as_ref());
    match pub_key.verify(GH_VERIFICATION_STRING_TO_SIGN.as_bytes(), &signature) {
        Ok(_) => {
            info!("Signature OK");
        }
        Err(_) => {
            error!("Invalid signature in Gist: {}", gist_contents);
            error!("Gist owner: {}. Is it your login?", github_login);
            help::emit_gist_instructions(&expected_gist_content);
            return None;
        }
    };

    Some(Gist {
        login: github_login,
        html_url: gist_html_url,
    })
}

/// Logs the body as error!(), if possible.
pub(crate) fn log_http_body(body_bytes: &hyper::body::Bytes) {
    if body_bytes.is_empty() {
        warn!("Empty response body.");
        return;
    }

    // log the body as-is if it's not too long
    if body_bytes.len() < 3000 {
        let s = match std::str::from_utf8(&body_bytes).to_owned() {
            Err(_e) => "The body is not UTF-8".to_string(),
            Ok(v) => v.to_string(),
        };
        info!("Response body: {}", s);
    } else {
        info!("Response is too long to log: {}B", body_bytes.len());
    }
}

/// Signs a "well-known" string with the user's key-pair to produce a unique signature.
fn generate_gist_content(user_key_pair: &Ed25519KeyPair) -> String {
    bs58::encode(user_key_pair.sign(GH_VERIFICATION_STRING_TO_SIGN.as_bytes()).as_ref()).into_string()
}
