use bs58;
use ring::{
    rand,
    signature::{self, Ed25519KeyPair, KeyPair},
};
use stackmuncher_lib::config::Config;
use stackmuncher_lib::utils::sha256::hash_str_to_sha256_as_base58;
use std::{path::PathBuf, process::exit};
use tracing::{info, warn};

/// The core sruct for storing the user pub key and signing payloads.
pub(crate) struct ReportSignature {
    /// The contributor email the report belongs to. The key is selected / generated based on this field.
    /// The value is normalized for hashing (lowercase, surrounding whitespace is removed).
    /// The email is not validated, so technically this can be any random string of characters.
    pub normalized_email: String,
    /// An SH256 hash of the email field in Base58 format.
    pub normalized_email_hash: String,
    /// Base58-encoded public key from the same key-pair.
    pub public_key: String,
    /// A base58 encoded signature of the payload
    /// E.g. `21kPtQj3qB6qdimLuBf8aWpnKhD7L6m57N5qpEoUZqYPDn7Ag2DgJFNX5yZXbs3T117fXA66UppanUtVuuhL3uvw`
    pub signature: String,
}

impl ReportSignature {
    /// Retrieves an existing key from the storage or generates a new one, then signs the payload and returns the signature details.
    /// Keys are stored in *reports/.keys/* folder with the norm hash as the file name. There should be only one key per email.
    /// If no keys are present they are generated and saved on disk.
    pub(crate) fn sign(email: &String, payload: &[u8], config: &Config) -> Self {
        // normalize the email
        let normalized_email = email.to_lowercase().trim().to_string();
        // the hash looks like 3xMKTSi8KZiJGG7vqGSaFS7hC9B2EAMDHv7Yp3CSr5LQ
        let normalized_email_hash = hash_str_to_sha256_as_base58(&email);
        info!(
            "Report signing. Norm email: {}, hash: {}",
            normalized_email, normalized_email_hash
        );

        // get a new or previously generated and stored locally key-pair
        let key_pair = get_key_pair(&normalized_email_hash, &config);
        // the public key is extracted from the key-pair (zero cost op)
        let public_key = bs58::encode(key_pair.public_key()).into_string();
        // sign and encode as base58
        let signature = bs58::encode(key_pair.sign(payload).as_ref()).into_string();

        info!("Pub: {}, Sig: {}", public_key, signature);

        Self {
            normalized_email,
            normalized_email_hash,
            public_key,
            signature,
        }
    }
}

/// Retrieves an existing key-pair from the disk or generates a new one and saves it for future use.
/// Panics on unrecoverable errors. May panic over file access or some infra issues generating a key in a particular environment.
fn get_key_pair(normalized_email_hash: &String, config: &Config) -> Ed25519KeyPair {
    // try to get the file from the disk first
    let key_file_path = get_key_file_name(normalized_email_hash, config);
    let pkcs8_bytes = match std::fs::read(key_file_path.clone()) {
        Err(e) => {
            // most likely the file doesn't exist, but it may be corrupt or inaccessible
            warn!("Cannot read key file {} due to {}", key_file_path.to_string_lossy(), e);
            generate_and_save_new_pkcs8(&key_file_path)
        }
        Ok(v) => {
            info!("Key read from: {}", key_file_path.to_string_lossy());
            v
        }
    };

    // extract the key pair from the contents of the key file
    let key_pair = match signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()) {
        Err(e) => {
            warn!("Failed to generate an ED25519 key pair from pkcs8 bytes due to {}", e);
            // try again - if the file is corrupt it may be easier to regenerate it
            match signature::Ed25519KeyPair::from_pkcs8(generate_and_save_new_pkcs8(&key_file_path).as_ref()) {
                Err(e) => {
                    // there is not much else can be done
                    warn!(
                        "Failed to generate an ED25519 key pair (attempt 2) from pkcs8 bytes due to {}",
                        e
                    );
                    eprintln!("STACKMUNCHER ERROR: failed to generate an ED25519 key pair");
                    exit(1);
                }
                Ok(v) => v,
            }
        }
        Ok(v) => v,
    };

    key_pair
}

/// Generates a new PKCS8 file and saves it in a common location with the hash as its name for future retrieval.
/// Panics on unrecoverable errors.
fn generate_and_save_new_pkcs8(key_file_name: &PathBuf) -> Vec<u8> {
    // Generate a key pair in PKCS#8 (v2) format.
    let rng = rand::SystemRandom::new();
    let pkcs8 = match signature::Ed25519KeyPair::generate_pkcs8(&rng) {
        Err(_) => {
            eprintln!("STACKMUNCHER ERROR: failed to generate PKCS8 key");
            exit(1);
        }
        Ok(v) => v,
    };

    // try to save it on disk
    if let Err(e) = std::fs::write(key_file_name.clone(), pkcs8.as_ref()) {
        eprintln!(
            "STACKMUNCHER ERROR: failed to save the key file in {}. Reason: {}",
            key_file_name.to_string_lossy(),
            e
        );
        exit(1);
    }

    info!("Key saved to: {}", key_file_name.to_string_lossy());

    // return the bytes of the key
    pkcs8.as_ref().to_vec()
}

/// Returns the name of the key file for the normalized_email_hash for consistency.
fn get_key_file_name(normalized_email_hash: &String, config: &Config) -> PathBuf {
    // check if the keys directory exists
    let keys_dir = config
        .report_dir
        .as_ref()
        .expect("Cannot unwrap config.report_dir. It's a bug.")
        .parent()
        .expect("Cannot unwrap the root folder for all reports. It's a bug.")
        .to_path_buf()
        .join(".keys");

    if !keys_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(keys_dir.clone()) {
            eprintln!(
                "STACKMUNCHER ERROR: failed to create a new directory for key files in {}. Reason: {}",
                keys_dir.to_string_lossy(),
                e
            );
            exit(1);
        };
        info!("Created keys folder in {}", keys_dir.to_string_lossy());
    }

    // complete bilding the file name
    keys_dir.join(normalized_email_hash).with_extension("pki")
}
