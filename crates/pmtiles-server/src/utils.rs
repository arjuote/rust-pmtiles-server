use pmtiles_core::s3utils::is_s3_path;
use rand::Rng;

pub fn canonicalize_local_path(path: &str) -> anyhow::Result<String> {
    if is_s3_path(path) {
        Ok(path.into())
    } else {
        let ref_path = {
            if path == "" {
                "."
            } else {
                path
            }
        };
        let local_path = std::path::Path::new(ref_path).canonicalize()?;
        let stringified_path = local_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid path {}", &local_path.to_string_lossy()))?;
        Ok(stringified_path.into())
    }
}

pub fn pick_random_element<T>(array: &[T]) -> Option<&T> {
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..array.len());
    array.get(index)
}

pub fn join_path(part1: &str, part2: &str) -> String {
    format!(
        "{}/{}",
        part1.trim_end_matches("/"),
        part2.trim_start_matches("/")
    )
}

pub fn trim_slash(path: &str) -> String {
    format!("{}", path.trim_matches('/'))
}
