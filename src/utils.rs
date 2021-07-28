use crypto::{digest::Digest, sha2::Sha256, sha2::Sha512};

pub fn sha512(input: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.input(input);
    hasher.result_str()
}

pub fn sha256(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.input(input);
    hasher.result_str()
}

pub fn print_banner() {
    let banner = "
    ░█▀█░█▀▀░▀█▀░░░▀█▀░█▄█░█▀█░█▀▀░█▀▀░░░█▀█░█▀▄░█▀█░█░█░█░█
    ░█░█░█▀▀░░█░░░░░█░░█░█░█▀█░█░█░█▀▀░░░█▀▀░█▀▄░█░█░▄▀▄░░█░
    ░▀░▀░▀░░░░▀░░░░▀▀▀░▀░▀░▀░▀░▀▀▀░▀▀▀░░░▀░░░▀░▀░▀▀▀░▀░▀░░▀░
    "
    .replace(" ", "");
    println!("{}", banner);
}
