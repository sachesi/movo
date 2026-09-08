//! Countries a listing can be from, under every name the provider or a person
//! might use for them.
//!
//! The provider names the country of a title in the language of its interface,
//! which is not the language of the person reading the app; a name typed in
//! Ukrainian or English never matched the Russian one in the listing. Names
//! are therefore reduced to a code on both sides, so "Росія", "Russia" and
//! "Россия" are the same country. A name this table does not know is kept as
//! typed and still matches the listing letter for letter.

use serde::Serialize;

/// One country: its code, its name in each language the app speaks, and the
/// other spellings the provider or a person might use.
pub struct Country {
    pub code: &'static str,
    pub en: &'static str,
    pub ru: &'static str,
    pub uk: &'static str,
    pub aliases: &'static [&'static str],
}

impl Country {
    /// Every name this country answers to, lower-cased, the way [`canonical`]
    /// compares them; a name shared by two languages is listed once.
    pub fn names(&self) -> impl Iterator<Item = String> + '_ {
        let mut seen = Vec::new();
        [self.en, self.ru, self.uk]
            .into_iter()
            .chain(self.aliases.iter().copied())
            .map(str::to_lowercase)
            .filter(move |name| {
                if seen.contains(name) {
                    false
                } else {
                    seen.push(name.clone());
                    true
                }
            })
    }
}

/// The country as a front end lists it.
#[derive(Serialize)]
pub struct CountryEntry {
    pub code: &'static str,
    pub name: CountryName,
    pub aliases: Vec<String>,
}

#[derive(Serialize)]
pub struct CountryName {
    pub en: &'static str,
    pub ru: &'static str,
    pub uk: &'static str,
}

/// Every known country, for a front end to offer.
pub fn entries() -> Vec<CountryEntry> {
    COUNTRIES
        .iter()
        .map(|country| CountryEntry {
            code: country.code,
            name: CountryName {
                en: country.en,
                ru: country.ru,
                uk: country.uk,
            },
            aliases: country.names().collect(),
        })
        .collect()
}

/// The code of the country `name` refers to, or the name itself, trimmed and
/// lower-cased, when no country is known by it.
pub fn canonical(name: &str) -> String {
    let name = name.trim().to_lowercase();
    COUNTRIES
        .iter()
        .find(|country| country.code == name || country.names().any(|known| known == name))
        .map(|country| country.code.to_string())
        .unwrap_or(name)
}

macro_rules! country {
    ($code:literal, $en:literal, $ru:literal, $uk:literal $(, $alias:literal)*) => {
        Country {
            code: $code,
            en: $en,
            ru: $ru,
            uk: $uk,
            aliases: &[$($alias),*],
        }
    };
}

