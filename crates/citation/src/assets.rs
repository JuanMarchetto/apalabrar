//! Phase 5.2-impl/parser — bundled CSL XML assets + lookup helpers.
//!
//! Style + locale XMLs are compiled into the binary via
//! `include_str!()`. `style_xml(id)` and `locale_xml(lang)` return
//! `None` for unknown ids — public API surfaces those as
//! [`Error::UnknownStyle`] / [`Error::UnknownLocale`].

const APA: &str = include_str!("../assets/styles/apa.csl");
const IEEE: &str = include_str!("../assets/styles/ieee.csl");
const MLA: &str = include_str!("../assets/styles/mla.csl");
const AMA: &str = include_str!("../assets/styles/ama.csl");
const CHICAGO: &str = include_str!("../assets/styles/chicago-notes-bibliography.csl");
const HARVARD: &str = include_str!("../assets/styles/harvard.csl");
const NATURE: &str = include_str!("../assets/styles/nature.csl");
const SCIENCE: &str = include_str!("../assets/styles/science.csl");
const CELL: &str = include_str!("../assets/styles/cell.csl");
const PLOS: &str = include_str!("../assets/styles/plos.csl");

const EN_US: &str = include_str!("../assets/locales/locales-en-US.xml");
const EN_GB: &str = include_str!("../assets/locales/locales-en-GB.xml");
const ES_ES: &str = include_str!("../assets/locales/locales-es-ES.xml");
const PT_BR: &str = include_str!("../assets/locales/locales-pt-BR.xml");
const DE_DE: &str = include_str!("../assets/locales/locales-de-DE.xml");
const FR_FR: &str = include_str!("../assets/locales/locales-fr-FR.xml");
const ZH_CN: &str = include_str!("../assets/locales/locales-zh-CN.xml");
const RU_RU: &str = include_str!("../assets/locales/locales-ru-RU.xml");

/// Look up a bundled style by id. Returns the raw CSL XML.
pub fn style_xml(id: &str) -> Option<&'static str> {
    match id {
        "apa" => Some(APA),
        "ieee" => Some(IEEE),
        "mla" => Some(MLA),
        "ama" => Some(AMA),
        "chicago-notes-bibliography" => Some(CHICAGO),
        "harvard" => Some(HARVARD),
        "nature" => Some(NATURE),
        "science" => Some(SCIENCE),
        "cell" => Some(CELL),
        "plos" => Some(PLOS),
        _ => None,
    }
}

/// Look up a bundled locale by `xml:lang` code. Returns the raw
/// CSL-locale XML.
pub fn locale_xml(lang: &str) -> Option<&'static str> {
    match lang {
        "en-US" => Some(EN_US),
        "en-GB" => Some(EN_GB),
        "es-ES" => Some(ES_ES),
        "pt-BR" => Some(PT_BR),
        "de-DE" => Some(DE_DE),
        "fr-FR" => Some(FR_FR),
        "zh-CN" => Some(ZH_CN),
        "ru-RU" => Some(RU_RU),
        _ => None,
    }
}

/// Iterator over the bundled style ids — useful for diagnostics
/// and `cargo doc` listings.
pub fn bundled_style_ids() -> &'static [&'static str] {
    &[
        "apa",
        "ieee",
        "mla",
        "ama",
        "chicago-notes-bibliography",
        "harvard",
        "nature",
        "science",
        "cell",
        "plos",
    ]
}

/// Iterator over the bundled locale ids.
pub fn bundled_locale_ids() -> &'static [&'static str] {
    &[
        "en-US", "en-GB", "es-ES", "pt-BR", "de-DE", "fr-FR", "zh-CN", "ru-RU",
    ]
}
