use std::net::ToSocketAddrs;

use tokio::sync::{mpsc, oneshot};

use anyhow::Result;

use actix_web::{error, get, web, App, HttpServer};
use serde::Deserialize;

use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, TokenResponse,
    TokenUrl,
};

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
        Ok(_) => {
            println!("query.code: {}", query.code);
            Ok(
                "Got authorization token. You can close this tab and go back to omnect-cli."
                    .to_string(),
            )
        }
        Err(err) => {
            log::error!("channel closed upon sending code: {:?}", err);
            Err(error::ErrorBadRequest(err))
        }
    }
}

enum RedirectServerState {
    Running,
    Failure(String),
}

async fn redirect_server<A: ToSocketAddrs>(
    bind_addrs: Vec<A>,
    server_setup_complete: oneshot::Sender<RedirectServerState>,
) -> Result<String> {
    // Logically, a oneshot channel would be sufficient here, but the actix
    // server expects the handler to be possibly called multiple times.
    let (tx, mut rx) = mpsc::channel(1);

    let server_builder = {
        // workaround with quirk of lifetimes of the Fn context. We put the
        // mutable builder into a separate scope to ensure it is only used
        // for construction.

        let mut builder = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(tx.clone()))
                .service(index)
        });

        for addr in &bind_addrs {
            builder = match builder.bind(addr) {
                Ok(builder) => builder,
                Err(err) => {
                    // this error is already unrecoverable, there is no sense in
                    // handling an additional error in send.
                    let _ =
                        server_setup_complete.send(RedirectServerState::Failure(format!("{err}")));
                    anyhow::bail!(err);
                }
            }
        }

        builder = builder.disable_signals(); // actix is not the main application so it must not handle signals

        builder
    };

    let server = server_builder.run();

    let addr_repr = &bind_addrs
        .iter()
        .flat_map(|addrs| addrs.to_socket_addrs().unwrap().map(|x| x.to_string()))
        .collect::<Vec<_>>()
        .join(", ");

    log::debug!("Started redirect server at {addr_repr}",);

    if server_setup_complete
        .send(RedirectServerState::Running)
        .is_err()
    {
        anyhow::bail!("could not send server up message.")
    }

    tokio::select! {
        code = rx.recv() => {
            match code {
                Some(code) => { Ok(code) },
                None => {
                    log::error!("communication channel closed");
                    anyhow::bail!("error with the communication channel");
                },
            }
        },
        Err(err) = server => {
            anyhow::bail!("error serving connection: {:?}", err);
        },
    }
}

fn get_refresh_token_from_key_ring(auth_info: &AuthInfo) -> Option<String> {
    let entry = match keyring::Entry::new("omnect-cli", &auth_info.client_id) {
        Ok(entry) => entry,
        Err(err) => {
            log::warn!("Failed to get entry from key ring: {}", err);
            return None;
        }
    };

    entry.get_password().ok()
}

fn store_refresh_token_in_key_ring(auth_info: &AuthInfo, refresh_token: String) {
    let entry = match keyring::Entry::new("omnect-cli", &auth_info.client_id) {
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
async fn request_access_token(auth_info: &AuthInfo) -> Result<Token> {
    let client = BasicClient::new(
        ClientId::new(auth_info.client_id.clone()),
        None,
        AuthUrl::new(auth_info.auth_url.clone()).unwrap(),
        Some(TokenUrl::new(auth_info.token_url.clone()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(auth_info.redirect_addr.to_string()).unwrap());

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge)
        .url();

    // start the redirect server so that clients may connect to them.
    let (server_setup_complete_tx, server_setup_complete_rx) = oneshot::channel();
    let server_task = tokio::spawn(redirect_server(
        auth_info.bind_addrs.clone(),
        server_setup_complete_tx,
    ));

    match server_setup_complete_rx.await {
        Err(e) => {
            anyhow::bail!("failed to setup redirect server: {e}");
        }
        Ok(RedirectServerState::Failure(error)) => {
            anyhow::bail!("failed to setup redirect server: {error}");
        }
        Ok(_) => {}
    }

    log::info!("Redirecting to authentication provider.");
    log::info!(
        "Note: if the browser does not open automatically, use this link to complete login: {}",
        auth_url.to_string()
    );

    // let _ = open::that(auth_url.to_string());

    let auth_code = server_task.await??;

    Ok(client
        .exchange_code(AuthorizationCode::new(auth_code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?)
}

async fn refresh_access_token(auth_info: &AuthInfo) -> Option<Token> {
    let refresh_token = get_refresh_token_from_key_ring(auth_info)?;
    log::debug!("Found refresh token in key ring.");

    let client = BasicClient::new(
        ClientId::new(auth_info.client_id.clone()),
        None,
        AuthUrl::new(auth_info.auth_url.clone()).unwrap(),
        Some(TokenUrl::new(auth_info.token_url.clone()).unwrap()),
    );

    let access_token = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(async_http_client)
        .await;

    access_token.ok()
}

pub async fn authorize<A>(auth_provider: A) -> Result<oauth2::AccessToken>
where
    A: Into<AuthInfo>,
{
    let mut auth_info: AuthInfo = auth_provider.into();

    if let Ok("true") | Ok("1") = std::env::var("CONTAINERIZED").as_deref() {
        auth_info.bind_addrs = vec!["0.0.0.0:4000".to_string()];
    }

    // If there is a refresh token from previous runs, try to create our access
    // token from that. Note, that we don't store access tokens themselves as
    // they are far too short lived.
    let token = if let Some(token) = refresh_access_token(&auth_info).await {
        log::debug!("Access token refresh successful.");
        token
    } else {
        log::debug!("Could not refresh access token, use authorization code flow instead.");
        request_access_token(&auth_info).await?
    };

    println!("token: {:#?}", token.access_token().secret());

    if let Some(refresh_token) = token.refresh_token() {
        store_refresh_token_in_key_ring(&auth_info, refresh_token.secret().to_string());
    }

    Ok(token.access_token().clone())
}

pub struct AuthInfo {
    pub auth_url: String,
    pub token_url: String,
    pub bind_addrs: Vec<String>,
    pub redirect_addr: url::Url,
    pub client_id: String,
}