pub static COUNTRIES: &[Country] = &[
    country!("us", "USA", "США", "США", "united states", "america"),
    country!(
        "gb",
        "United Kingdom",
        "Великобритания",
        "Великобританія",
        "uk",
        "great britain",
        "britain",
        "england",
        "англия",
        "англія"
    ),
    country!("ru", "Russia", "Россия", "Росія", "russian federation"),
    country!("ua", "Ukraine", "Украина", "Україна"),
    country!("fr", "France", "Франция", "Франція"),
    country!("de", "Germany", "Германия", "Німеччина"),
    country!("it", "Italy", "Италия", "Італія"),
    country!("es", "Spain", "Испания", "Іспанія"),
    country!("jp", "Japan", "Япония", "Японія"),
    country!(
        "kr",
        "South Korea",
        "Корея Южная",
        "Південна Корея",
        "korea",
        "южная корея",
        "корея",
        "корея південна"
    ),
    country!("cn", "China", "Китай", "Китай"),
    country!("in", "India", "Индия", "Індія"),
    country!("tr", "Turkey", "Турция", "Туреччина", "türkiye"),
    country!("ca", "Canada", "Канада", "Канада"),
    country!("au", "Australia", "Австралия", "Австралія"),
    country!("br", "Brazil", "Бразилия", "Бразилія"),
    country!("mx", "Mexico", "Мексика", "Мексика"),
    country!("ar", "Argentina", "Аргентина", "Аргентина"),
    country!("pl", "Poland", "Польша", "Польща"),
    country!("cz", "Czech Republic", "Чехия", "Чехія", "czechia"),
    country!("se", "Sweden", "Швеция", "Швеція"),
    country!("no", "Norway", "Норвегия", "Норвегія"),
    country!("dk", "Denmark", "Дания", "Данія"),
    country!("fi", "Finland", "Финляндия", "Фінляндія"),
    country!("nl", "Netherlands", "Нидерланды", "Нідерланди", "holland"),
    country!("be", "Belgium", "Бельгия", "Бельгія"),
    country!("ie", "Ireland", "Ирландия", "Ірландія"),
    country!("nz", "New Zealand", "Новая Зеландия", "Нова Зеландія"),
    country!("hk", "Hong Kong", "Гонконг", "Гонконг"),
    country!("tw", "Taiwan", "Тайвань", "Тайвань"),
    country!("th", "Thailand", "Таиланд", "Таїланд"),
    country!("by", "Belarus", "Беларусь", "Білорусь", "белоруссия"),
    country!("kz", "Kazakhstan", "Казахстан", "Казахстан"),
    country!("ge", "Georgia", "Грузия", "Грузія"),
    country!("am", "Armenia", "Армения", "Вірменія"),
    country!("az", "Azerbaijan", "Азербайджан", "Азербайджан"),
    country!("uz", "Uzbekistan", "Узбекистан", "Узбекистан"),
    country!("kg", "Kyrgyzstan", "Киргизия", "Киргизстан", "кыргызстан"),
    country!("md", "Moldova", "Молдова", "Молдова", "молдавия"),
    country!("lt", "Lithuania", "Литва", "Литва"),
    country!("lv", "Latvia", "Латвия", "Латвія"),
    country!("ee", "Estonia", "Эстония", "Естонія"),
    country!("il", "Israel", "Израиль", "Ізраїль"),
    country!("ir", "Iran", "Иран", "Іран"),
    country!("eg", "Egypt", "Египет", "Єгипет"),
    country!(
        "za",
        "South Africa",
        "ЮАР",
        "ПАР",
        "южная африка",
        "південна африка"
    ),
    country!("ae", "United Arab Emirates", "ОАЭ", "ОАЕ", "uae"),
    country!(
        "sa",
        "Saudi Arabia",
        "Саудовская Аравия",
        "Саудівська Аравія"
    ),
    country!("pk", "Pakistan", "Пакистан", "Пакистан"),
    country!("ng", "Nigeria", "Нигерия", "Нігерія"),
    country!("at", "Austria", "Австрия", "Австрія"),
    country!("ch", "Switzerland", "Швейцария", "Швейцарія"),
    country!("hu", "Hungary", "Венгрия", "Угорщина"),
    country!("gr", "Greece", "Греция", "Греція"),
    country!("pt", "Portugal", "Португалия", "Португалія"),
    country!("ro", "Romania", "Румыния", "Румунія"),
    country!("bg", "Bulgaria", "Болгария", "Болгарія"),
    country!("rs", "Serbia", "Сербия", "Сербія"),
    country!("hr", "Croatia", "Хорватия", "Хорватія"),
    country!("sk", "Slovakia", "Словакия", "Словаччина"),
    country!("is", "Iceland", "Исландия", "Ісландія"),
    country!("lu", "Luxembourg", "Люксембург", "Люксембург"),
    country!("co", "Colombia", "Колумбия", "Колумбія"),
    country!("cl", "Chile", "Чили", "Чилі"),
    country!("ph", "Philippines", "Филиппины", "Філіппіни"),
    country!("id", "Indonesia", "Индонезия", "Індонезія"),
    country!("vn", "Vietnam", "Вьетнам", "В'єтнам"),
    country!("su", "USSR", "СССР", "СРСР", "soviet union"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_country_under_any_of_its_names() {
        assert_eq!(canonical("Россия"), "ru");
        assert_eq!(canonical(" росія "), "ru");
        assert_eq!(canonical("Russia"), "ru");
        assert_eq!(canonical("ru"), "ru");
        assert_eq!(canonical("Корея Южная"), "kr");
        assert_eq!(canonical("Південна Корея"), "kr");
    }

    #[test]
    fn an_unknown_name_is_kept_as_typed() {
        assert_eq!(canonical(" Атлантида "), "атлантида");
    }

    #[test]
    fn codes_and_names_are_unique() {
        let mut codes = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for country in COUNTRIES {
            assert!(codes.insert(country.code), "code {} twice", country.code);
            for name in country.names() {
                assert!(
                    names.insert(name.clone()),
                    "name {name} under two countries"
                );
            }
        }
        assert_eq!(entries().len(), COUNTRIES.len());
    }
}
