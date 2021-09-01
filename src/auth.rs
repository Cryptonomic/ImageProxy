use hocon::{Error, HoconLoader};
use hyper::{Body, Request};
use log::{error, info};
use serde::Deserialize;
use std::thread;
use std::time::Duration;
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

type GenericError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize, Clone)]
pub struct ApiKeys {
    pub api_keys: Vec<String>,
}

impl ApiKeys {
    pub fn load() -> Result<ApiKeys, Error> {
        let keys: ApiKeys = HoconLoader::new().load_file("keys")?.resolve()?;
        Ok(keys)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Status {
    Active,
    Failed,
    Exited,
}
pub struct ApiKeysService {
    keys: Arc<Mutex<ApiKeys>>,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    status: Arc<Mutex<Status>>,
}

impl ApiKeysService {
    pub fn new(refresh_secs: u64) -> Result<ApiKeysService, GenericError> {
        let apikeys = ApiKeys::load()?;
        let key_service = ApiKeysService {
            keys: Arc::new(Mutex::new(apikeys)),
            handle: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(Status::Active)),
        };
        key_service.load_service(refresh_secs)?;
        Ok(key_service)
    }

    pub fn get_status(&self) -> Result<Status, GenericError> {
        let status = self.status.clone();
        let guard = status
            .lock()
            .map_err(|_| "Failed to acquire lock for status mutex")?;
        let stat = *guard;
        drop(guard);
        Ok(stat)
    }

    pub fn stop_service(&self) -> Result<(), GenericError> {
        ApiKeysService::set_status(self.status.clone(), Status::Exited)?;
        Ok(())
    }

    fn read_status(status: Arc<Mutex<Status>>) -> Result<Status, GenericError> {
        let guard = status
            .lock()
            .map_err(|_| "Failed to acquire lock for status mutex")?;
        let stat = *guard;
        drop(guard);
        Ok(stat)
    }

    fn set_status(status: Arc<Mutex<Status>>, set_to: Status) -> Result<(), GenericError> {
        let mut guard = status
            .lock()
            .map_err(|_| "Failed to acquire lock for status mutex")?;
        *guard = set_to;
        drop(guard);
        Ok(())
    }

    fn reload(keys: Arc<Mutex<ApiKeys>>) -> Result<(), GenericError> {
        let api_keys_from_file = ApiKeys::load()?;
        let mut guard = keys
            .lock()
            .map_err(|_| "Failed to acquire lock for api keys mutex")?;
        *guard = api_keys_from_file;
        drop(guard);
        Ok(())
    }

    fn start_service(
        &self,
        keys: Arc<Mutex<ApiKeys>>,
        status: Arc<Mutex<Status>>,
        refresh_secs: u64,
    ) -> JoinHandle<()> {
        thread::spawn(move || loop {
            match ApiKeysService::read_status(status.clone()) {
                Ok(Status::Active) => match ApiKeysService::reload(keys.clone()) {
                    Ok(_) => {}
                    Err(e) => match ApiKeysService::set_status(status.clone(), Status::Failed) {
                        Ok(_) => {
                            error!("Api Key Service: Failed to update keys from file, {}", e);
                        }
                        Err(e1) => {
                            error!("Api Key Service: Failed to update keys from file {} and failed to change status to failed {}",e,e1);
                        }
                    },
                },
                Ok(Status::Failed) => match ApiKeysService::reload(keys.clone()) {
                    Ok(_) => match ApiKeysService::set_status(status.clone(), Status::Active) {
                        Ok(_) => {
                            info!("Api Key Service: Failed Api Key Service succefully restarted");
                        }
                        Err(e) => {
                            error!("Api Key Service: updated keys from file but failed to change status to Active {}",e);
                        }
                    },
                    Err(_e) => {
                        // TODO: add a failure strategy that exits after x number of failed attempts 
                    }
                },
                Ok(Status::Exited) => {
                    break;
                }
                Err(e) => {
                    error!("Api Key Service: failed to read status {}", e);
                }
            }
            thread::sleep(Duration::from_secs(refresh_secs));
        })
    }

    fn load_service(&self, refresh_secs: u64) -> Result<(), GenericError> {
        let handle = Arc::clone(&self.handle);
        let new_handle = self.start_service(self.keys.clone(), self.status.clone(), refresh_secs);
        let mut guard = handle
            .lock()
            .map_err(|_| " Failed to acquire lock for api key service handle")?;
        *guard = Some(new_handle);
        drop(guard);
        Ok(())
    }

    fn validate(&self, key: &str) -> bool {
        match self.get_status() {
            Ok(Status::Active) => {
                let dt = self.keys.lock().unwrap();
                dt.api_keys.contains(&key.to_owned())
            }
            _ => false,
        }
    }

    pub fn authenticate(&self, req: &Request<Body>) -> bool {
        match req.headers().get("apikey") {
            Some(h) => match String::from_utf8(h.as_bytes().to_vec()) {
                Ok(key) => self.validate(&key),
                Err(e) => {
                    error!("Unable to convert api key header to string, reason={}", e);
                    false
                }
            },
            None => false,
        }
    }
}
