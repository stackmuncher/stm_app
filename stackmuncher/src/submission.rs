use crate::help;
use crate::signing::ReportSignature;
use hyper::{Client, Request};
use hyper_rustls::HttpsConnector;
use stackmuncher_lib::config::Config;
use tracing::{debug, info, warn};

const STM_REPORT_SUBMISSION_URL: &str = "https://emvu2i81ec.execute-api.us-east-1.amazonaws.com";
const HEADER_USER_ID: &str = "stackmuncher_id";
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
pub(crate) async fn submit_report(email: &String, report_as_bytes: Vec<u8>, config: &Config) {
    // sign the report
    let report_sig = ReportSignature::sign(email, &report_as_bytes, config);

    // prepare HTTP request which should go without a hitch unless the report or one of the headers is somehow invalid
    let req = Request::builder()
        .method("POST")
        .uri(STM_REPORT_SUBMISSION_URL)
        .header(HEADER_USER_ID, report_sig.normalized_email)
        .header(HEADER_USER_PUB_KEY, report_sig.public_key.clone())
        .header(HEADER_USER_SIGNATURE, report_sig.signature.clone())
        .body(hyper::Body::from(report_as_bytes))
        .expect("Invalid report submission payload. It's a bug.");

    debug!("Http rq: {:?}", req);

    // send out the request
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
    info!("STM response arrived. Status: {}", status);

    // Concatenate the body stream into a single buffer...
    let buf = match hyper::body::to_bytes(res).await {
        Err(e) => {
            warn!(
                "Failed to convert StackMuncher report to bytes due to: {}. It's a bug",
                e
            );
            eprintln!(
                "Failed to convert StackMuncher report to bytes due to: {}. It's a bug",
                e
            );
            help::emit_detailed_output_msg();
            return;
        }
        Ok(v) => v,
    };

    // a 200 OK body can be empty if everything is OK
    if status.as_u16() == 200 && buf.is_empty() {
        info!("Empty response body, 200 OK");
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
        info!(
            "StackMuncher server response is too long to log: {}B. Something's broken at their end.",
            body_bytes.len()
        );
    }
}
