use hocon::HoconLoader;
use lazy_static::lazy_static;
use log::error;
use serde::Deserialize;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::sync::{Arc, RwLock};
use std::thread;

type GenericError = Box<dyn std::error::Error + Send + Sync>;

lazy_static! {
    static ref API_KEYS: Arc<RwLock<KeyService>> = Arc::new(RwLock::new(KeyService::new()));
}

pub enum Status {
    Active,
    Failed,
}

struct KeyService {
    keys: HashSet<String>,
    status: Status,
}

impl From<ApiKeys> for KeyService {
    fn from(k: ApiKeys) -> Self {
        Self {
            keys: HashSet::from_iter(k.api_keys),
            status: Status::Active,
        }
    }
}

impl KeyService {
    fn new() -> Self {
        Self {
            keys: HashSet::new(),
            status: Status::Failed,
        }
    }

    fn validate(&self, key: &str) -> bool {
        match self.status {
            Status::Active => self.keys.contains(key),
            _ => false,
        }
    }

    fn load(path: &str) -> Result<(), GenericError> {
        match API_KEYS.write() {
            Ok(mut guard) => {
                let keys = ApiKeys::fetch_from_file(path);
                match keys {
                    Ok(k) => {
                        *guard = k.into();
                        drop(guard);
                        Ok(())
                    }
                    Err(e) => {
                        *guard = KeyService::new();
                        drop(guard);
                        Err(format!(
                            "key service has failed , could not update keys from file due to an error: {} ",
                            e
                        )
                        .into())
                    }
                }
            }
            Err(_e) => {
                Err("API_KEYS is poisoned, an error was encountered during key update".into())
            }
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct ApiKeys {
    pub api_keys: Vec<String>,
}

impl ApiKeys {
    fn fetch_from_file(path: &str) -> Result<ApiKeys, GenericError> {
        let keys: ApiKeys = HoconLoader::new().load_file(path)?.resolve()?;
        Ok(keys)
    }
}





pub fn validate(key: &str) -> bool {
    let guard = API_KEYS.read();
    match guard {
        Ok(keys) => {
            keys.validate(key)
        }
        Err(_e) => {
            error!("API_KEYS is poisoned, an error was possibly encountered during key update");
            false
        }
    }
}

pub fn start_service(key_file_path: &'static str, refresh_in_seconds: u64) {
    thread::spawn(move || loop {
        match KeyService::load(key_file_path) {
            Ok(_) => {}
            Err(e) => {
                error!("error : {} , was encountered during key update", e);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(refresh_in_seconds));
    });
}


