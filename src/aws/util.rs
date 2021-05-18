use crypto::digest::Digest;
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha2::Sha256;

pub fn sign(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut hmac = Hmac::new(Sha256::new(), key);
    hmac.input(msg);
    let res = hmac.result();
    res.code().to_vec()
}

pub fn get_signature_key(key: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let kdate = sign(&format!("AWS4{}", key).as_bytes(), &date_stamp.as_bytes());
    let kregion = sign(&kdate, &region.as_bytes());
    let kservice = sign(&kregion, &service.as_bytes());
    let signing_key = sign(&kservice, &"aws4_request".as_bytes());
    signing_key.to_vec()
}

pub fn hash(to_hash: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.input_str(to_hash);
    return hasher.result_str();
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_hash() {
        let input = "8074CF820F831F385AB5D1D3".to_string();
        assert_eq!(
            hash(&input),
            "cc441b9bbff910a67b7519f8ca51bcb3433ccc2fe4de58f511c4a2580a9095ec"
        );
    }
}
