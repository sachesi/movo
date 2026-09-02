use gettextrs::{
    bind_textdomain_codeset, bindtextdomain, gettext, setlocale, textdomain, LocaleCategory,
};
use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

const DOMAIN: &str = "movo";

static TRANSLATIONS: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();

pub fn init() {
    // No other threads exist yet, so changing the process locale is safe here.
    unsafe { setlocale(LocaleCategory::LcAll, "") };

    let locale_dir = std::env::var_os("MOVO_LOCALEDIR")
        .map(PathBuf::from)
        .or_else(development_locale_dir)
        .unwrap_or_else(|| PathBuf::from("/usr/share/locale"));

    bindtextdomain(DOMAIN, locale_dir).expect("failed to bind translation catalog");
    bind_textdomain_codeset(DOMAIN, "UTF-8").expect("failed to select UTF-8 translations");
    textdomain(DOMAIN).expect("failed to select translation domain");
}

fn development_locale_dir() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../locale");
    path.is_dir().then_some(path)
}

pub fn tr(message: &'static str) -> &'static str {
    let translations = TRANSLATIONS.get_or_init(Default::default);
    let mut translations = translations.lock().expect("translation cache poisoned");
    translations
        .entry(message)
        .or_insert_with(|| Box::leak(gettext(message).into_boxed_str()))
}

pub fn trf(message: &'static str, values: &[&dyn Display]) -> String {
    let mut parts = tr(message).split("{}");
    let mut result = parts.next().unwrap_or_default().to_string();
    for (value, part) in values.iter().zip(parts) {
        result.push_str(&value.to_string());
        result.push_str(part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_catalogs_exist() {
        let root = development_locale_dir().expect("development locale directory");
        for language in ["ru", "uk"] {
            assert!(root.join(language).join("LC_MESSAGES/movo.mo").is_file());
        }
    }

    #[test]
    fn translated_values_are_interpolated() {
        assert_eq!(trf("Season {}", &[&2]), "Season 2");
    }
}
