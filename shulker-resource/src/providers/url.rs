use snafu::{ensure, ResultExt, Snafu};
use std::collections::HashMap;
use std::fs::File;

use crate::provider::ResourceProvider;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Malformed spec for resource provider url: {}", reason))]
    MalformedSpec {
        reason: String,
    },
    NotFoundError,
    #[snafu(display(
        "Failed to perform request to url {}, server returned {} status",
        url,
        status
    ))]
    NetworkError {
        url: String,
        status: u16,
    },
    ReqwestError {
        source: reqwest::Error,
    },
    IOError {
        source: std::io::Error,
    },
}

pub struct UrlResourceProvider {
    pub url: String,
}

impl UrlResourceProvider {
    pub fn deserialize(spec: &HashMap<String, String>) -> Result<ResourceProvider, Error> {
        UrlResourceProvider::validate_spec(spec)?;

        Ok(ResourceProvider::Url(UrlResourceProvider {
            url: spec.get("url").unwrap().clone(),
        }))
    }

    pub async fn fetch(&self, file: &mut File) -> Result<(), Error> {
        let response = reqwest::get(&self.url).await.context(ReqwestError)?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let response_content = response.text().await.context(ReqwestError)?;
                std::io::copy(&mut response_content.as_bytes(), file).context(IOError)?;
                Ok(())
            }
            reqwest::StatusCode::NOT_FOUND => Err(Error::NotFoundError),
            status => Err(Error::NetworkError {
                url: self.url.to_owned(),
                status: status.as_u16(),
            }),
        }
    }

    pub async fn get_remote_hash(&self) -> Result<String, Error> {
        let url = format!("{}.sha1", self.url);
        let response = reqwest::get(&url).await.context(ReqwestError)?;

        match response.status() {
            reqwest::StatusCode::OK => response.text().await.context(ReqwestError),
            reqwest::StatusCode::NOT_FOUND => Err(Error::NotFoundError),
            status => Err(Error::NetworkError {
                url: url.clone(),
                status: status.as_u16(),
            }),
        }
    }

    fn validate_spec(spec: &HashMap<String, String>) -> Result<(), Error> {
        ensure!(
            spec.contains_key("url"),
            MalformedSpec {
                reason: "malformed url".to_owned(),
            }
        );

        Ok(())
    }
}