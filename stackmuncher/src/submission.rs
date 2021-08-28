use crate::help;
use crate::signing::ReportSignature;
use crate::AppConfig;
use hyper::{Client, Request};
use hyper_rustls::HttpsConnector;
use stackmuncher_lib::report::Report;
use tracing::{debug, info, warn};

//const STM_REPORT_SUBMISSION_URL: &str = "https://emvu2i81ec.execute-api.us-east-1.amazonaws.com";
const STM_REPORT_SUBMISSION_URL: &str = "https://inbox.stackmuncher.com";
const HEADER_USER_PUB_KEY: &str = "stackmuncher_key";
const HEADER_USER_SIGNATURE: &str = "stackmuncher_sig";

/// Submits the serialized report to STM or some other web service. Includes signing.
/// May panic if the signing fails (missing keys, can't access keystore).
pub(crate) async fn submit_report(report: Report, config: &AppConfig) {
    // compress the report
    let report = match report.gzip() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("STACKMUNCHER: no report was submitted.");
            return;
        }
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
    info!("Sending request to INBOX for {}", report_sig.public_key.clone());
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
    info!("stm_inbox response arrived, status: {}", status,);

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

        // public profile is preferred, but not be enabled
        if let Some(gh_login) = &config.gh_login {
            println!("    Project added to:    https://stackmuncher.com/{}", gh_login);
        } else {
            println!("    Project added to:    https://stackmuncher.com/?dev={}", report_sig.public_key);
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
