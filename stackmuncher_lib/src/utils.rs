use sha1::{Digest, Sha1};

/// Returns a string representation of a hash hex using SHA1.
/// E.g. `6bdf08b30f8cc1173729d8559933bea5c024c25`
pub fn hash_str_sha1(string: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(string);
    format!("{:x}", hasher.finalize())
}

/// Returns a string representation of a hash hex using SHA1.
/// E.g. `6bdf08b30f8cc1173729d8559933bea5c024c25`
pub fn hash_vec_sha1(vec_of_strings: Vec<String>) -> String {
    let mut hasher = Sha1::new();

    for string in vec_of_strings {
        hasher.update(string);
    }

    format!("{:x}", hasher.finalize())
}

// The mod was created to avoid having Digest twice, for SHA1 and SHA2.
// It compiles with just one Digest, but the implications are unknown.

pub mod sha256 {
    use bs58;
    use sha2::{Digest, Sha256};

    /// Returns a string representation of a hash hex using SHA256 encoded .
    /// E.g. `3xMKTSi8KZiJGG7vqGSaFS7hC9B2EAMDHv7Yp3CSr5LQ`
    pub fn hash_str_to_sha256_as_base58(string: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(string);

        bs58::encode(hasher.finalize().as_slice()).into_string()
    }
}
