use serde::Deserialize;
use std::sync::LazyLock;

use crate::auth::AuthInfo;

#[derive(Clone, Deserialize)]
pub struct KeycloakInfo {
    provider: String,
    realm: String,
    client_id: String,
    bind_addrs: Vec<String>,
    redirect: url::Url,
}

impl From<KeycloakInfo> for AuthInfo {
    fn from(val: KeycloakInfo) -> Self {
        AuthInfo {
            auth_url: format!(
                "{}/realms/{}/protocol/openid-connect/auth",
                val.provider, val.realm
            ),
            token_url: format!(
                "{}/realms/{}/protocol/openid-connect/token",
                val.provider, val.realm
            ),
            bind_addrs: val.bind_addrs,
            redirect_addr: val.redirect,
            client_id: val.client_id,
        }
    }
}

#[derive(Clone, Deserialize)]
pub enum AuthProvider {
    Keycloak(KeycloakInfo),
}

impl From<AuthProvider> for AuthInfo {
    fn from(val: AuthProvider) -> Self {
        match val {
            AuthProvider::Keycloak(kc) => kc.into(),
        }
    }
}

#[derive(Deserialize)]
pub struct BackendConfig {
    pub backend: url::Url,
    pub auth: AuthProvider,
}

pub static AUTH_INFO_PROD: LazyLock<AuthProvider> = LazyLock::new(|| {
    let provider = "https://keycloak.omnect.conplement.cloud".to_string();
    let realm = "cp-prod".to_string();
    let client_id = "cp-cli".to_string();
    let bind_addrs = vec!["127.0.0.1:4000".to_string(), "[::1]:4000".to_string()];
    let redirect = url::Url::parse("http://localhost:4000").unwrap();

    AuthProvider::Keycloak(KeycloakInfo {
        provider,
        realm,
        client_id,
        bind_addrs,
        redirect,
    })
});
