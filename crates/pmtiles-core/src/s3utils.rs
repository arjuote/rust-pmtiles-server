use super::fetcher::FetcherError;
#[cfg(feature = "s3")]
use aws_sdk_s3 as s3;

// Check whether path is S3 path, i.e. starts with s3-protocol specifier
pub fn is_s3_path(path: &str) -> bool {
    if path.starts_with(&"s3://") {
        true
    } else {
        false
    }
}

pub fn bucket_and_key_from_path<'a>(path: &'a str) -> anyhow::Result<(&'a str, String)> {
    let parts: Vec<&str> = path.trim_start_matches("s3://").split("/").collect();
    if parts.len() < 2 {
        anyhow::bail!("invalid path")
    }
    Ok((parts[0], parts[1..].join("/")))
}

#[cfg(feature = "s3")]
pub async fn get_object_range(
    path: &str,
    client: &s3::Client,
    offset: usize,
    length: usize,
) -> anyhow::Result<(Vec<u8>, Option<String>), FetcherError> {
    let (bucket, key) = bucket_and_key_from_path(path)?;
    tracing::debug!(
        "get_object_range bucket={}, key={}, offset={}, length={}",
        bucket,
        key,
        offset,
        length
    );
    let res = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .range(format!("bytes={}-{}", offset, offset + length))
        .send()
        .await
        .map_err(Into::<s3::Error>::into)?;
    let etag = res.e_tag;
    let data = res
        .body
        .collect()
        .await
        .map_err(|_| anyhow::anyhow!("unable to get data"))?;
    Ok((data.to_vec(), etag))
}

#[cfg(feature = "s3")]
pub async fn get_object(
    path: &str,
    client: &s3::Client,
) -> anyhow::Result<(Vec<u8>, Option<String>)> {
    let (bucket, key) = bucket_and_key_from_path(path)?;
    tracing::debug!("get_object bucket={}, key={}", bucket, key);

    let res = client.get_object().bucket(bucket).key(key).send().await?;
    let etag = res.e_tag;
    let data = res.body.collect().await?;
    Ok((data.to_vec(), etag))
}
