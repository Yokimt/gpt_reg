use reqwest::{Client, Method, Response, StatusCode};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{DeserializeOwned, Error as DeError, Visitor},
};
use std::{error::Error as StdError, fmt};

#[derive(Clone, Debug)]
pub struct MoEmailClient {
    base_url: String,
    api_key: Option<String>,
    http: Client,
}

impl MoEmailClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            http: Client::new(),
        }
    }

    pub fn with_api_key(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: Some(api_key.into()),
            http: Client::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    pub fn set_api_key(&mut self, api_key: impl Into<String>) {
        self.api_key = Some(api_key.into());
    }

    pub fn clear_api_key(&mut self) {
        self.api_key = None;
    }

    pub fn shared_email_url(&self, token: &str) -> String {
        format!("{}/shared/{}", self.base_url.trim_end_matches('/'), token)
    }

    pub fn shared_message_url(&self, token: &str) -> String {
        format!(
            "{}/shared/message/{}",
            self.base_url.trim_end_matches('/'),
            token
        )
    }

    pub async fn get_config(&self) -> Result<SystemConfig, MoEmailError> {
        self.get_json("/api/config", None).await
    }

    pub async fn generate_email(
        &self,
        request: GenerateEmailRequest,
    ) -> Result<GeneratedEmail, MoEmailError> {
        self.post_json("/api/emails/generate", &request).await
    }

    pub async fn list_emails(
        &self,
        cursor: Option<&str>,
    ) -> Result<EmailListResponse, MoEmailError> {
        let query = cursor.map(|cursor| vec![("cursor", cursor.to_string())]);
        self.get_json(
            "/api/emails",
            query.as_ref().map(|params| params.as_slice()),
        )
        .await
    }

    pub async fn list_email_messages(
        &self,
        email_id: &str,
        cursor: Option<&str>,
    ) -> Result<MessageListResponse, MoEmailError> {
        let query = cursor.map(|cursor| vec![("cursor", cursor.to_string())]);
        self.get_json(
            &format!("/api/emails/{email_id}"),
            query.as_ref().map(|params| params.as_slice()),
        )
        .await
    }

    pub async fn delete_email(&self, email_id: &str) -> Result<OperationResult, MoEmailError> {
        self.delete_json(&format!("/api/emails/{email_id}")).await
    }

    pub async fn get_message(
        &self,
        email_id: &str,
        message_id: &str,
    ) -> Result<MessageDetail, MoEmailError> {
        let response: MessageDetailEnvelope = self
            .get_json(&format!("/api/emails/{email_id}/{message_id}"), None)
            .await?;
        Ok(response.message)
    }

    pub async fn create_email_share(
        &self,
        email_id: &str,
        request: ShareRequest,
    ) -> Result<EmailShare, MoEmailError> {
        self.post_json(&format!("/api/emails/{email_id}/share"), &request)
            .await
    }

    pub async fn list_email_shares(
        &self,
        email_id: &str,
    ) -> Result<EmailShareListResponse, MoEmailError> {
        self.get_json(&format!("/api/emails/{email_id}/share"), None)
            .await
    }

    pub async fn delete_email_share(
        &self,
        email_id: &str,
        share_id: &str,
    ) -> Result<OperationResult, MoEmailError> {
        self.delete_json(&format!("/api/emails/{email_id}/share/{share_id}"))
            .await
    }

    pub async fn create_message_share(
        &self,
        email_id: &str,
        message_id: &str,
        request: ShareRequest,
    ) -> Result<MessageShare, MoEmailError> {
        self.post_json(
            &format!("/api/emails/{email_id}/messages/{message_id}/share"),
            &request,
        )
        .await
    }

    pub async fn list_message_shares(
        &self,
        email_id: &str,
        message_id: &str,
    ) -> Result<MessageShareListResponse, MoEmailError> {
        self.get_json(
            &format!("/api/emails/{email_id}/messages/{message_id}/share"),
            None,
        )
        .await
    }

    pub async fn delete_message_share(
        &self,
        email_id: &str,
        message_id: &str,
        share_id: &str,
    ) -> Result<OperationResult, MoEmailError> {
        self.delete_json(&format!(
            "/api/emails/{email_id}/messages/{message_id}/share/{share_id}"
        ))
        .await
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let mut builder = self.http.request(method, self.url(path));
        if let Some(api_key) = &self.api_key {
            builder = builder.header("X-API-Key", api_key);
        }
        builder.header(reqwest::header::ACCEPT, "application/json")
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: Option<&[(&str, String)]>,
    ) -> Result<T, MoEmailError> {
        let mut request = self.request(Method::GET, path);
        if let Some(query) = query {
            request = request.query(query);
        }
        Self::parse_response(request.send().await?).await
    }

    async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, MoEmailError> {
        Self::parse_response(self.request(Method::POST, path).json(body).send().await?).await
    }

    async fn delete_json(&self, path: &str) -> Result<OperationResult, MoEmailError> {
        let response = self.request(Method::DELETE, path).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(MoEmailError::Api { status, body });
        }

        if body.trim().is_empty() {
            return Ok(OperationResult { success: true });
        }

        serde_json::from_str(&body).map_err(MoEmailError::Json)
    }

    async fn parse_response<T: DeserializeOwned>(response: Response) -> Result<T, MoEmailError> {
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(MoEmailError::Api { status, body });
        }

        serde_json::from_str(&body).map_err(MoEmailError::Json)
    }
}

