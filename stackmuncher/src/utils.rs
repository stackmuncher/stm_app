use sha1::{Digest, Sha1};

/// Returns a string representation of a hash hex using SHA1.
/// E.g. `6bdf08b30f8cc1173729d8559933bea5c024c25`
pub fn hash_str_sha1(string: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(string);
    format!("{:x}", hasher.finalize())
}
