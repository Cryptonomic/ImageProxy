use sha2::{Digest, Sha256, Sha512};

pub fn sha512(input: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

pub fn sha256(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

pub fn print_banner() {
    let banner = "
    ░█▀█░█▀▀░▀█▀░░░▀█▀░█▄█░█▀█░█▀▀░█▀▀░░░█▀█░█▀▄░█▀█░█░█░█░█
    ░█░█░█▀▀░░█░░░░░█░░█░█░█▀█░█░█░█▀▀░░░█▀▀░█▀▄░█░█░▄▀▄░░█░
    ░▀░▀░▀░░░░▀░░░░▀▀▀░▀░▀░▀░▀░▀▀▀░▀▀▀░░░▀░░░▀░▀░▀▀▀░▀░▀░░▀░
    "
    .replace(' ', "");
    println!("{}", banner);
}