#[derive(Debug)]
pub enum MoEmailError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    Api { status: StatusCode, body: String },
}

impl fmt::Display for MoEmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => write!(f, "request failed: {error}"),
            Self::Json(error) => write!(f, "invalid JSON response: {error}"),
            Self::Api { status, body } => {
                if body.trim().is_empty() {
                    write!(f, "API returned status {status}")
                } else {
                    write!(f, "API returned status {status}: {body}")
                }
            }
        }
    }
}

impl StdError for MoEmailError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Api { .. } => None,
        }
    }
}

impl From<reqwest::Error> for MoEmailError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<serde_json::Error> for MoEmailError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Civilian,
    Knight,
    Duke,
}

impl<'de> Deserialize<'de> for UserRole {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.to_ascii_lowercase().as_str() {
            "civilian" => Ok(Self::Civilian),
            "knight" => Ok(Self::Knight),
            "duke" => Ok(Self::Duke),
            _ => Err(DeError::unknown_variant(
                &value,
                ["CIVILIAN", "KNIGHT", "DUKE"].as_slice(),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfig {
    pub default_role: UserRole,
    pub email_domains: String,
    pub admin_contact: String,
    #[serde(deserialize_with = "deserialize_string_or_u64")]
    pub max_emails: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GenerateEmailRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_time: Option<u64>,
    pub domain: String,
}

impl GenerateEmailRequest {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            name: None,
            expiry_time: None,
            domain: domain.into(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn expiry_time(mut self, expiry_time: u64) -> Self {
        self.expiry_time = Some(expiry_time);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedEmail {
    pub id: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailSummary {
    pub id: String,
    pub address: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailListResponse {
    pub emails: Vec<EmailSummary>,
    pub next_cursor: Option<String>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageSummary {
    pub id: String,
    #[serde(rename = "from_address")]
    pub from_address: String,
    pub subject: String,
    #[serde(rename = "received_at", deserialize_with = "deserialize_i64_or_string")]
    pub received_at: i64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessageListResponse {
    pub messages: Vec<MessageSummary>,
    pub next_cursor: Option<String>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageDetail {
    pub id: String,
    #[serde(rename = "from_address")]
    pub from_address: String,
    pub subject: String,
    pub content: String,
    pub html: String,
    #[serde(rename = "received_at", deserialize_with = "deserialize_i64_or_string")]
    pub received_at: i64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct MessageDetailEnvelope {
    message: MessageDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShareRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
}

impl ShareRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn expires_in(mut self, expires_in: u64) -> Self {
        self.expires_in = Some(expires_in);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailShare {
    pub id: String,
    pub email_id: String,
    pub token: String,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailShareListResponse {
    pub shares: Vec<EmailShare>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessageShare {
    pub id: String,
    pub message_id: String,
    pub token: String,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessageShareListResponse {
    pub shares: Vec<MessageShare>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationResult {
    pub success: bool,
}

fn deserialize_string_or_u64<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrNumberVisitor;

    impl<'de> Visitor<'de> for StringOrNumberVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string or unsigned integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(value.to_owned())
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(value.to_string())
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            if value < 0 {
                return Err(E::custom("expected a non-negative integer"));
            }

            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumberVisitor)
}

fn deserialize_i64_or_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    struct TimestampVisitor;

    impl<'de> Visitor<'de> for TimestampVisitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an integer timestamp or a numeric string")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            if value > i64::MAX as u64 {
                return Err(E::custom("timestamp is too large"));
            }

            Ok(value as i64)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            value.parse::<i64>().map_err(E::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            value.parse::<i64>().map_err(E::custom)
        }
    }

    deserializer.deserialize_any(TimestampVisitor)
}
