use super::fileutils::{get_file, get_file_range};

use crate::s3utils::is_s3_path;
#[cfg(feature = "s3")]
use crate::s3utils::{get_object, get_object_range};

#[cfg(feature = "s3")]
use aws_sdk_s3 as s3;
#[cfg(feature = "s3")]
use aws_sdk_s3::error::ProvideErrorMetadata;
#[cfg(feature = "s3")]
use s3::Error as S3Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetcherError {
    #[error("not found")]
    NotFound(),
    #[error("s3 error: {0}")]
    S3Error(String),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

#[cfg(feature = "s3")]
impl From<S3Error> for FetcherError {
    fn from(err: S3Error) -> Self {
        match err {
            S3Error::InvalidObjectState(err) => {
                FetcherError::S3Error(format!("invalid object state: {}", err))
            }
            S3Error::NoSuchBucket(_) => FetcherError::NotFound(),
            S3Error::NoSuchKey(_) => FetcherError::NotFound(),
            S3Error::NotFound(_) => FetcherError::NotFound(),
            _ => {
                tracing::error!("s3 error with code: {:?}", err.code());
                FetcherError::S3Error("s3 error".into())
            }
        }
    }
}

pub trait Fetcher {
    fn get_data_range(
        &self,
        path: &str,
        offset: usize,
        length: usize,
    ) -> impl std::future::Future<Output = Result<(Vec<u8>, Option<String>), FetcherError>> + Send;
    fn get_data(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = Result<(Vec<u8>, Option<String>), FetcherError>> + Send;
}

#[cfg(feature = "s3")]
pub struct S3Fetcher {
    client: s3::Client,
}

#[cfg(feature = "s3")]
impl Fetcher for S3Fetcher {
    async fn get_data_range(
        &self,
        path: &str,
        offset: usize,
        length: usize,
    ) -> Result<(Vec<u8>, Option<String>), FetcherError> {
        if is_s3_path(path) {
            get_object_range(path, &self.client, offset, length)
                .await
                .map_err(Into::into)
        } else {
            Err(anyhow::anyhow!("invalid S3 path").into())
        }
    }
    async fn get_data(&self, path: &str) -> Result<(Vec<u8>, Option<String>), FetcherError> {
        if is_s3_path(path) {
            get_object(path, &self.client).await.map_err(Into::into)
        } else {
            Err(anyhow::anyhow!("invalid S3 path").into())
        }
    }
}

pub struct LocalFetcher {}
impl LocalFetcher {
    pub fn new() -> Self {
        LocalFetcher {}
    }
}
impl Fetcher for LocalFetcher {
    async fn get_data_range(
        &self,
        path: &str,
        offset: usize,
        length: usize,
    ) -> Result<(Vec<u8>, Option<String>), FetcherError> {
        Ok((get_file_range(path, offset, length).await?, None))
    }
    async fn get_data(&self, path: &str) -> Result<(Vec<u8>, Option<String>), FetcherError> {
        Ok((get_file(path).await?, None))
    }
}

#[cfg(feature = "s3")]
pub struct S3OrLocalFetcher {
    client: s3::Client,
}

#[cfg(feature = "s3")]
impl Fetcher for S3OrLocalFetcher {
    async fn get_data_range(
        &self,
        path: &str,
        offset: usize,
        length: usize,
    ) -> Result<(Vec<u8>, Option<String>), FetcherError> {
        if is_s3_path(path) {
            get_object_range(path, &self.client, offset, length)
                .await
                .map_err(Into::into)
        } else {
            Ok((get_file_range(path, offset, length).await?, None))
        }
    }
    async fn get_data(&self, path: &str) -> Result<(Vec<u8>, Option<String>), FetcherError> {
        if is_s3_path(path) {
            get_object(path, &self.client).await.map_err(Into::into)
        } else {
            Ok((get_file(path).await?, None))
        }
    }
}

#[cfg(feature = "s3")]
impl S3OrLocalFetcher {
    pub fn new(s3: s3::Client) -> Self {
        S3OrLocalFetcher { client: s3 }
    }
}
