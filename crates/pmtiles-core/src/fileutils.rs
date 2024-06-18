use std::io::SeekFrom;

use tokio::{
    fs::{read, File},
    io::{AsyncReadExt, AsyncSeekExt},
};

pub async fn get_file_range(path: &str, offset: usize, length: usize) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(path).await?;
    file.seek(SeekFrom::Start(offset as u64)).await?;
    let mut buffer: Vec<u8> = vec![0; length];
    file.read_exact(&mut buffer).await?;
    Ok(buffer)
}

pub async fn get_file(path: &str) -> anyhow::Result<Vec<u8>> {
    let buffer = read(&path).await?;
    Ok(buffer)
}
