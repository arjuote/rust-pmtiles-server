use crate::config::{get_path, ServerConfig};
use pmtiles_core::models::Headers;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct StyleSource {
    pub url: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Style {
    pub version: u64,
    pub name: String,
    pub center: (f64, f64),
    pub zoom: u8,
    pub bearing: u8,
    pub pitch: u8,
    pub sources: HashMap<String, StyleSource>,
    pub sprite: String,
    pub glyphs: String,
    pub layers: serde_json::Value,
}

impl Style {
    pub fn resolve(&self, config: &ServerConfig) -> Style {
        let domain = config.get_domain();

        let mut resolved = self.to_owned();

        let resolved_sources: HashMap<_, _> = resolved
            .sources
            .into_iter()
            .map(|(key, mut src)| {
                let path = get_path(&src.url, &config);
                src.url = format!("{}/{}", &domain, &path);
                (key, src)
            })
            .collect();
        resolved.sources = resolved_sources;

        if !resolved.glyphs.is_empty() {
            let path = get_path(&resolved.glyphs, &config);
            resolved.glyphs = format!(
                "{}/{}",
                &domain.trim_end_matches("/"),
                &path.trim_start_matches("/")
            );
        };

        if !resolved.sprite.is_empty() {
            let path = get_path(&resolved.sprite, &config);
            resolved.sprite = format!(
                "{}/{}",
                &domain.trim_end_matches("/"),
                &path.trim_start_matches("/")
            );
        };

        resolved
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VectorLayer {
    id: String,
    fields: Option<HashMap<String, String>>,
    minzoom: Option<u64>,
    maxzoom: Option<u64>,
    description: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TileSource {
    tilejson: String,
    tiles: Vec<String>,
    name: Option<String>,
    bounds: Option<Vec<f64>>,
    center: Option<Vec<f64>>,
    maxzoom: Option<i64>,
    minzoom: Option<i64>,
    vector_layers: Vec<VectorLayer>,
}

impl TileSource {
    pub fn try_from_headers_and_metadata(
        tileset: &str,
        headers: &Headers,
        metadata: &serde_json::Value,
        cfg: &ServerConfig,
    ) -> Result<Self, anyhow::Error> {
        let name = metadata
            .get("name")
            .unwrap_or_else(|| &Value::Null)
            .as_str()
            .clone();

        let minzoom = metadata
            .get("minzoom")
            .unwrap_or_else(|| &Value::Null)
            .as_str()
            .clone()
            .unwrap_or_else(|| "")
            .parse::<i64>()
            .ok();
        let maxzoom = metadata
            .get("maxzoom")
            .unwrap_or_else(|| &Value::Null)
            .as_str()
            .clone()
            .unwrap_or_else(|| "")
            .parse::<i64>()
            .ok();

        let vector_layers: Vec<VectorLayer> = serde_json::from_value(
            metadata
                .get("vector_layers")
                .unwrap_or_else(|| &Value::Null)
                .clone(),
        )
        .unwrap_or_else(|_| vec![]);

        let tile_urls = vec![format!(
            "{}/{}/{{z}}/{{x}}/{{y}}.pbf",
            cfg.get_domain(),
            tileset
        )];
        Ok(TileSource {
            tilejson: "3.0.0".into(),
            name: name.map(ToOwned::to_owned),
            tiles: tile_urls,
            maxzoom,
            minzoom,
            center: Some(vec![
                headers.center_lon,
                headers.center_lat,
                f64::from(headers.center_zoom),
            ]),
            bounds: Some(vec![
                headers.min_lon,
                headers.min_lat,
                headers.max_lon,
                headers.max_lat,
            ]),
            vector_layers,
        })
    }
}

#[test]
fn test_style_deserialize() {
    let path = std::path::Path::new("../../testdata/styles/cadastral.json")
        .canonicalize()
        .unwrap();
    let data = std::fs::read_to_string(path).unwrap();
    let style: Style = serde_json::from_str(&data).unwrap();
    assert_eq!(style.name, "Cadastral Map");
    assert_eq!(style.version, 8);
    assert_eq!(style.center, (25f64, 60.5));
    assert_eq!(style.zoom, 12);
    assert_eq!(
        style.sources.get("cadastral_fi").unwrap().url,
        "pmtiles://cadastral_fi.pmtiles"
    );
}

#[test]
fn test_style_render() {
    let style_path = std::path::Path::new("../../testdata/styles/cadastral.json")
        .canonicalize()
        .unwrap();
    let style_data = std::fs::read_to_string(style_path).unwrap();
    let style: Style = serde_json::from_str(&style_data).unwrap();

    let config_path = std::path::Path::new("../../testdata/styles/config.json")
        .canonicalize()
        .unwrap();
    let config_data = std::fs::read_to_string(config_path).unwrap();
    let config: ServerConfig = serde_json::from_str(&config_data).unwrap();

    let expected_path = std::path::Path::new("../../testdata/styles/rendered_style.json")
        .canonicalize()
        .unwrap();
    let expected_data = std::fs::read_to_string(expected_path).unwrap();
    let expected: Style = serde_json::from_str(&expected_data).unwrap();

    let rendered = style.resolve(&config);

    assert_eq!(rendered.version, expected.version);
    assert_eq!(rendered.name, expected.name);
    assert_eq!(rendered.center, expected.center);
    assert_eq!(rendered.bearing, expected.bearing);
    assert_eq!(rendered.zoom, expected.zoom);
    assert_eq!(rendered.sprite, expected.sprite);
    assert_eq!(rendered.layers, expected.layers);
    assert_eq!(
        rendered.sources.get("cadastral_fi").unwrap(),
        expected.sources.get("cadastral_fi").unwrap()
    );
}
