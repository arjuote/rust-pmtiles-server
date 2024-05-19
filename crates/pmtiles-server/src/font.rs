use pbf_font_tools::protobuf::{self, Message};
use pbf_font_tools::{combine_glyphs, Glyphs};
use pmtiles::{cache::Cache, fetcher::Fetcher};

use crate::error::APIError;

fn combine_fonts(fonts_data: Vec<Vec<u8>>) -> Result<Vec<u8>, APIError> {
    if fonts_data.len() == 0 {
        return Ok(Vec::new());
    }
    let glyphs = fonts_data
        .iter()
        .map(|x| Glyphs::parse_from_bytes(x))
        .collect::<Result<Vec<_>, protobuf::Error>>()
        .map_err(|err| {
            tracing::error!("{}", err);
            APIError::Internal("unable to parse font".into())
        })?;
    let glyphs = combine_glyphs(glyphs);
    match glyphs {
        Some(glyphs) => glyphs.write_to_bytes().map_err(|err| {
            tracing::error!("{}", err);
            APIError::Internal("unable to combine fonts".into())
        }),
        None => Err(APIError::Internal("unable to combine fonts".into())),
    }
}

pub async fn fetch_fonts<F: Fetcher, C: Cache>(
    paths: Vec<String>,
    client: &F,
    cache: Option<&C>,
) -> Result<Vec<u8>, APIError> {
    let key = paths.join(",");
    let cache_hit = {
        match &cache {
            Some(cache) => cache.get(&key),
            None => None,
        }
    };
    let pbf_data = match cache_hit {
        Some(cached) => {
            tracing::debug!("cache hit for key {}", key);
            cached
        }
        None => {
            let mut fonts_data: Vec<Vec<u8>> = Vec::with_capacity(paths.len());
            for path in paths {
                let (data, _) = client.get_data(&path).await.map_err(|err| {
                    tracing::error!("{}", err);
                    err
                })?;
                fonts_data.push(data);
            }
            let fonts_combined = combine_fonts(fonts_data)?;
            if let Some(cache) = cache {
                let res = cache.set(&key, &fonts_combined);
                if let Err(err) = res {
                    tracing::warn!("failed to cache key {} with {}", key, err);
                } else {
                    tracing::debug!("cached key {}", key);
                }
            };
            fonts_combined
        }
    };

    Ok(pbf_data)
}

#[cfg(test)]
use pmtiles::cache::InMemoryCache;
#[cfg(test)]
use pmtiles::fetcher::LocalFetcher;

#[tokio::test]
async fn test_fetch_fonts() {
    let client = LocalFetcher::new();
    let fonts = vec![
        "../../testdata/fonts/Arial Unicode MS Regular/0-255.pbf".to_string(),
        "../../testdata/fonts/Arial Unicode MS Bold/0-255.pbf".to_string(),
    ];
    let data = fetch_fonts(fonts, &client, None as Option<&InMemoryCache>)
        .await
        .unwrap();
    assert_eq!(data.len(), 79096)
}
