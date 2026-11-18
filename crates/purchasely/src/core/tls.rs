//! Mozilla root certificates plus PEM certificates selected by SSL_CERT_FILE / legacy alias.
use rustls_pki_types::pem::PemObject;
use std::{collections::BTreeMap, fs};
pub fn ca_env(env: &BTreeMap<String, String>) -> Option<&str> {
    env.get("SSL_CERT_FILE")
        .or_else(|| env.get("NODE_EXTRA_CA_CERTS"))
        .map(String::as_str)
}
pub fn pem_certificate_count(pem: &[u8]) -> usize {
    rustls_pki_types::CertificateDer::pem_slice_iter(pem)
        .filter_map(Result::ok)
        .count()
}
pub fn load_extra_certs(
    path: Option<&str>,
) -> Result<Vec<rustls_pki_types::CertificateDer<'static>>, String> {
    let Some(path) = path else { return Ok(vec![]) };
    let data = fs::read(path).map_err(|e| e.to_string())?;
    let certs = rustls_pki_types::CertificateDer::pem_slice_iter(&data)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid PEM certificate: {e}"))?;
    if certs.is_empty() {
        return Err("no valid PEM certificates".into());
    }
    Ok(certs)
}
pub fn config(
    env: &BTreeMap<String, String>,
    mut warning: impl FnMut(&str),
) -> ureq::tls::TlsConfig {
    let Some(path) = ca_env(env) else {
        return ureq::tls::TlsConfig::default();
    };
    match load_extra_certs(Some(path)) {
        Ok(certs) => {
            // As NODE_EXTRA_CA_CERTS: the extra certificates extend the Mozilla roots, they do not replace them.
            let certs = webpki_root_certs::TLS_SERVER_ROOT_CERTS
                .iter()
                .chain(certs.iter())
                .map(|c| ureq::tls::Certificate::from_der(c.as_ref()).to_owned())
                .collect::<Vec<_>>();
            let roots = ureq::tls::RootCerts::new_with_certs(&certs);
            ureq::tls::TlsConfig::builder().root_certs(roots).build()
        }
        Err(_) => {
            warning(&format!(
                "Warning: could not load extra CA certificates from {path}; using default roots."
            ));
            ureq::tls::TlsConfig::default()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pem_file_extends_the_mozilla_roots() {
        let path = std::env::temp_dir().join(format!("extra-ca-{}.pem", std::process::id()));
        fs::write(
            &path,
            "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
        )
        .unwrap();
        let env = BTreeMap::from([("SSL_CERT_FILE".into(), path.to_string_lossy().into_owned())]);
        let cfg = config(&env, |_| {});
        let ureq::tls::RootCerts::Specific(certs) = cfg.root_certs() else {
            panic!("expected the Mozilla roots plus the extra certificate");
        };
        let mozilla = webpki_root_certs::TLS_SERVER_ROOT_CERTS;
        assert_eq!(certs.len(), mozilla.len() + 1);
        // Both trust paths: a public root (as webpki::TLS_SERVER_ROOTS) and the extra certificate.
        assert!(certs.iter().any(|c| c.der() == mozilla[0].as_ref()));
        assert_eq!(certs.last().unwrap().der(), [0u8]);
        let _ = fs::remove_file(path);
    }
    #[test]
    fn node_extra_ca_is_read_when_ssl_is_absent() {
        let env = BTreeMap::from([("NODE_EXTRA_CA_CERTS".into(), "legacy.pem".into())]);
        assert_eq!(ca_env(&env), Some("legacy.pem"));
    }
    #[test]
    fn unreadable_file_gives_one_warning() {
        let env = BTreeMap::from([("SSL_CERT_FILE".into(), "/missing/cert.pem".into())]);
        let mut warnings = vec![];
        let _ = config(&env, |s| warnings.push(s.to_string()));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Warning:"));
    }
}
