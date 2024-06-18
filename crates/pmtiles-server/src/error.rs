use axum::{extract::rejection::BytesRejection, http::StatusCode, response::IntoResponse, Json};
use pmtiles_core::{fetcher::FetcherError, PMTilesError};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum APIError {
    #[error("validation error:{0}")]
    Validation(String),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("other error")]
    Other(#[from] anyhow::Error),
    #[error("not found")]
    NotFound(Option<String>),
    #[error("bad parameter")]
    BadRequest(Option<String>),
    #[error(transparent)]
    BytesRejection(#[from] BytesRejection),
}

impl From<PMTilesError> for APIError {
    fn from(err: PMTilesError) -> Self {
        match err {
            PMTilesError::OutOfBounds() => APIError::NotFound(None),
            PMTilesError::OutOfBoundsZ() => APIError::NotFound(None),
            PMTilesError::Other(err) => APIError::Internal(format!("internal error: {}", err)),
            PMTilesError::NotFound(_) => APIError::NotFound(None),
            PMTilesError::TileConversionError(err) => {
                APIError::Internal(format!("tile data conversion failed: {})", err))
            }
            PMTilesError::MetadataError(err) => APIError::Internal(err.to_string()),
            PMTilesError::BadRequest(err) => APIError::BadRequest(Some(err)),
            PMTilesError::Internal(err) => APIError::Internal(err),
            PMTilesError::CacheError(err) => APIError::Internal(err.to_string()),
        }
    }
}

impl From<FetcherError> for APIError {
    fn from(err: FetcherError) -> Self {
        match err {
            FetcherError::NotFound() => APIError::NotFound(None),
            FetcherError::S3Error(_) => APIError::Internal("failed to fetch S3 data".into()),
            FetcherError::Other(err) => APIError::Other(err),
        }
    }
}

impl IntoResponse for APIError {
    fn into_response(self) -> axum::response::Response {
        let (status, payload) = match &self {
            APIError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, Json(self)),
            APIError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(self)),
            APIError::Other(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(self)),
            APIError::NotFound(_) => (StatusCode::NOT_FOUND, Json(self)),
            APIError::BytesRejection(_) => (StatusCode::BAD_REQUEST, Json(self)),
            APIError::BadRequest(_) => (StatusCode::BAD_REQUEST, Json(self)),
        };

        (status, payload).into_response()
    }
}

impl Serialize for APIError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut errors: Vec<String> = vec![];
        let mut code = 500;
        let mut description = None;
        let mut name = "Internal server error";
        match self {
            APIError::Validation(err) => {
                code = 422;
                name = "Unprocessable entity";
                description = Some("Validation failed");
                errors.push(err.into());
            }
            APIError::Internal(_) => {}
            APIError::Other(_) => {}
            APIError::NotFound(err) => {
                code = 404;
                name = "Not found";
                description = Some("Requested item was not found in the collection");
                if let Some(err) = err {
                    errors.push(err.into());
                }
            }
            APIError::BytesRejection(_) => {
                code = 400;
                name = "Bad request";
                description = Some("Failed to read body");
            }
            APIError::BadRequest(err) => {
                code = 400;
                name = "Bad Request";
                description = Some("Bad request");
                if let Some(err) = err {
                    errors.push(err.into());
                }
            }
        }
        let mut state = serializer.serialize_struct("errors", 4)?;
        state.serialize_field("code", &code)?;
        state.serialize_field("description", &description)?;
        state.serialize_field("name", &name)?;
        state.serialize_field("errors", &errors)?;
        state.end()
    }
}
