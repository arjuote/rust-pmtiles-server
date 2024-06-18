use super::{
    cache::Cache,
    compress::decompress,
    fetcher::{Fetcher, FetcherError},
    models::Headers,
};
use crate::cache::CacheError;
use crate::compress::Compression;
use crate::helpers::{find_tile, get_entries, get_headers};
use std::num::TryFromIntError;
use thiserror::Error;

async fn fetch_metadata<T: Fetcher, C: Cache>(
    path: &str,
    headers: &Headers,
    client: &T,
    cache: Option<&C>,
) -> anyhow::Result<Vec<u8>, PMTilesError> {
    let cache_key = format!("{}|metadata", path);
    let (raw, _) = client
        .get_data_range(
            path,
            headers.json_metadata_offset as usize,
            headers.json_metadata_length as usize,
        )
        .await?;
    let decompressed = decompress(&raw, Compression::from(headers.internal_compression))?
        .into_iter()
        .collect::<Vec<u8>>();
    if let Some(cache) = cache {
        cache.set(&cache_key, &decompressed)?;
    }
    Ok(decompressed)
}

pub async fn get_metadata<T: Fetcher, C: Cache>(
    path: &str,
    client: &T,
    cache: Option<&C>,
) -> anyhow::Result<(Headers, serde_json::Value), PMTilesError> {
    let (headers, _) = get_headers(path, client, cache).await?;
    let cache_key = format!("{}|metadata", path);
    let raw = {
        match cache {
            Some(cache) => match cache.get(&cache_key) {
                Some(raw) => raw,
                None => fetch_metadata(path, &headers, client, Some(cache)).await?,
            },
            None => fetch_metadata(path, &headers, client, cache).await?,
        }
    };
    let json: serde_json::Value = serde_json::from_slice(&raw).map_err(|err| {
        tracing::error!("failed to deserialize json metadata: {}", err);
        PMTilesError::MetadataError("failed to deserialize tileset metadata".into())
    })?;
    Ok((headers, json))
}

#[cfg(test)]
#[tokio::test]
async fn test_get_metadata() {
    use crate::cache::InMemoryCache;
    use crate::fetcher::LocalFetcher;
    let client = LocalFetcher::new();
    let path = std::path::Path::new("../../testdata/data/data.pmtiles")
        .canonicalize()
        .unwrap();
    let (_, metadata) = get_metadata(
        path.to_str().unwrap(),
        &client,
        None as Option<&InMemoryCache>,
    )
    .await
    .unwrap();

    let format = metadata.get("format").unwrap().as_str().unwrap();
    assert_eq!(format, "pbf");

    let maxzoom = metadata.get("maxzoom").unwrap().as_str().unwrap();
    assert_eq!(maxzoom, "14");

    let minzoom = metadata.get("minzoom").unwrap().as_str().unwrap();
    assert_eq!(minzoom, "0");

    let layers = metadata.get("vector_layers").unwrap().as_array().unwrap();
    assert_eq!(layers.len(), 3);

    let city_borders = &layers[0];
    assert_eq!(
        city_borders.get("id").unwrap().as_str().unwrap(),
        "city_borders"
    );
    assert_eq!(
        city_borders
            .get("fields")
            .unwrap()
            .get("code")
            .unwrap()
            .as_str()
            .unwrap(),
        "Number"
    );
    assert_eq!(
        city_borders
            .get("fields")
            .unwrap()
            .get("type")
            .unwrap()
            .as_str()
            .unwrap(),
        "String"
    );
}

pub async fn get_tile<T: Fetcher, C: Cache>(
    z: u64,
    x: u64,
    y: u64,
    path: &str,
    client: &T,
    cache: Option<&C>,
) -> anyhow::Result<Vec<u8>, PMTilesError> {
    let (headers, mut entries) = get_headers(path, client, cache).await?;
    if z < headers.min_zoom as u64 || z > headers.max_zoom as u64 {
        return Err(PMTilesError::OutOfBoundsZ());
    }

    let mut offset = headers.root_directory_offset.clone();
    let mut length = headers.root_directory_length.clone();
    let tile_compression = Compression::from(headers.tile_compression);
    for i in 0..4 {
        if i > 0 {
            // First iteration entry fetch is skipped because we already have it
            // from fetching headers. If further iterations are needed, fetch
            // new entry data from the nested offset.
            let (tile_dir_data, _) = client
                .get_data_range(path, offset as usize, length as usize)
                .await?;
            entries = get_entries(
                &tile_dir_data,
                Compression::from(headers.internal_compression),
            )?
        }

        let tile_entry = find_tile(z, x, y, &entries)?;
        if tile_entry.run_length > 0 {
            let (tile_data, _) = client
                .get_data_range(
                    path,
                    (headers.tile_data_offset + tile_entry.offset) as usize,
                    tile_entry.length as usize,
                )
                .await?;
            let decompressed = decompress(&tile_data, tile_compression)?
                .into_iter()
                .collect();
            return Ok(decompressed);
        }
        offset = headers.leaf_directory_offset + tile_entry.offset;
        length = tile_entry.length;
    }
    Err(PMTilesError::NotFound(None))
}

#[cfg(test)]
#[tokio::test]
async fn test_get_headers() {
    use crate::cache::InMemoryCache;
    use crate::fetcher::LocalFetcher;

    let client = LocalFetcher::new();
    let path = std::path::Path::new("../../testdata/data/data.pmtiles")
        .canonicalize()
        .unwrap();
    let (headers, entries) = get_headers(
        path.to_str().unwrap(),
        &client,
        None as Option<&InMemoryCache>,
    )
    .await
    .unwrap();
    println!("{:?}", headers);
    assert_eq!(headers.num_addressed_tiles, 28);
    assert_eq!(headers.num_tile_contents, 28);
    assert_eq!(headers.num_tile_entries, 28);
    assert_eq!(entries.len(), 28);
}

#[tokio::test]
async fn test_get_tile() {
    use crate::cache::InMemoryCache;
    use crate::fetcher::LocalFetcher;
    let client = LocalFetcher::new();
    let path = std::path::Path::new("../../testdata/data/data.pmtiles")
        .canonicalize()
        .unwrap();
    let data = get_tile(
        14,
        9325,
        4732,
        path.to_str().unwrap(),
        &client,
        None as Option<&InMemoryCache>,
    )
    .await
    .unwrap();
    assert_eq!(data.len(), 78408);
}

#[derive(Error, Debug)]
pub enum PMTilesError {
    #[error("tile out of bounds error")]
    OutOfBounds(),
    #[error("tile out of bounds error")]
    OutOfBoundsZ(),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    #[error("not found")]
    NotFound(Option<String>),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal error:{0}")]
    Internal(String),
    #[error("tile internal conversion error")]
    TileConversionError(#[from] TryFromIntError),
    #[error("metadata error: {0}")]
    MetadataError(String),
    #[error(transparent)]
    CacheError(#[from] CacheError),
}

impl From<FetcherError> for PMTilesError {
    fn from(err: FetcherError) -> Self {
        match err {
            FetcherError::NotFound() => PMTilesError::NotFound(None),
            FetcherError::S3Error(err) => PMTilesError::BadRequest(err.to_string()),
            FetcherError::Other(err) => PMTilesError::Internal(err.to_string()),
        }
    }
}
