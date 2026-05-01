use super::*;

pub(super) fn validate_mime_type(mime_type: Option<&str>) -> ApiResult<()> {
    let Some(mime_type) = mime_type else {
        return Ok(());
    };
    let allowed = [
        "text/",
        "image/",
        "audio/",
        "video/",
        "application/pdf",
        "application/json",
        "application/octet-stream",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.",
        "application/vnd.ms-excel",
        "application/vnd.ms-powerpoint",
        "application/zip",
    ];
    if allowed.iter().any(|prefix| mime_type.starts_with(prefix)) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "Unsupported upload MIME type {mime_type}"
        )))
    }
}
