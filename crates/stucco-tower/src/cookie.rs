use http::{
    HeaderMap, HeaderValue,
    header::{CACHE_CONTROL, COOKIE},
};

// Reject duplicate cookie names, including across Cookie header lines.
// The caller must not silently choose an attacker-controlled sibling cookie.
pub(crate) fn read<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, ()> {
    let mut found = None;
    for header in headers.get_all(COOKIE) {
        for pair in header.to_str().map_err(|_| ())?.split(';') {
            let Some((key, value)) = pair.trim().split_once('=') else {
                continue;
            };
            if key == name {
                if found.is_some() {
                    return Err(());
                }
                found = Some(value);
            }
        }
    }
    Ok(found)
}

pub(crate) fn header(name: &str, value: &str, secure: bool, max_age: Option<u64>) -> HeaderValue {
    let mut text = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax");
    if secure {
        text.push_str("; Secure");
    }
    if let Some(age) = max_age {
        text.push_str(&format!("; Max-Age={age}"));
    }
    HeaderValue::from_str(&text).expect("cookie payload is URL-safe base64")
}

pub(crate) fn private(headers: &mut HeaderMap) {
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
}
