//! Curated list of Edge TTS voices for supported languages.
//!
//! Each voice has a name, language code, and gender.
//! These are high-quality Neural voices from Microsoft.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EdgeVoice {
    pub name: String,
    pub lang: String,
    pub gender: String,
}

/// Get the default Edge TTS voice for a given language code.
pub fn default_voice_for_lang(lang: &str) -> Option<&'static str> {
    match lang {
        "en" => Some("en-US-EmmaMultilingualNeural"),
        "vi" => Some("vi-VN-HoaiMyNeural"),
        "es" => Some("es-ES-ElviraNeural"),
        "fr" => Some("fr-FR-DeniseNeural"),
        "de" => Some("de-DE-KatjaNeural"),
        "zh" => Some("zh-CN-XiaoxiaoNeural"),
        "ja" => Some("ja-JP-NanamiNeural"),
        "ko" => Some("ko-KR-SunHiNeural"),
        "pt" => Some("pt-BR-FranciscaNeural"),
        "ru" => Some("ru-RU-SvetlanaNeural"),
        "ar" => Some("ar-SA-ZariyahNeural"),
        "hi" => Some("hi-IN-SwaraNeural"),
        _ => None,
    }
}

/// Get all available Edge TTS voices.
pub fn all_voices() -> Vec<EdgeVoice> {
    vec![
        // English
        EdgeVoice { name: "en-US-EmmaMultilingualNeural".into(), lang: "en".into(), gender: "Female".into() },
        EdgeVoice { name: "en-US-JennyMultilingualNeural".into(), lang: "en".into(), gender: "Female".into() },
        EdgeVoice { name: "en-US-GuyNeural".into(), lang: "en".into(), gender: "Male".into() },
        EdgeVoice { name: "en-US-ChristopherNeural".into(), lang: "en".into(), gender: "Male".into() },
        // Vietnamese
        EdgeVoice { name: "vi-VN-HoaiMyNeural".into(), lang: "vi".into(), gender: "Female".into() },
        EdgeVoice { name: "vi-VN-NamMinhNeural".into(), lang: "vi".into(), gender: "Male".into() },
        // Spanish
        EdgeVoice { name: "es-ES-ElviraNeural".into(), lang: "es".into(), gender: "Female".into() },
        EdgeVoice { name: "es-ES-AlvaroNeural".into(), lang: "es".into(), gender: "Male".into() },
        // French
        EdgeVoice { name: "fr-FR-DeniseNeural".into(), lang: "fr".into(), gender: "Female".into() },
        EdgeVoice { name: "fr-FR-HenriNeural".into(), lang: "fr".into(), gender: "Male".into() },
        // German
        EdgeVoice { name: "de-DE-KatjaNeural".into(), lang: "de".into(), gender: "Female".into() },
        EdgeVoice { name: "de-DE-ConradNeural".into(), lang: "de".into(), gender: "Male".into() },
        // Chinese
        EdgeVoice { name: "zh-CN-XiaoxiaoNeural".into(), lang: "zh".into(), gender: "Female".into() },
        EdgeVoice { name: "zh-CN-YunxiNeural".into(), lang: "zh".into(), gender: "Male".into() },
        // Japanese
        EdgeVoice { name: "ja-JP-NanamiNeural".into(), lang: "ja".into(), gender: "Female".into() },
        EdgeVoice { name: "ja-JP-KeitaNeural".into(), lang: "ja".into(), gender: "Male".into() },
        // Korean
        EdgeVoice { name: "ko-KR-SunHiNeural".into(), lang: "ko".into(), gender: "Female".into() },
        EdgeVoice { name: "ko-KR-InJoonNeural".into(), lang: "ko".into(), gender: "Male".into() },
        // Portuguese
        EdgeVoice { name: "pt-BR-FranciscaNeural".into(), lang: "pt".into(), gender: "Female".into() },
        EdgeVoice { name: "pt-BR-AntonioNeural".into(), lang: "pt".into(), gender: "Male".into() },
        // Russian
        EdgeVoice { name: "ru-RU-SvetlanaNeural".into(), lang: "ru".into(), gender: "Female".into() },
        EdgeVoice { name: "ru-RU-DmitryNeural".into(), lang: "ru".into(), gender: "Male".into() },
        // Arabic
        EdgeVoice { name: "ar-SA-ZariyahNeural".into(), lang: "ar".into(), gender: "Female".into() },
        EdgeVoice { name: "ar-SA-HamedNeural".into(), lang: "ar".into(), gender: "Male".into() },
        // Hindi
        EdgeVoice { name: "hi-IN-SwaraNeural".into(), lang: "hi".into(), gender: "Female".into() },
        EdgeVoice { name: "hi-IN-MadhurNeural".into(), lang: "hi".into(), gender: "Male".into() },
    ]
}
