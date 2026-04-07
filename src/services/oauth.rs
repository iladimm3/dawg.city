use anyhow::Result;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl,
    AuthorizationCode, CsrfToken, Scope, TokenResponse,
};
use serde::Deserialize;

pub struct GoogleOAuth {
    pub client: BasicClient,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,         // Google's unique user ID
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
}

impl GoogleOAuth {
    pub fn new() -> Result<Self> {
        let client_id = ClientId::new(
            std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set"),
        );
        let client_secret = ClientSecret::new(
            std::env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET must be set"),
        );

        let client = BasicClient::new(
            client_id,
            Some(client_secret),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(
            std::env::var("GOOGLE_REDIRECT_URI")
                .or_else(|_| std::env::var("GOOGLE_REDIRECT_URL"))
                .unwrap_or_else(|_| "http://localhost:3000/auth/google/callback".to_string()),
        )?);

        Ok(Self { client })
    }

    /// Generate the Google login URL + CSRF state
    pub fn auth_url(&self) -> (String, CsrfToken) {
        let (url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();

        (url.to_string(), csrf_token)
    }

    /// Exchange auth code for access token, then fetch user info
    pub async fn exchange_code(&self, code: String) -> Result<GoogleUserInfo> {
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(oauth2::reqwest::async_http_client)
            .await?;

        let user_info = reqwest::Client::new()
            .get("https://www.googleapis.com/oauth2/v3/userinfo")
            .bearer_auth(token.access_token().secret())
            .send()
            .await?
            .json::<GoogleUserInfo>()
            .await?;

        Ok(user_info)
    }
}
