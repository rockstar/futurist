//! Stark Future cloud API client.
//!
//! Handles authentication and firmware update discovery via
//! `https://api.starkfuture.com`.

use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://api.starkfuture.com";

/// The hardcoded AES-256 key used by the update library to generate the
/// `x-auth` header.  The plaintext is a device identifier encrypted with
/// AES/CBC/PKCS5Padding.
const UPDATE_AUTH_KEY: &[u8; 32] = b"K1YeVf4uN5gXLw00zdYLvylYwMu64aVe";

/// Device identifier used for non-Emdoor phones.  The official app sends
/// `Build.MANUFACTURER.toUpperCase()` which is `"BLACKVIEW"` for the
/// Blackview handsets shipped with the bike.
const DEVICE_ID: &str = "BLACKVIEW";

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SignInRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateResponse {
    pub vin: String,
    pub firmware: Option<Firmware>,
    pub status: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Firmware {
    pub _id: String,
    pub name: String,
    pub phone_app: Option<UpdateDetail>,
    pub phone_firmware: Option<UpdateDetail>,
    pub bike_firmware: Option<UpdateDetail>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDetail {
    pub _id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub build_version: String,
    pub version_number: String,
    pub status: String,
    pub file: FileInfo,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub key: String,
    pub bucket: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub token: Option<String>,
}

// ---------------------------------------------------------------------------
// x-auth header generation (AES-256-CBC)
// ---------------------------------------------------------------------------

/// Generate the `x-auth` header value expected by the update endpoint.
///
/// The official app computes:
///   `CryptoHelper.encrypt(DeviceInfoHelper.getId(), ID_KEY)`
///
/// which is AES/CBC/PKCS5Padding with a random 16-byte IV.  The result is
/// hex-encoded as `hex(iv) || hex(ciphertext)`.
fn generate_x_auth() -> String {
    use aes::cipher::{BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};

    type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

    let mut iv = [0u8; 16];
    getrandom::fill(&mut iv).expect("getrandom");

    let encryptor = Aes256CbcEnc::new(UPDATE_AUTH_KEY.into(), &iv.into());
    let ciphertext = encryptor.encrypt_padded_vec_mut::<Pkcs7>(DEVICE_ID.as_bytes());

    let mut out = String::with_capacity((16 + ciphertext.len()) * 2);
    for b in &iv {
        out.push_str(&format!("{:02x}", b));
    }
    for b in &ciphertext {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

// ---------------------------------------------------------------------------
// HTTP clients
// ---------------------------------------------------------------------------

/// Build the HTTP client for the main Stark API (sign-in, refresh, etc.).
fn build_api_client() -> reqwest::Client {
    use reqwest::header::{self, HeaderMap, HeaderValue};

    let mut defaults = HeaderMap::new();
    defaults.insert("clientsecret", HeaderValue::from_static(""));
    defaults.insert("app-version", HeaderValue::from_static("3.0.5"));
    defaults.insert("app-env", HeaderValue::from_static("production"));
    defaults.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

    reqwest::Client::builder()
        .user_agent("okhttp/4.12.0")
        .default_headers(defaults)
        .timeout(std::time::Duration::from_secs(130))
        .build()
        .expect("reqwest client")
}

/// Build the HTTP client for the update library (bare, no extra headers).
fn build_update_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("okhttp/4.12.0")
        .timeout(std::time::Duration::from_secs(130))
        .build()
        .expect("reqwest client")
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A thin wrapper around the Stark Future REST API.
pub struct StarkApi {
    api_client: reqwest::Client,
    update_client: reqwest::Client,
    token: String,
}

impl StarkApi {
    /// Authenticate with email + password.
    ///
    /// Returns a client that carries the session token for subsequent calls.
    pub async fn sign_in(email: &str, password: &str) -> anyhow::Result<Self> {
        let api_client = build_api_client();
        let url = format!("{API_BASE}/v1/user/sign-in");

        let resp = api_client
            .post(&url)
            .json(&SignInRequest {
                email: email.to_string(),
                password: password.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("sign-in failed (HTTP {status}): {body}");
        }

        let auth: AuthResponse = resp.json().await?;
        let token = auth
            .token
            .ok_or_else(|| anyhow::anyhow!("sign-in succeeded but no token in response"))?;

        Ok(Self {
            api_client,
            update_client: build_update_client(),
            token,
        })
    }

    /// Check for available firmware updates.
    ///
    /// The update endpoint needs both:
    /// - `x-auth`: AES-encrypted device ID (device-level auth)
    /// - `Authorization: Bearer`: user JWT (to identify which vehicle)
    pub async fn check_for_updates(&self) -> anyhow::Result<UpdateResponse> {
        let url = format!("{API_BASE}/v2/vehicles/phone");
        let x_auth = generate_x_auth();

        let resp = self
            .update_client
            .get(&url)
            .header("x-auth", x_auth)
            .header("Authorization", format!("Bearer {}", self.token))
            .query(&[("phone_type", "android"), ("app_version", "3.0.5")])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("update check failed (HTTP {status}): {body}");
        }

        Ok(resp.json().await?)
    }

    /// Download a firmware file from the given URL, returning the raw bytes.
    pub async fn download_firmware(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let resp = self
            .update_client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            anyhow::bail!("firmware download failed (HTTP {status})");
        }

        Ok(resp.bytes().await?.to_vec())
    }

    /// Refresh the session token.
    pub async fn refresh_token(&mut self) -> anyhow::Result<()> {
        let url = format!("{API_BASE}/v1/user/refresh");

        let resp = self
            .api_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("token refresh failed (HTTP {status}): {body}");
        }

        let token_resp: TokenResponse = resp.json().await?;
        if let Some(new_token) = token_resp.token {
            self.token = new_token;
        }

        Ok(())
    }
}
