use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::Config;

const SANDBOX_BASE: &str = "https://sandbox.safaricom.co.ke";
const PRODUCTION_BASE: &str = "https://api.safaricom.co.ke";

fn base_url(config: &Config) -> &'static str {
    if config.mpesa_env.as_deref() == Some("production") {
        PRODUCTION_BASE
    } else {
        SANDBOX_BASE
    }
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StkPushResponse {
    #[serde(rename = "MerchantRequestID")]
    pub merchant_request_id: String,
    #[serde(rename = "CheckoutRequestID")]
    pub checkout_request_id: String,
    #[serde(rename = "ResponseCode")]
    pub response_code: String,
    #[serde(rename = "ResponseDescription")]
    pub response_description: String,
    #[serde(rename = "CustomerMessage")]
    pub customer_message: String,
}

/// Fetch OAuth access token from Daraja.
pub async fn get_access_token(config: &Config) -> Result<String> {
    let consumer_key = config
        .mpesa_consumer_key
        .as_deref()
        .ok_or_else(|| anyhow!("MPESA_CONSUMER_KEY not set"))?;
    let consumer_secret = config
        .mpesa_consumer_secret
        .as_deref()
        .ok_or_else(|| anyhow!("MPESA_CONSUMER_SECRET not set"))?;

    let credentials = B64.encode(format!("{consumer_key}:{consumer_secret}"));

    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/oauth/v1/generate?grant_type=client_credentials",
            base_url(config)
        ))
        .header("Authorization", format!("Basic {credentials}"))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("Failed to fetch M-Pesa access token")?;

    let body: AccessTokenResponse = resp
        .json()
        .await
        .context("Failed to parse M-Pesa access token response")?;

    Ok(body.access_token)
}

/// Initiate Lipa Na M-Pesa STK Push.
pub async fn stk_push(
    config: &Config,
    phone: &str,
    amount: u32,
    account_ref: &str,
) -> Result<StkPushResponse> {
    let shortcode = config
        .mpesa_shortcode
        .as_deref()
        .ok_or_else(|| anyhow!("MPESA_SHORTCODE not set"))?;
    let passkey = config
        .mpesa_passkey
        .as_deref()
        .ok_or_else(|| anyhow!("MPESA_PASSKEY not set"))?;
    let callback_url = config
        .mpesa_callback_url
        .as_deref()
        .ok_or_else(|| anyhow!("MPESA_CALLBACK_URL not set"))?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let password = B64.encode(format!("{shortcode}{passkey}{timestamp}"));

    let token = get_access_token(config).await?;

    let payload = serde_json::json!({
        "BusinessShortCode": shortcode,
        "Password": password,
        "Timestamp": timestamp,
        "TransactionType": "CustomerPayBillOnline",
        "Amount": amount,
        "PartyA": phone,
        "PartyB": shortcode,
        "PhoneNumber": phone,
        "CallBackURL": callback_url,
        "AccountReference": account_ref,
        "TransactionDesc": "PolyPulse deposit",
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "{}/mpesa/stkpush/v1/processrequest",
            base_url(config)
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .context("STK Push request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("STK Push failed ({status}): {body}"));
    }

    let result: StkPushResponse = resp
        .json()
        .await
        .context("Failed to parse STK Push response")?;

    Ok(result)
}

/// Normalise phone number to 2547XXXXXXXX format.
pub fn normalise_phone(phone: &str) -> Option<String> {
    let phone = phone.replace([' ', '-'], "");
    let phone = phone.trim_start_matches('+');

    if phone.starts_with("07") && phone.len() == 10 {
        return Some(format!("254{}", &phone[1..]));
    }
    if phone.starts_with("01") && phone.len() == 10 {
        return Some(format!("254{}", &phone[1..]));
    }
    if phone.starts_with("2547") && phone.len() == 12 {
        return Some(phone.to_string());
    }
    if phone.starts_with("2541") && phone.len() == 12 {
        return Some(phone.to_string());
    }
    None
}
