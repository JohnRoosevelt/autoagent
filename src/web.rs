use reqwest::Url;
#[derive(Clone, Debug)]
pub struct FetchPolicy {
    allowed_hosts: Vec<String>,
    pub max_bytes: usize,
}
#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("URL is not permitted")]
    ForbiddenUrl,
    #[error("response exceeds size limit")]
    TooLarge,
}
impl FetchPolicy {
    pub fn new(allowed_hosts: impl IntoIterator<Item = String>, max_bytes: usize) -> Self {
        Self {
            allowed_hosts: allowed_hosts
                .into_iter()
                .map(|h| h.to_ascii_lowercase())
                .collect(),
            max_bytes,
        }
    }
    pub fn validate(&self, raw: &str) -> Result<Url, WebError> {
        let url = Url::parse(raw).map_err(|_| WebError::ForbiddenUrl)?;
        let host = url.host_str().map(str::to_ascii_lowercase);
        if url.scheme() != "https"
            || url.username() != ""
            || url.password().is_some()
            || url.port().is_some()
            || !host.is_some_and(|h| self.allowed_hosts.contains(&h))
        {
            return Err(WebError::ForbiddenUrl);
        }
        Ok(url)
    }
    pub fn html_to_text(&self, bytes: &[u8]) -> Result<String, WebError> {
        if bytes.len() > self.max_bytes {
            return Err(WebError::TooLarge);
        }
        let html = String::from_utf8_lossy(bytes);
        let mut text = String::new();
        let mut tag = false;
        for c in html.chars() {
            match c {
                '<' => tag = true,
                '>' => tag = false,
                _ if !tag => text.push(c),
                _ => {}
            }
        }
        Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_ssrf_shapes_and_bounds_local_html() {
        let p = FetchPolicy::new(["example.com".into()], 20);
        assert!(p.validate("https://example.com/a").is_ok());
        assert!(p.validate("http://example.com").is_err());
        assert!(p.validate("https://127.0.0.1").is_err());
        assert!(p.validate("https://evil.example.com").is_err());
        assert_eq!(
            p.html_to_text(b"<h1>Hello</h1> world").unwrap(),
            "Hello world"
        );
        assert!(matches!(
            p.html_to_text(&[b'x'; 21]),
            Err(WebError::TooLarge)
        ));
    }
}
