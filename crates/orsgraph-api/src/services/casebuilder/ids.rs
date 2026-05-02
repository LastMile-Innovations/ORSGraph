use super::*;

pub(super) fn timestamp_after(seconds: u64) -> String {
    (now_secs() + seconds).to_string()
}

pub(super) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub(super) fn parse_timestamp(value: &str) -> Option<u64> {
    value.parse().ok()
}

pub(super) fn now_string() -> String {
    now_secs().to_string()
}

pub(super) fn days_until(iso_date: &str) -> i64 {
    let Some(target) = days_from_iso_date(iso_date) else {
        return 0;
    };
    let today = (now_secs() / 86_400) as i64;
    target - today
}

pub(super) fn days_from_iso_date(value: &str) -> Option<i64> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

pub(super) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = (year - era * 400) as u32;
    let month = month as i32;
    let doy = ((153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5) as u32 + day - 1;
    let doe = yoe as i64 * 365 + (yoe / 4) as i64 - (yoe / 100) as i64 + doy as i64;
    era as i64 * 146_097 + doe - 719_468
}

pub(super) fn generate_id(prefix: &str, seed: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("{prefix}:{}:{millis}", slug(seed))
}

pub(super) fn generate_opaque_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let seed = format!("{prefix}:{nanos}");
    format!("{prefix}:{}", hex_prefix(seed.as_bytes(), 26))
}

pub(super) fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(chars);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
        if out.len() >= chars {
            break;
        }
    }
    out.truncate(chars);
    out
}

pub(super) fn slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "item".to_string()
    } else {
        slug.chars().take(48).collect()
    }
}

pub(super) fn short_name(name: &str) -> String {
    name.split(" v. ").next().unwrap_or(name).trim().to_string()
}

pub(super) fn title_from_filename(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(filename)
        .replace(['_', '-'], " ")
}

pub(super) fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn normalize_upload_relative_path(value: Option<String>) -> ApiResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.replace('\\', "/");
    let trimmed = normalized.trim_matches('/');
    if trimmed.trim().is_empty() {
        return Ok(None);
    }
    if normalized.starts_with('/')
        || normalized.starts_with('\\')
        || normalized.contains('\0')
        || normalized.chars().any(|ch| ch.is_control())
    {
        return Err(ApiError::BadRequest(
            "Upload relative_path must be a safe relative path".to_string(),
        ));
    }
    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        let segment = segment.trim();
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(ApiError::BadRequest(
                "Upload relative_path cannot contain empty, current, or parent segments"
                    .to_string(),
            ));
        }
        if segment.contains(':') {
            return Err(ApiError::BadRequest(
                "Upload relative_path cannot contain drive or URL separators".to_string(),
            ));
        }
        segments.push(segment);
    }
    Ok(Some(segments.join("/")))
}

pub(super) fn normalize_upload_batch_id(value: Option<String>) -> ApiResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.len() > 96
        || trimmed
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.')))
    {
        return Err(ApiError::BadRequest(
            "upload_batch_id must use only letters, numbers, dash, underscore, colon, or dot"
                .to_string(),
        ));
    }
    Ok(Some(trimmed.to_string()))
}

#[cfg(test)]
pub(super) fn sanitize_filename(value: &str) -> String {
    let candidate = Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("upload.txt");
    sanitize_path_segment(candidate)
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    format!("sha256:{out}")
}
