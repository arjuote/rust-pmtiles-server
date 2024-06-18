use brotli_decompressor::Decompressor;
use std::io::Read;
use zstd;
use zune_inflate::DeflateDecoder;

pub enum Compression {
    Unknown,
    None,
    Gzip,
    Brotli,
    Zstd,
}

impl From<u8> for Compression {
    fn from(value: u8) -> Self {
        match value {
            1 => Compression::None,
            2 => Compression::Gzip,
            3 => Compression::Brotli,
            4 => Compression::Zstd,
            _ => Compression::Unknown,
        }
    }
}

pub fn decompress(
    data: &[u8],
    compression: Compression,
) -> anyhow::Result<impl IntoIterator<Item = u8>> {
    match compression {
        Compression::Unknown => Ok(data.to_vec()),
        Compression::None => Ok(data.to_vec()),
        Compression::Gzip => {
            let mut decoder = DeflateDecoder::new(data);
            let decompressed = decoder.decode_gzip()?;
            Ok(decompressed)
        }
        Compression::Brotli => {
            let mut decoder = Decompressor::new(data, 4096);
            let mut decompressed = Vec::with_capacity(data.len());
            decoder.read_to_end(&mut decompressed)?;
            Ok(decompressed)
        }
        Compression::Zstd => {
            let decompressed = zstd::decode_all(data)?;
            Ok(decompressed)
        }
    }
}
