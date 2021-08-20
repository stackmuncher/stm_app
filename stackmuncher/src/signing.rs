use crate::help;
use bs58;
use path_absolutize::Absolutize;
use ring::{
    rand,
    signature::{self, Ed25519KeyPair, KeyPair},
};
use std::{path::PathBuf, process::exit};
use tracing::{debug, error, info, warn};

/// The core sruct for storing the user pub key and signing payloads.
pub(crate) struct ReportSignature {
    /// Base58-encoded public key from the same key-pair.
    /// E.g. `9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK`
    pub public_key: String,
    /// A base58 encoded signature of the payload
    /// E.g. `21kPtQj3qB6qdimLuBf8aWpnKhD7L6m57N5qpEoUZqYPDn7Ag2DgJFNX5yZXbs3T117fXA66UppanUtVuuhL3uvw`
    pub signature: String,
}

impl ReportSignature {
    /// Retrieves an existing key from the storage or generates a new one, then signs the payload and returns the signature details.
    /// Keys are stored in *reports/.keys/* folder with the norm hash as the file name. There should be only one key per email.
    /// If no keys are present they are generated and saved on disk.
    pub(crate) fn sign(report_as_bytes: &[u8], key_pair: &Ed25519KeyPair) -> Self {
        // the public key is extracted from the key-pair (zero cost op)
        let public_key = key_pair.public_key();

        // sign and encode as base58
        let signature = bs58::encode(key_pair.sign(report_as_bytes).as_ref()).into_string();

        // we need the public key in a string format for sending it in a header
        let public_key = bs58::encode(public_key).into_string();
        debug!("Pub: {}, Sig: {}", public_key, signature);

        Self { public_key, signature }
    }

    /// Retrieves an existing key from the storage or generates a new one, then signs the payload and returns the signature details.
    /// Keys are stored in *reports/.keys/* folder with the norm hash as the file name. There should be only one key per email.
    /// If no keys are present they are generated and saved on disk.
    pub(crate) fn get_public_key(key_pair: &Ed25519KeyPair) -> String {
        // the public key is extracted from the key-pair (zero cost op)
        let public_key = key_pair.public_key();

        bs58::encode(public_key).into_string()
    }
}

/// Retrieves an existing key-pair from the disk or generates a new one and saves it for future use.
/// Panics on unrecoverable errors. May panic over file access or some infra issues generating a key in a particular environment.
pub(crate) fn get_key_pair(keys_dir: &PathBuf) -> Ed25519KeyPair {
    // the validity of the path and the presence of the folder should be validated during config time
    // try to get the file from the disk first
    let key_file_path = get_key_file_name(keys_dir);
    let key_file_path_str = key_file_path
        .absolutize()
        .expect(&format!("Cannot convert {} to absolute path.", key_file_path.to_string_lossy()))
        .to_string_lossy()
        .to_string();

    // does it exist?
    if !key_file_path.exists() {
        // this is a bit wasteful - the call returns the key, but it is read in the next statement from disk
        // did this to keep the flow of the code more or less linear
        info!("No key file found at {}", key_file_path_str);
        generate_and_save_new_pkcs8(&key_file_path);
    }

    // read the contents of the key file
    let pkcs8_bytes = match std::fs::read(key_file_path.clone()) {
        Err(e) => {
            // most likely the file doesn't exist, but it may be corrupt or inaccessible
            warn!("Cannot read key file {} due to {}", key_file_path_str, e);
            generate_and_save_new_pkcs8(&key_file_path)
        }
        Ok(v) => {
            debug!("Key read from: {}", key_file_path_str);
            v
        }
    };

    // decode the bs58-encoded key
    let pkcs8_bytes = match bs58::decode(pkcs8_bytes).into_vec() {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to decode {} from base58 due to {}", key_file_path_str, e);
            help::emit_key_err_msg(&key_file_path_str);
            exit(1);
        }
    };

    // extract the key pair from the contents of the key file
    let key_pair = match signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()) {
        Err(e) => {
            warn!("Invalid key-pair in {} due to {}", key_file_path_str, e);
            help::emit_key_err_msg(&key_file_path_str);
            exit(1);
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

    // convert raw bytes into base58 to make easier to copy between machines
    let contents = bs58::encode(pkcs8.as_ref()).into_vec();

    // try to save it on disk
    if let Err(e) = std::fs::write(key_file_name.clone(), contents) {
        eprintln!(
            "STACKMUNCHER ERROR: failed to save the key file in {}. Reason: {}",
            key_file_name.to_string_lossy(),
            e
        );
        exit(1);
    }

    info!("A new key saved to: {}", key_file_name.to_string_lossy());

    // return the bytes of the key
    pkcs8.as_ref().to_vec()
}

/// Returns the name of the key file for the normalized_email_hash for consistency.
fn get_key_file_name(keys_dir: &PathBuf) -> PathBuf {
    // check if the keys directory exists
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

    // complete building the file name
    keys_dir.join("key.txt")
}
