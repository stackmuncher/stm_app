use crate::help;
use crate::signing::ReportSignature;
use crate::AppConfig;
use chrono;
use flate2::write::GzEncoder;
use flate2::Compression;
use hyper::{Client, Request};
use hyper_rustls::HttpsConnector;
use stackmuncher_lib::utils::sha256::hash_str_to_sha256_as_base58;
use stackmuncher_lib::{report::Report, tech::Tech};
use std::io::prelude::*;
use tracing::{debug, info, warn};

//const STM_REPORT_SUBMISSION_URL: &str = "https://emvu2i81ec.execute-api.us-east-1.amazonaws.com";
const STM_REPORT_SUBMISSION_URL: &str = "https://inbox.stackmuncher.com";
const HEADER_USER_PUB_KEY: &str = "stackmuncher_key";
const HEADER_USER_SIGNATURE: &str = "stackmuncher_sig";

// these constants are used to compare the latest available version with what is run locally
// const CLIENT_VERSION: &'static str = env!("CARGO_PKG_VERSION");
// #[cfg(target_os = "linux")]
// const CLIENT_PLATFORM: &str = "linux";
// #[cfg(target_os = "windows")]
// const CLIENT_PLATFORM: &str = "windows";

/// Submits the serialized report to STM or some other web service. Includes signing.
/// May panic if the signing fails (missing keys, can't access keystore).
pub(crate) async fn submit_report(report: Report, config: &AppConfig) {
    // remove any sensitive info from the report and gzip it
    let report = match pre_submission_cleanup(report) {
        Err(_) => {
            return;
        }
        Ok(v) => v,
    };

    // sign the report
    let report_sig = ReportSignature::sign(&report, &config.user_key_pair);

    // prepare HTTP request which should go without a hitch unless the report or one of the headers is somehow invalid
    let req = Request::builder()
        .method("POST")
        .uri(STM_REPORT_SUBMISSION_URL)
        .header(HEADER_USER_PUB_KEY, report_sig.public_key.clone())
        .header(HEADER_USER_SIGNATURE, report_sig.signature.clone())
        .body(hyper::Body::from(report))
        .expect("Invalid report submission payload. It's a bug.");

    debug!("Http rq: {:?}", req);

    // send out the request
    info!("Sending request to INBOX");
    let res = match Client::builder()
        .build::<_, hyper::Body>(HttpsConnector::with_native_roots())
        .request(req)
        .await
    {
        Err(e) => {
            warn!("StackMuncher report submission failed due to: {}.", e);
            eprintln!("Sending the stack report to stackmuncher.com failed. It may go through with the next commit.");
            help::emit_detailed_output_msg();
            return;
        }
        Ok(v) => v,
    };

    let status = res.status();
    info!("stm_inbox response arrived. Status: {}", status);

    // Concatenate the body stream into a single buffer...
    let buf = match hyper::body::to_bytes(res).await {
        Err(e) => {
            warn!("Failed to convert StackMuncher report to bytes due to: {}. It's a bug", e);
            eprintln!("Failed to convert StackMuncher report to bytes due to: {}. It's a bug", e);
            help::emit_detailed_output_msg();
            return;
        }
        Ok(v) => v,
    };

    // a 200 OK body can be empty if everything is OK
    if status.as_u16() == 200 && buf.is_empty() {
        debug!("Empty response body, 200 OK");
        if config.lib_config.log_level == tracing::Level::ERROR {
            println!("Directory profile updated.");
        }
        return;
    }

    if !buf.is_empty() {
        log_http_body(&buf);
    }
}

/// Logs the body as warn!() and prints out for the user, if possible.
fn log_http_body(body_bytes: &hyper::body::Bytes) {
    // log the body as-is if it's not too long
    if body_bytes.len() < 5000 {
        let s = match std::str::from_utf8(&body_bytes).to_owned() {
            Err(_e) => "The body is not UTF-8".to_string(),
            Ok(v) => v.to_string(),
        };
        warn!("StackMuncher server response: {}", s);
        eprintln!("{}", s);
    } else {
        warn!(
            "StackMuncher server response is too long to log: {}B. Something's broken at their end.",
            body_bytes.len()
        );
    }
}

/// Removes or replaces any sensitive info from the report for submission to stackmuncher.com.
/// Gzips the sanitized report and returns the raw bytes.
/// Returns a sanitized report as bytes ready to be sent out
pub(crate) fn pre_submission_cleanup(report: Report) -> Result<Vec<u8>, ()> {
    // this function should be replaced with a macro
    // see https://github.com/stackmuncher/stm/issues/12

    info!("Report pre-submission cleanup started");
    // expensive, but probably unavoidable given that the original report will still be used at the point of call
    let mut report = report.clone();

    // clean up per_file_tech section
    let per_file_tech = report.per_file_tech.drain().collect::<Vec<Tech>>();
    for mut x in per_file_tech {
        x.file_name = Some(hash_str_to_sha256_as_base58(&x.file_name.unwrap_or_default()));
        x.keywords.clear();
        x.pkgs.clear();
        x.pkgs_kw = None;
        x.refs.clear();
        x.refs_kw = None;
        report.per_file_tech.insert(x);
    }

    // this may be an email address of someone else
    report.last_commit_author = None;
    // someone's else commit hash can be used for matching across devs
    report.report_commit_sha1 = None;

    // reset time component of the project head and init commit timestamps to prevent cross-developer project matching
    if let Some(date_head) = &report.date_head {
        match chrono::DateTime::parse_from_rfc3339(date_head) {
            Err(e) => {
                warn!("Invalid HEAD commit date: {} ({}). Expected RFC3339 format.", date_head, e);
                report.date_head = None;
            }
            Ok(v) => {
                report.date_head = Some(v.date().and_hms(0, 0, 0).to_rfc3339());
            }
        }
    }

    if let Some(date_init) = &report.date_init {
        match chrono::DateTime::parse_from_rfc3339(date_init) {
            Err(e) => {
                warn!("Invalid INIT commit date: {} ({}). Expected RFC3339 format.", date_init, e);
                report.date_init = None;
            }
            Ok(v) => {
                report.date_init = Some(v.date().and_hms(0, 0, 0).to_rfc3339());
            }
        }
    }

    // serialize the report into bytes
    let report = match serde_json::to_vec(&report) {
        Err(e) => {
            eprintln!("Cannot serialize a report after pre-sub cleanup due to {}", e);
            return Err(());
        }
        Ok(v) => v,
    };

    // gzip it
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    if let Err(e) = encoder.write_all(&report) {
        eprintln!("Cannot gzip the report due to {}", e);
        return Err(());
    };
    let gzip_bytes = match encoder.finish() {
        Err(e) => {
            eprintln!("Cannot finish gzipping the report due to {}", e);
            return Err(());
        }

        Ok(v) => v,
    };

    info!("Report size: {}, GZip: {}", report.len(), gzip_bytes.len());

    Ok(gzip_bytes)
}
