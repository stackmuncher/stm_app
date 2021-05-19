use crate::signing::ReportSignature;
use stackmuncher_lib::config::Config;

/// Submits the serialized report to STM or some other web service. Includes signing.
/// May panic if the signing fails (missing keys, can't access keystore).
pub(crate) async fn submit_report(email: &String, payload: Vec<u8>, config: &Config) {
    let report_sig = ReportSignature::sign(email, &payload, config);

    println!("Signature: {}", report_sig.signature);
}
