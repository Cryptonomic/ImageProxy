use crypto::{digest::Digest, sha2::Sha512};

pub fn sha512(input: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.input(input);
    hasher.result_str()
}
