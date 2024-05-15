use super::{
    cache::Cache,
    compress::decompress,
    fetcher::Fetcher,
    models::Headers,
    pmtiles::PMTilesError,
    utils::{rotate, TILES_PER_LEVEL},
};
use crate::compress::Compression;
use crate::models::TileEntry;
use crate::utils::read_varint;

pub fn find_tile(
    z: u64,
    x: u64,
    y: u64,
    entries: &[TileEntry],
) -> anyhow::Result<&TileEntry, PMTilesError> {
    let tile_id = zxy_to_tile_id(z, x, y)?;

    let mut m = 0;
    let mut n = entries.len() - 1;
    while m <= n {
        let k = (n + m) >> 1;
        let cmp = i64::try_from(tile_id)? - i64::try_from(entries[k].tile_id)?;
        if cmp > 0 {
            m = k + 1;
        } else if cmp < 0 {
            n = k - 1
        } else {
            return Ok(&entries[k]);
        }
    }

    if entries[n].run_length == 0 {
        return Ok(&entries[n]);
    };
    if tile_id - entries[n].tile_id < entries[n].run_length {
        return Ok(&entries[n]);
    };
    Err(PMTilesError::OutOfBounds())
}

fn decode_entries(data: &[u8]) -> anyhow::Result<Vec<TileEntry>> {
    let mut pos = 0 as usize;
    let num_entries = read_varint(data, &mut pos)?;
    let mut entries = Vec::<TileEntry>::with_capacity(num_entries as usize);
    let mut last_id = 0;
    for _ in 0..num_entries {
        let val = read_varint(data, &mut pos)?;
        let tile_id = last_id + val;
        entries.push(TileEntry {
            tile_id: last_id + val,
            offset: 0,
            length: 0,
            run_length: 1,
        });
        last_id = tile_id;
    }

    for i in 0..num_entries as usize {
        let val = read_varint(data, &mut pos)?;
        entries[i].run_length = val;
    }

    for i in 0..num_entries as usize {
        let val = read_varint(data, &mut pos)?;
        entries[i].length = val;
    }

    for i in 0..num_entries as usize {
        let val = read_varint(data, &mut pos)?;
        if val == 0 && i > 0 {
            entries[i].offset = entries[i - 1].offset + entries[i - 1].length;
        } else if val == 0 && i == 0 {
            entries[i].offset = 0;
        } else {
            entries[i].offset = val - 1;
        }
    }
    Ok(entries)
}

pub fn get_entries(data: &[u8], compression: Compression) -> anyhow::Result<Vec<TileEntry>> {
    let decompressed = decompress(&data, compression)?;
    let data = decompressed.into_iter().collect::<Vec<u8>>();
    let entries = decode_entries(&data)?;
    Ok(entries)
}

const HEADER_SIZE_BYTES: usize = 127;

pub async fn get_headers<T: Fetcher, C: Cache>(
    path: &str,
    client: &T,
    cache: Option<&C>,
) -> Result<(Headers, Vec<TileEntry>), PMTilesError> {
    let cache_hit = {
        match &cache {
            Some(cache) => cache.get(&path),
            None => None,
        }
    };

    let raw_data = match cache_hit {
        Some(cached) => {
            tracing::info!("cache hit for key {}", path);
            cached
        }
        None => {
            let (data, _) = client.get_data_range(path, 0, 16384).await?;
            if let Some(cache) = cache {
                let res = cache.set(path, &data);
                if let Err(err) = res {
                    tracing::warn!("failed to cache key {} with {}", path, err);
                } else {
                    tracing::info!("cached key {}", path);
                }
            };
            data
        }
    };

    if raw_data.len() < HEADER_SIZE_BYTES {
        tracing::error!("{} tile dataset does not contain valid headers", path);
        return Err(PMTilesError::Other(anyhow::anyhow!(
            "tile dataset does not contain valid headers"
        )));
    }
    let headers = Headers::from_bytes(&raw_data[..HEADER_SIZE_BYTES])?;
    let root_dir_data = &raw_data[headers.root_directory_offset as usize
        ..(&headers.root_directory_offset + &headers.root_directory_length) as usize];

    let entries = get_entries(
        &root_dir_data,
        Compression::from(headers.internal_compression),
    )?;
    Ok((headers, entries))
}

fn zxy_to_tile_id(z: u64, x: u64, y: u64) -> anyhow::Result<u64> {
    if z > 26 {
        anyhow::bail!("zoom level exceeds maximum")
    }
    if x > 2_u64.pow(z as u32) - 1 || y > 2_u64.pow(z as u32) - 1 {
        anyhow::bail!("x or y outsize zoom level bounds")
    }

    let mut tmp_x: i64 = x.clone().try_into()?;
    let mut tmp_y: i64 = y.clone().try_into()?;

    let acc = TILES_PER_LEVEL[z as usize];
    let n = 2_i64.pow(z as u32);
    let mut d = 0;
    let mut s = n / 2;
    while s > 0 {
        let rx = if (tmp_x & s) > 0 { 1 } else { 0 };
        let ry = if (tmp_y & s) > 0 { 1 } else { 0 };
        d += s * s * ((3_i64 * rx) ^ ry);
        rotate(s, &mut tmp_x, &mut tmp_y, rx, ry);
        s = s / 2;
    }

    Ok(acc + d as u64)
}
