use hmac::{Hmac, Mac, NewMac};
use sha2::Sha256;

pub fn sign(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let hmac = Hmac::<Sha256>::new_from_slice(key);
    if let Ok(mut hmac) = hmac {
        hmac.update(msg);
        let result = hmac.finalize();
        result.into_bytes().to_vec()
    } else {
        Vec::new()
    }
}

pub fn get_signature_key(key: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let kdate = sign(format!("AWS4{}", key).as_bytes(), date_stamp.as_bytes());
    let kregion = sign(&kdate, region.as_bytes());
    let kservice = sign(&kregion, service.as_bytes());
    let signing_key = sign(&kservice, "aws4_request".as_bytes());
    signing_key.to_vec()
}
