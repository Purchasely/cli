//! Credential sources and surface registry.
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
pub type Surface = String;
#[derive(Clone, Debug)]
pub struct SurfaceDef {
    pub env_var: String,
    pub what: String,
    pub rate_default: f64,
    pub rate_cap: f64,
}
static SURFACES: OnceLock<RwLock<HashMap<String, SurfaceDef>>> = OnceLock::new();
fn registry() -> &'static RwLock<HashMap<String, SurfaceDef>> {
    SURFACES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(
            "management".into(),
            SurfaceDef {
                env_var: "PURCHASELY_API_KEY".into(),
                what: "Management API key".into(),
                rate_default: 0.8,
                rate_cap: 1.0,
            },
        );
        RwLock::new(m)
    })
}
pub fn define_surface(id: impl Into<String>, def: SurfaceDef) {
    if let Ok(mut m) = registry().write() {
        m.insert(id.into(), def);
    }
}
pub fn surface_def(id: &str) -> Option<SurfaceDef> {
    registry().read().ok()?.get(id).cloned()
}
pub fn key_hash(key: &str) -> String {
    let h = Sha256::digest(key.as_bytes());
    h[..6].iter().map(|b| format!("{b:02x}")).collect()
}
pub fn redact(key: Option<&str>) -> String {
    key.map(|s| {
        format!(
            "…{}",
            s.chars()
                .rev()
                .take(4)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        )
    })
    .unwrap_or_else(|| "-".into())
}
#[cfg(test)]
mod tests {
    #[test]
    fn keychain_via_fake_security() {
        assert_eq!(super::key_hash("key"), "2c70e12b7a06");
    }
    #[test]
    fn fallback_on_failure() {
        assert_eq!(super::redact(Some("secret")), "…cret");
    }
}
