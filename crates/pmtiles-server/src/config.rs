use crate::{
    error::APIError,
    utils::{canonicalize_local_path, join_path, pick_random_element, trim_slash},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct PathsConfig {
    pub home: Option<String>,
    pub root: Option<String>,
    pub fonts: Option<String>,
    pub sprites: Option<String>,
    pub icons: Option<String>,
    pub pmtiles: Option<String>,
    pub styles: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct OptionsConfig {
    pub paths: PathsConfig,
    pub domains: Vec<String>,
}
#[derive(Serialize, Deserialize)]
pub struct StyleConfig {
    pub style: String,
}
#[derive(Serialize, Deserialize)]
pub struct DataConfig {
    pub pmtiles: String,
}
#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    pub options: OptionsConfig,
    pub styles: HashMap<String, StyleConfig>,
    pub data: HashMap<String, DataConfig>,
}

impl ServerConfig {
    pub fn get_style_path(&self, style: &StyleConfig) -> anyhow::Result<String> {
        let root = canonicalize_local_path(&self.options.paths.root.clone().unwrap_or(".".into()))?;
        let styles_prefix = &self.options.paths.styles.clone().unwrap_or("styles".into());
        let style_path = &style.style;
        Ok(format!("{}/{}/{}", root, styles_prefix, style_path))
    }
    pub fn get_font_path(&self, font: &str, range: &str) -> anyhow::Result<String> {
        let root = canonicalize_local_path(&self.options.paths.root.clone().unwrap_or(".".into()))?;
        let fonts_prefix = &self.options.paths.fonts.clone().unwrap_or("fonts".into());
        Ok(format!("{}/{}/{}/{}.pbf", root, fonts_prefix, font, range))
    }
    pub fn get_tileset_path(&self, tileset: &str) -> anyhow::Result<String> {
        let root = canonicalize_local_path(&self.options.paths.root.clone().unwrap_or(".".into()))?;
        let found = self
            .data
            .get(tileset)
            .ok_or_else(|| APIError::NotFound(Some("tileset does not exist in config".into())))?;
        let pmtiles_prefix = &self.options.paths.pmtiles;
        if let Some(pmtiles_prefix) = pmtiles_prefix {
            Ok(format!("{}/{}/{}", root, pmtiles_prefix, found.pmtiles))
        } else {
            Ok(format!("{}/{}", root, found.pmtiles))
        }
    }
    pub fn get_domain(&self) -> String {
        let default_domain = "".to_string();
        let picked_domain: String = {
            if let Ok(domain) = std::env::var("API_DOMAIN") {
                domain
            } else {
                pick_random_element(&self.options.domains)
                    .unwrap_or_else(|| &default_domain)
                    .to_owned()
            }
        };
        picked_domain
    }
}

pub fn get_data_path(source: &str, url: &str, config: &ServerConfig) -> String {
    let parsed = Url::parse(url);
    if let Ok(parsed) = parsed {
        // Parse the path part and restore the curly braces that get url-encoded
        let prefix = match parsed.scheme() {
            "pmtiles" => config.options.paths.pmtiles.as_ref(),
            _ => None,
        };
        let path = source.to_string();
        let mut prefixed_path = {
            if let Some(prefix) = prefix {
                trim_slash(&join_path(prefix, &path))
            } else {
                trim_slash(&path)
            }
        };
        prefix_with_home(&mut prefixed_path, &config, false, true);
        return prefixed_path;
    }
    return "".to_owned();
}

pub fn get_path(url: &str, config: &ServerConfig) -> String {
    let parsed = Url::parse(url);
    if let Ok(parsed) = parsed {
        // Parse the path part and restore the curly braces that get url-encoded
        let path_part = parsed.path().replace("%7B", "{").replace("%7D", "}");
        let prefix = match parsed.scheme() {
            "fonts" => config.options.paths.fonts.as_ref(),
            "sprites" => config.options.paths.sprites.as_ref(),
            "styles" => config.options.paths.styles.as_ref(),
            _ => None,
        };
        let path = {
            if let Some(domain) = parsed.domain() {
                if !path_part.is_empty() {
                    trim_slash(&join_path(domain, &path_part))
                } else {
                    domain.to_string()
                }
            } else {
                path_part.to_string()
            }
        };
        let mut prefixed_path = {
            if let Some(prefix) = prefix {
                trim_slash(&join_path(prefix, &path))
            } else {
                trim_slash(&path)
            }
        };
        prefix_with_home(&mut prefixed_path, &config, false, true);
        return prefixed_path;
    }
    return "".to_owned();
}

pub fn prefix_with_home(
    path: &mut String,
    cfg: &ServerConfig,
    leading_slash: bool,
    trailing_slash: bool,
) {
    if let Some(home) = &cfg.options.paths.home {
        if !home.is_empty() {
            if trailing_slash {
                path.insert_str(0, &format!("{}/", home.trim_end_matches("/")));
            } else {
                path.insert_str(0, home);
            }
        }
    }
    if leading_slash && !path.starts_with("/") {
        path.insert(0, '/')
    }
}

#[test]
fn test_config_deserialize() {
    let path = std::path::Path::new("../../testdata/styles/config.json")
        .canonicalize()
        .unwrap();
    let data = std::fs::read_to_string(path).unwrap();
    let cfg: ServerConfig = serde_json::from_str(&data).unwrap();
    assert_eq!(cfg.options.domains[0], "http://api.example.com/tile");
    assert_eq!(cfg.options.paths.pmtiles, Some("data".into()));
    assert_eq!(cfg.options.paths.fonts, Some("fonts".into()));
    assert_eq!(cfg.options.paths.icons, Some("".into()));
    assert_eq!(
        cfg.data.get("cadastral_fi").unwrap().pmtiles,
        "cadastral_fi.pmtiles"
    );
    assert_eq!(cfg.styles.get("cadastral").unwrap().style, "cadastral.json");
}
