use crate::config::{prefix_with_home, ServerConfig};
use crate::error::APIError;
use crate::font::fetch_fonts;
use crate::server::AppState;
use crate::style::{Style, TileSource};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum::{routing::get, Router};
use hyper::StatusCode;
use pmtiles_core::cache::InMemoryCache;
use pmtiles_core::fetcher::{Fetcher, S3OrLocalFetcher};
use pmtiles_core::{self, get_metadata};
use std::borrow::Borrow;

async fn fetch_style<F: Fetcher>(
    config: &ServerConfig,
    fetcher: &F,
    style_id: &str,
) -> Result<Style, APIError> {
    let style_cfg = config
        .styles
        .get(style_id)
        .ok_or_else(|| APIError::NotFound(Some("style not found".into())))?;
    let style_path = config.get_style_path(&style_cfg).map_err(|err| {
        tracing::error!("unable to get style {} path: {}", style_id, err);
        APIError::Internal("unable to get style path".into())
    })?;
    let (data, _) = fetcher.get_data(&style_path).await.map_err(|err| {
        tracing::error!("unable to get style {}: {}", style_id, err);
        APIError::NotFound(Some("unable to fetch style".into()))
    })?;
    let style: Style = serde_json::from_slice(&data).map_err(|err| {
        tracing::error!("unable to parse style {}: {}", style_id, err);
        APIError::Internal("error parsing style".into())
    })?;
    let resolved = style.resolve(&config);
    Ok(resolved)
}

async fn get_style(
    State(state): State<AppState>,
    Path(style_id): Path<String>,
) -> Result<Response, APIError> {
    let fetcher: &S3OrLocalFetcher = state.fetcher.borrow();
    let resolved = fetch_style(&state.config, fetcher, &style_id).await?;
    Ok((StatusCode::OK, Json(resolved)).into_response())
}

async fn get_tilejson(
    State(state): State<AppState>,
    Path(tileset): Path<String>,
) -> Result<Response, APIError> {
    let tileset = tileset.replace(".json", "");
    let path = &state.config.get_tileset_path(&tileset).map_err(|err| {
        tracing::error!("unable to get tileset: {}", err);
        APIError::NotFound(Some("tileset not found".into()))
    })?;
    let fetcher: &S3OrLocalFetcher = state.fetcher.borrow();
    let cache: &InMemoryCache = state.cache.borrow();
    let (headers, metadata) = get_metadata(path, fetcher, Some(cache)).await?;
    let tilejson =
        TileSource::try_from_headers_and_metadata(&tileset, &headers, &metadata, &state.config)?;
    Ok((StatusCode::OK, Json(tilejson)).into_response())
}

fn parse_tile(tile_param: &str) -> Result<(u64, u64, u64), APIError> {
    let parts = tile_param
        .replace(".pbf", "")
        .split("/")
        .map(|p| p.parse::<u64>())
        .collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(APIError::Validation(
            "invalid tile parameters: provide tiles as /z/x/y.pbf".to_string(),
        ));
    };
    let z = parts[0]
        .as_ref()
        .map_err(|_| APIError::Validation("invalid tile specified".to_string()))?;
    let x = parts[1]
        .as_ref()
        .map_err(|_| APIError::Validation("invalid tile specified".to_string()))?;
    let y = parts[2]
        .as_ref()
        .map_err(|_| APIError::Validation("invalid tile specified".to_string()))?;
    Ok((*z, *x, *y))
}

async fn get_tile(
    State(state): State<AppState>,
    // Path(params): Path<GetTileParams>,
    Path((tileset, tile)): Path<(String, String)>,
) -> Result<Response, APIError> {
    let path = &state.config.get_tileset_path(&tileset).map_err(|err| {
        tracing::error!("unable to get tileset: {}", err);
        APIError::NotFound(Some("tileset not found".into()))
    })?;
    tracing::debug!("Fetching tiles from path {}", path);
    let (z, x, y) = parse_tile(&tile)?;
    let fetcher: &S3OrLocalFetcher = state.fetcher.borrow();
    let cache: &InMemoryCache = state.cache.borrow();
    let tile_res = pmtiles_core::get_tile(z, x, y, &path, fetcher, Some(cache)).await;
    match tile_res {
        Ok(tile_data) => Response::builder()
            .body(Body::from(tile_data))
            .map_err(|err| {
                tracing::error!("{}", err);
                APIError::Internal("invalid tile data".into())
            }),
        Err(err) => {
            tracing::error!("{}", err);
            return Err(err.into());
        }
    }
}

async fn get_fontstack(
    State(state): State<AppState>,
    Path((fontstack, range)): Path<(String, String)>,
) -> Result<Response, APIError> {
    let range = range.replace(".pbf", "");
    let font_paths_resolved = fontstack
        .split(",")
        .map(|f| state.config.get_font_path(f.trim(), &range))
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let fetcher: &S3OrLocalFetcher = state.fetcher.borrow();
    let cache: &InMemoryCache = state.cache.borrow();
    let result = fetch_fonts(font_paths_resolved, fetcher, Some(cache)).await;
    match result {
        Ok(fonts_pbf) => Response::builder()
            .body(Body::from(fonts_pbf))
            .map_err(|err| {
                tracing::error!("{}", err);
                APIError::Internal("invalid font data".into())
            }),
        Err(err) => {
            tracing::error!("{}", err);
            Err(err)
        }
    }
}

async fn get_sprite(
    State(state): State<AppState>,
    Path(sprite): Path<String>,
) -> Result<Response, APIError> {
    let fetcher: &S3OrLocalFetcher = state.fetcher.borrow();
    let cache: &InMemoryCache = state.cache.borrow();
    todo!()
}

pub fn create_router(state: AppState) -> Router {
    let mut get_tilejson_path = format!(
        "/{}/:tileset",
        &state
            .config
            .options
            .paths
            .pmtiles
            .clone()
            .unwrap_or("data".into())
    );
    prefix_with_home(&mut get_tilejson_path, &state.config, true, false);

    let get_tile_path = format!("{}/*tile", get_tilejson_path);

    let mut get_style_path = format!(
        "/{}/:style_id",
        &state
            .config
            .options
            .paths
            .styles
            .clone()
            .unwrap_or("style".into())
    );
    prefix_with_home(&mut get_style_path, &state.config, true, false);

    let get_style_json_path = format!("{}/style.json", get_style_path);

    let mut router = Router::new()
        .route(&get_style_path, get(get_style))
        .route(&get_style_json_path, get(get_style))
        .route(&get_tilejson_path, get(get_tilejson))
        .route(&get_tile_path, get(get_tile));

    tracing::debug!(
        "Exposing paths: \nGET {} \nGET {} \nGET {} \nGET {}",
        get_style_path,
        get_style_json_path,
        get_tilejson_path,
        get_tile_path
    );

    if let Some(fonts_path) = &state.config.options.paths.fonts {
        let mut get_font_path = format!("/{}/:fontstack/*range", fonts_path);
        prefix_with_home(&mut get_font_path, &state.config, true, false);
        router = router.route(&get_font_path, get(get_fontstack));
        tracing::debug!("Exposing path: \nGET {}", get_font_path,);
    }
    if let Some(sprites_path) = &state.config.options.paths.sprites {
        let mut get_sprite_path = format!("/{}/*sprite", sprites_path);
        prefix_with_home(&mut get_sprite_path, &state.config, true, false);
        router = router.route(&get_sprite_path, get(get_sprite));
        tracing::debug!("Exposing path: \nGET {}", get_sprite_path,);
    }

    router.with_state(state)
}
