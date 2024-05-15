use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Debug)]
pub struct Headers {
    pub spec_version: u8,
    pub root_directory_offset: u64,
    pub root_directory_length: u64,
    pub json_metadata_offset: u64,
    pub json_metadata_length: u64,
    pub leaf_directory_offset: u64,
    pub leaf_directory_length: u64,
    pub tile_data_offset: u64,
    pub tile_data_length: u64,
    pub num_addressed_tiles: u64,
    pub num_tile_contents: u64,
    pub clustered: u8,
    pub internal_compression: u8,
    pub tile_compression: u8,
    pub tile_type: u8,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub num_tile_entries: u64,
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
    pub center_zoom: u8,
    pub center_lon: f64,
    pub center_lat: f64,
    pub etag: Option<String>,
}

impl Headers {
    pub fn from_bytes<'a>(v: &'a [u8]) -> anyhow::Result<Self> {
        let mut rdr = Cursor::new(v);
        if rdr.read_u16::<LittleEndian>()? != 0x4d50 {
            return Err(anyhow::anyhow!(format!(
                "wrong magic number for pmtiles archive - input file is likely not pmtiles",
            )));
        }
        rdr.set_position(7);
        let spec_version = rdr.read_u8()?;
        if spec_version != 3 {
            return Err(anyhow::anyhow!(format!(
                "pmtiles version 3 required but got version {}",
                spec_version
            )));
        }
        let root_directory_offset = rdr.read_u64::<LittleEndian>()?;
        let root_directory_length = rdr.read_u64::<LittleEndian>()?;
        let json_metadata_offset = rdr.read_u64::<LittleEndian>()?;
        let json_metadata_length = rdr.read_u64::<LittleEndian>()?;
        let leaf_directory_offset = rdr.read_u64::<LittleEndian>()?;
        let leaf_directory_length = rdr.read_u64::<LittleEndian>()?;
        let tile_data_offset = rdr.read_u64::<LittleEndian>()?;
        let tile_data_length = rdr.read_u64::<LittleEndian>()?;
        let num_addressed_tiles = rdr.read_u64::<LittleEndian>()?;
        let num_tile_entries = rdr.read_u64::<LittleEndian>()?;
        let num_tile_contents = rdr.read_u64::<LittleEndian>()?;
        let clustered = rdr.read_u8()?;
        let internal_compression = rdr.read_u8()?;
        let tile_compression = rdr.read_u8()?;
        let tile_type = rdr.read_u8()?;
        let min_zoom = rdr.read_u8()?;
        let max_zoom = rdr.read_u8()?;
        let min_lon = rdr.read_i32::<LittleEndian>()? as f64 / 10_000_000.;
        let min_lat = rdr.read_i32::<LittleEndian>()? as f64 / 10_000_000.;
        let max_lon = rdr.read_i32::<LittleEndian>()? as f64 / 10_000_000.;
        let max_lat = rdr.read_i32::<LittleEndian>()? as f64 / 10_000_000.;
        let center_zoom = rdr.read_u8()?;
        let center_lon = rdr.read_i32::<LittleEndian>()? as f64 / 10_000_000.;
        let center_lat = rdr.read_i32::<LittleEndian>()? as f64 / 10_000_000.;

        Ok(Headers {
            spec_version,
            root_directory_length,
            root_directory_offset,
            json_metadata_length,
            json_metadata_offset,
            leaf_directory_length,
            leaf_directory_offset,
            tile_data_length,
            tile_data_offset,
            num_tile_entries,
            num_addressed_tiles,
            num_tile_contents,
            clustered,
            internal_compression,
            tile_compression,
            tile_type,
            min_zoom,
            max_zoom,
            min_lon,
            min_lat,
            max_lon,
            max_lat,
            center_zoom,
            center_lon,
            center_lat,
            etag: None,
        })
    }
}

pub struct TileEntry {
    pub tile_id: u64,
    pub offset: u64,
    pub length: u64,
    pub run_length: u64,
}
