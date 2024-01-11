use serde::Deserialize;

use crate::auth::AuthInfo;

#[derive(Clone, Deserialize)]
pub struct KeycloakInfo {
    provider: String,
    realm: String,
    client_id: String,
    bind_addr: String,
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
            bind_addr: val.bind_addr,
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

lazy_static::lazy_static! {
    pub static ref AUTH_INFO_PROD: AuthProvider = {
        let provider = "https://keycloak.omnect.conplement.cloud".to_string();
        let realm = "cp-prod".to_string();
        let client_id = "cp-production".to_string();
        let bind_addr = "localhost:4000".to_string();
        let redirect = url::Url::parse(&format!("http://{bind_addr}")).unwrap();

        AuthProvider::Keycloak(
            KeycloakInfo {
            provider,
            realm,
            client_id,
            bind_addr,
            redirect,
        })
    };
}
