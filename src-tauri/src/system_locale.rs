//! The system's regional format for dates, as a BCP 47 tag.
//!
//! The webview's `Intl` default follows the interface language, while native
//! apps format dates by the regional format (`LC_TIME` on Linux, the Region
//! setting on Windows and macOS). The frontend asks for it once at startup.

#![cfg_attr(windows, allow(unsafe_code))]

/// The tag from one locale variable such as `ru_RU.UTF-8@euro`: the codeset
/// and modifier are dropped and `_` becomes `-`. `None` for an empty value,
/// `C`, and `POSIX`.
#[cfg(any(all(unix, not(target_os = "macos")), test))]
fn parse_posix_locale(value: &str) -> Option<String> {
    let name = value
        .split(['.', '@'])
        .next()
        .map(str::trim)
        .unwrap_or_default();
    if name.is_empty() || name == "C" || name == "POSIX" {
        return None;
    }
    Some(name.replace('_', "-"))
}

/// The first of `LC_ALL`, `LC_TIME`, `LANG` that names a locale, as POSIX
/// lets them override each other.
#[cfg(all(unix, not(target_os = "macos")))]
fn read_system_locale() -> String {
    for variable in ["LC_ALL", "LC_TIME", "LANG"] {
        if let Ok(value) = std::env::var(variable)
            && !value.trim().is_empty()
        {
            // A set variable decides, even when it names `C`.
            return parse_posix_locale(&value).unwrap_or_default();
        }
    }
    String::new()
}

#[cfg(windows)]
fn read_system_locale() -> String {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;

    let mut buffer = [0u16; 85];
    // SAFETY: the buffer is LOCALE_NAME_MAX_LENGTH long and the call writes
    // at most that many units, with a terminating nul.
    let length = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    let length = usize::try_from(length).unwrap_or(0).saturating_sub(1);
    match buffer.get(..length) {
        Some(name) => String::from_utf16_lossy(name),
        None => String::new(),
    }
}

// The identifier is the regional format, such as `en_RU` for English with the
// region Russia. The webview's ICU may lack data for such a tag; then
// `Intl` falls back to the language, which the frontend checks.
#[cfg(target_os = "macos")]
fn read_system_locale() -> String {
    use objc2_foundation::NSLocale;

    NSLocale::currentLocale()
        .localeIdentifier()
        .to_string()
        .replace('_', "-")
}

/// Tauri command `system_locale`: the BCP 47 tag of the regional format for
/// dates, or an empty string when the system names none.
#[tauri::command]
pub fn system_locale() -> String {
    read_system_locale()
}

#[cfg(test)]
mod tests {
    use super::parse_posix_locale;

    #[test]
    fn parses_a_posix_locale() {
        assert_eq!(parse_posix_locale("ru_RU.UTF-8").as_deref(), Some("ru-RU"));
        assert_eq!(parse_posix_locale("de_DE@euro").as_deref(), Some("de-DE"));
        assert_eq!(
            parse_posix_locale("sr_RS.UTF-8@latin").as_deref(),
            Some("sr-RS")
        );
        assert_eq!(parse_posix_locale("de").as_deref(), Some("de"));
        assert_eq!(parse_posix_locale("en_US").as_deref(), Some("en-US"));
    }

    #[test]
    fn names_no_locale_for_c_posix_and_empty() {
        assert_eq!(parse_posix_locale("C"), None);
        assert_eq!(parse_posix_locale("POSIX"), None);
        assert_eq!(parse_posix_locale("C.UTF-8"), None);
        assert_eq!(parse_posix_locale(""), None);
        assert_eq!(parse_posix_locale("  "), None);
    }
}
