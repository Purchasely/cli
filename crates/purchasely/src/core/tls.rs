//! Extra PEM certificate selection and parsing helpers.
use rustls_pki_types::pem::PemObject;
pub fn ca_env(ssl: Option<&str>, node: Option<&str>) -> Option<String> {
    ssl.filter(|s| !s.is_empty())
        .or(node.filter(|s| !s.is_empty()))
        .map(str::to_owned)
}
pub fn pem_certificate_count(pem: &str) -> usize {
    rustls_pki_types::CertificateDer::pem_slice_iter(pem.as_bytes())
        .filter_map(Result::ok)
        .count()
}
pub fn load_extra_certs(
    path: Option<&str>,
) -> Result<Vec<rustls_pki_types::CertificateDer<'static>>, String> {
    let Some(path) = path else { return Ok(vec![]) };
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(rustls_pki_types::CertificateDer::pem_slice_iter(&data)
        .filter_map(Result::ok)
        .collect())
}
#[cfg(test)]
mod tests {
    #[test]
    fn pem_file_adds_one_certificate() {
        assert_eq!(
            super::pem_certificate_count(
                "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----"
            ),
            1
        );
    }
    #[test]
    fn node_extra_ca_is_fallback() {
        assert_eq!(
            super::ca_env(Some("SSL_CERT_FILE"), Some("NODE_EXTRA_CA_CERTS")).as_deref(),
            Some("SSL_CERT_FILE")
        );
        assert_eq!(
            super::ca_env(None, Some("NODE_EXTRA_CA_CERTS")).as_deref(),
            Some("NODE_EXTRA_CA_CERTS")
        );
    }
}
