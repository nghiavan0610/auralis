//! Edge TTS WebSocket protocol implementation.
//!
//! Connects to Microsoft's Edge TTS WebSocket endpoint,
//! sends SSML synthesis requests, and returns MP3 audio data.

use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::connect_async_tls_with_config;

use super::drm;
use super::voices::default_voice_for_lang;

const WSS_BASE_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";
const SEC_MS_GEC_VERSION: &str = "1-143.0.3650.75";

/// Synthesize text to speech using Edge TTS.
///
/// Returns MP3 audio data (24kHz, 48kbitrate, mono).
pub async fn synthesize(
    text: &str,
    voice: &str,
    rate: f64,
    lang: &str,
) -> Result<Vec<u8>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Text is empty".into());
    }

    // Pick voice: user preference > default for language
    let voice = if voice.is_empty() {
        default_voice_for_lang(lang)
            .unwrap_or("en-US-EmmaMultilingualNeural")
            .to_string()
    } else {
        voice.to_string()
    };

    // Convert rate from 0.5-2.0 to percentage string like "+50%" or "-25%"
    let rate_str = if rate >= 1.0 {
        format!("+{:.0}%", (rate - 1.0) * 100.0)
    } else {
        format!("-{:.0}%", (1.0 - rate) * 100.0)
    };

    // Generate connection ID (UUID without dashes)
    let connection_id = uuid::Uuid::new_v4().to_string().replace('-', "");

    // Generate DRM token
    let sec_ms_gec = drm::generate_sec_ms_gec();

    // Build WebSocket URL
    let ws_url = format!(
        "{}?TrustedClientToken={}&ConnectionId={}&Sec-MS-GEC={}&Sec-MS-GEC-Version={}",
        WSS_BASE_URL,
        drm::TRUSTED_CLIENT_TOKEN,
        connection_id,
        sec_ms_gec,
        SEC_MS_GEC_VERSION,
    );

    // Build request with custom headers
    let mut request = request_from_url(&ws_url)?;
    let headers = request.headers_mut();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0".parse().unwrap());
    headers.insert("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".parse().unwrap());
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Cache-Control", "no-cache".parse().unwrap());

    // Connect
    let (mut ws_stream, _) = connect_async_tls_with_config(request, None, false, None)
        .await
        .map_err(|e| format!("WebSocket connect failed: {}", e))?;

    // Send config message
    let config_json = serde_json::json!({
        "context": {
            "synthesis": {
                "audio": {
                    "metadataoptions": {
                        "sentenceBoundaryEnabled": "true",
                        "wordBoundaryEnabled": "false"
                    },
                    "outputFormat": "audio-24khz-48kbitrate-mono-mp3"
                }
            }
        }
    });
    let config_msg = format!(
        "X-Timestamp:{}Z\r\n\
         Content-Type:application/json; charset=utf-8\r\n\
         Path:speech.config\r\n\r\n\
         {}",
        timestamp_string(),
        config_json
    );
    ws_stream
        .send(tokio_tungstenite::tungstenite::Message::Text(config_msg.into()))
        .await
        .map_err(|e| format!("Send config failed: {}", e))?;

    // Send SSML
    let request_id = uuid::Uuid::new_v4().to_string().replace('-', "");
    let ssml = format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>\
         <voice name='{}'>\
         <prosody pitch='+0Hz' rate='{}' volume='+0%'>\
         {}\
         </prosody>\
         </voice>\
         </speak>",
        voice, rate_str, escape_xml(text)
    );
    let ssml_msg = format!(
        "X-RequestId:{}\r\n\
         Content-Type:application/ssml+xml\r\n\
         X-Timestamp:{}Z\r\n\
         Path:ssml\r\n\r\n\
         {}",
        request_id,
        timestamp_string(),
        ssml
    );
    ws_stream
        .send(tokio_tungstenite::tungstenite::Message::Text(ssml_msg.into()))
        .await
        .map_err(|e| format!("Send SSML failed: {}", e))?;

    // Receive audio
    let mut audio_data = Vec::new();

    while let Some(msg) = ws_stream.next().await {
        let msg = msg.map_err(|e| format!("WebSocket receive error: {}", e))?;

        match msg {
            tokio_tungstenite::tungstenite::Message::Binary(data) => {
                if data.len() < 2 {
                    continue;
                }

                // First 2 bytes = header length (big-endian)
                let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
                if header_len as usize + 2 >= data.len() {
                    continue;
                }

                // Skip header + \r\n separator, extract audio payload
                let payload_start = header_len + 2 + 2; // 2 for length, 2 for \r\n
                if payload_start < data.len() {
                    audio_data.extend_from_slice(&data[payload_start..]);
                }
            }
            tokio_tungstenite::tungstenite::Message::Text(text_data) => {
                // Check for turn.end to know when synthesis is complete
                if text_data.contains("Path:turn.end") {
                    break;
                }
            }
            tokio_tungstenite::tungstenite::Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    if audio_data.is_empty() {
        return Err("No audio received from Edge TTS service".into());
    }

    tracing::info!(
        "Edge TTS: synthesized {} chars in {} -> {} bytes MP3",
        text.len(),
        lang,
        audio_data.len()
    );

    Ok(audio_data)
}

/// Build a tungstenite request from a URL string.
fn request_from_url(
    url: &str,
) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request, String> {
    url.into_client_request()
        .map_err(|e| format!("Invalid WebSocket URL: {}", e))
}

/// Generate a JavaScript-style timestamp string.
fn timestamp_string() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    // Simple UTC formatting
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year/month/day from days since epoch
    let (year, month, day) = days_to_ymd(days_since_epoch);
    let weekday = days_to_weekday(days_since_epoch);

    format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        weekday, month, day, year, hours, minutes, seconds
    )
}

fn days_to_weekday(days: i64) -> &'static str {
    match days % 7 {
        0 => "Thu",
        1 => "Fri",
        2 => "Sat",
        3 => "Sun",
        4 => "Mon",
        5 => "Tue",
        _ => "Wed",
    }
}

fn days_to_ymd(days: i64) -> (i64, &'static str, i64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let month_str = match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        _ => "Dec",
    };

    (y, month_str, d)
}

/// Escape text for XML/SSML.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
