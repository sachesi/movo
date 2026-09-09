use gettextrs::{
    bind_textdomain_codeset, bindtextdomain, gettext, ngettext, setlocale, textdomain,
    LocaleCategory,
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
    substitute(tr(message), values)
}

/// Picks the plural form for `n` and substitutes it into the result, the same way `trf` does.
/// `singular` and `plural` are the English msgid/msgid_plural gettext keys the catalogs are
/// built from.
pub fn trn(singular: &'static str, plural: &'static str, n: u64) -> String {
    let template = ngettext(singular, plural, n as u32);
    substitute(&template, &[&n])
}

/// Fills placeholders into `template`. `{}` placeholders are substituted in order; `{0}`,
/// `{1}`, ... are substituted by index, which lets a translation reorder the arguments.
fn substitute(template: &str, values: &[&dyn Display]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut positional = 0usize;
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        result.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            result.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let inner = &after_open[..close];
        let index = if inner.is_empty() {
            let index = positional;
            positional += 1;
            Some(index)
        } else {
            inner.parse::<usize>().ok()
        };
        if let Some(value) = index.and_then(|index| values.get(index)) {
            result.push_str(&value.to_string());
        }
        rest = &after_open[close + 1..];
    }
    result.push_str(rest);
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

    #[test]
    fn numbered_placeholders_can_be_reordered() {
        // Exercise the substitution logic directly: going through trf would add a msgid to
        // the catalog for a string nothing in the app actually shows.
        assert_eq!(substitute("Page {1} of {0}", &[&5, &2]), "Page 2 of 5");
    }

    #[test]
    fn plural_form_follows_the_count() {
        // No catalog is bound for the "C" locale in tests, so gettext falls back to the
        // English msgid/msgid_plural pair using the default plural rule (n == 1).
        assert_eq!(trn("{} title", "{} titles", 1), "1 title");
        assert_eq!(trn("{} title", "{} titles", 5), "5 titles");
    }
}
