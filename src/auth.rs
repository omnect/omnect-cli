use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::Notify;

use anyhow::Result;

use actix_web::{error, get, web, App, HttpServer};
use serde::Deserialize;

use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, TokenResponse,
    TokenUrl,
};

const RETRY_THRESHOLD: usize = 8;

pub trait AuthInfo: Send {
    fn auth_url(&self) -> String;
    fn token_url(&self) -> String;
    fn bind_addr(&self) -> String;
    fn redirect_addr(&self) -> String;
    fn client_id(&self) -> String;
}

#[derive(Deserialize)]
struct QueryCode {
    code: String,
}

#[get("/")]
async fn index(
    query: web::Query<QueryCode>,
    tx: web::Data<mpsc::Sender<String>>,
) -> Result<String, error::Error> {
    log::debug!("Received callback from OAuth2 service");

    match tx.send(query.code.clone()).await {
        Ok(_) => Ok(
            "Got authorization token. You can close this tab and go back to omnect-cli."
                .to_string(),
        ),
        Err(err) => {
            log::error!("channel closed upon sending code: {:?}", err);
            Err(error::ErrorBadRequest(err))
        }
    }
}

async fn redirect_server(bind_addr: String, server_setup_complete: Arc<Notify>) -> Result<String> {
    let mut retry_count: usize = 0;
    loop {
        // Logically, a oneshot channel would be sufficient here, but the actix
        // server expects the handler to be possibly called multiple times.
        let (tx, mut rx) = mpsc::channel(1);

        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(tx.clone()))
                .service(index)
        })
        .bind(bind_addr.clone())?
        .disable_signals() // actix is not the main application so it must not handle signals
        .run();

        log::debug!("Started redirect server at {:#?}", &bind_addr);

        server_setup_complete.notify_one();

        tokio::select! {
            code = rx.recv() => {
                match code {
                    Some(code) => { return Ok(code); },
                    None => {
                        log::error!("communication channel closed");
                        anyhow::bail!("error with the communication channel");
                    },
                }
            },
            Err(err) = server => {
                // error creating the server, retry; possibly we should stop
                // here after a while
                log::error!("Error serving connection: {:?}", err);
                anyhow::ensure!(retry_count < RETRY_THRESHOLD, "failed to setup web server: {}", err);

                retry_count += 1;
            },
        };
    }
}

fn get_refresh_token_from_key_ring<A: AuthInfo>(auth_info: &A) -> Option<String> {
    let entry = match keyring::Entry::new("omnect-cli", &auth_info.client_id()) {
        Ok(entry) => entry,
        Err(err) => {
            log::warn!("Failed to get entry from key ring: {}", err);
            return None;
        }
    };

    entry.get_password().ok()
}

fn store_refresh_token_in_key_ring<A: AuthInfo>(auth_info: &A, refresh_token: String) {
    let entry = match keyring::Entry::new("omnect-cli", &auth_info.client_id()) {
        Ok(entry) => entry,
        Err(err) => {
            log::warn!("Failed to store token into key ring: {}", err);
            return;
        }
    };

    if let Err(err) = entry.set_password(&refresh_token) {
        log::warn!("Failed to store token into key ring: {}", err);
    }
}

type Token =
    oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>;

async fn request_access_token<A: AuthInfo>(auth_info: &A) -> Result<Token> {
    let client = BasicClient::new(
        ClientId::new(auth_info.client_id()),
        None,
        AuthUrl::new(auth_info.auth_url()).unwrap(),
        Some(TokenUrl::new(auth_info.token_url()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(auth_info.redirect_addr()).unwrap());

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge)
        .url();

    // start the redirect server so that clients may connect to them.
    let server_setup_complete = Arc::new(Notify::new());
    let server_task = tokio::spawn(redirect_server(
        auth_info.bind_addr(),
        server_setup_complete.clone(),
    ));
    server_setup_complete.notified().await;

    log::info!("Redirecting to authentication provider.");
    log::info!(
        "Note: if the browser does not open automatically, use this link to complete login: {}",
        auth_url.to_string()
    );
    let _ = open::that(auth_url.to_string());

    let auth_code = server_task.await??;

    Ok(client
        .exchange_code(AuthorizationCode::new(auth_code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?)
}

async fn refresh_access_token<A: AuthInfo>(auth_info: &A) -> Option<Token> {
    let refresh_token = get_refresh_token_from_key_ring(auth_info)?;
    log::debug!("Found refresh token in key ring.");

    let client = BasicClient::new(
        ClientId::new(auth_info.client_id()),
        None,
        AuthUrl::new(auth_info.auth_url()).unwrap(),
        Some(TokenUrl::new(auth_info.token_url()).unwrap()),
    );

    let access_token = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(async_http_client)
        .await;

    access_token.ok()
}

pub async fn authorize<A: AuthInfo>(auth_info: &A) -> Result<oauth2::AccessToken> {
    // If there is a refresh token from previous runs, try to create our access
    // token from that. Note, that we don't store access tokens themselves as
    // they are far too short lived.
    let token = if let Some(token) = refresh_access_token(auth_info).await {
        log::debug!("Access token refresh successful.");
        token
    } else {
        log::debug!("Could not refresh access token, use authorization code flow instead.");
        request_access_token(auth_info).await?
    };

    if let Some(refresh_token) = token.refresh_token() {
        store_refresh_token_in_key_ring(auth_info, refresh_token.secret().to_string());
    }

    Ok(token.access_token().clone())
}

pub struct KeycloakInfo {
    provider: String,
    realm: String,
    client_id: String,
    bind_addr: String,
    redirect: String,
}

impl KeycloakInfo {
    pub fn new(
        provider: &str,
        realm: &str,
        client_id: &str,
        bind_addr: &str,
        redirect: &str,
    ) -> KeycloakInfo {
        KeycloakInfo {
            provider: provider.to_string(),
            realm: realm.to_string(),
            client_id: client_id.to_string(),
            bind_addr: bind_addr.to_string(),
            redirect: redirect.to_string(),
        }
    }
}

impl AuthInfo for KeycloakInfo {
    fn auth_url(&self) -> String {
        format!(
            "{}/realms/{}/protocol/openid-connect/auth",
            self.provider, self.realm
        )
    }

    fn token_url(&self) -> String {
        format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.provider, self.realm
        )
    }

    fn bind_addr(&self) -> String {
        self.bind_addr.clone()
    }

    fn redirect_addr(&self) -> String {
        self.redirect.clone()
    }

    fn client_id(&self) -> String {
        self.client_id.clone()
    }
}

lazy_static! {
    pub static ref AUTH_INFO_DEV: KeycloakInfo = KeycloakInfo::new(
        "https://keycloak.omnect.conplement.cloud",
        "cp-dev",
        "cp-development",
        "localhost:4000",
        "http://localhost:4000",
    );
    pub static ref AUTH_INFO_PROD: KeycloakInfo = KeycloakInfo::new(
        "https://keycloak.omnect.conplement.cloud",
        "cp-prod",
        "cp-production",
        "localhost:4000",
        "http://localhost:4000",
    );
}
