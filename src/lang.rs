//! Languages and localized strings.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {
    En,
    Zh,
}

pub const SUPPORTED: [Lang; 2] = [Lang::En, Lang::Zh];

impl Lang {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "en" | "en-us" | "english" => Some(Lang::En),
            "zh" | "zh-cn" | "chinese" | "中文" => Some(Lang::Zh),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Zh => "中文",
        }
    }

    /// Anchor/ID prefix. English keeps the plain id, Chinese gets `zh-`.
    pub fn scoped_id(self, id: &str) -> String {
        match self {
            Lang::En => id.to_string(),
            Lang::Zh => format!("zh-{id}"),
        }
    }
}

/// A string with optional English and Chinese variants.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Loc {
    pub en: Option<String>,
    pub zh: Option<String>,
}

impl Loc {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn en(value: impl Into<String>) -> Self {
        Self {
            en: Some(value.into()),
            zh: None,
        }
    }

    pub fn zh(value: impl Into<String>) -> Self {
        Self {
            en: None,
            zh: Some(value.into()),
        }
    }

    pub fn both(en: impl Into<String>, zh: impl Into<String>) -> Self {
        Self {
            en: Some(en.into()),
            zh: Some(zh.into()),
        }
    }

    pub fn get(&self, lang: Lang) -> Option<&str> {
        match lang {
            Lang::En => self.en.as_deref(),
            Lang::Zh => self.zh.as_deref(),
        }
        .filter(|value| !value.trim().is_empty())
    }

    /// Value for the requested language, falling back to the other one.
    pub fn get_or_fallback(&self, lang: Lang) -> Option<&str> {
        self.get(lang).or_else(|| match lang {
            Lang::En => self.zh.as_deref(),
            Lang::Zh => self.en.as_deref(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.get(Lang::En).is_none() && self.get(Lang::Zh).is_none()
    }

    pub fn from_yaml(value: &crate::yaml::Yaml) -> Self {
        match value {
            crate::yaml::Yaml::Str(text) => {
                let text = text.trim();
                if text.is_empty() {
                    Self::default()
                } else {
                    // A plain string is used for every locale.
                    Self {
                        en: Some(text.to_string()),
                        zh: Some(text.to_string()),
                    }
                }
            }
            crate::yaml::Yaml::Map(_) => {
                let en = value
                    .get("en")
                    .and_then(|value| value.as_str())
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty());
                let zh = value
                    .get("zh")
                    .and_then(|value| value.as_str())
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty());
                Self { en, zh }
            }
            _ => Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loc_falls_back() {
        let loc = Loc::en("hello");
        assert_eq!(loc.get(Lang::En), Some("hello"));
        assert_eq!(loc.get(Lang::Zh), None);
        assert_eq!(loc.get_or_fallback(Lang::Zh), Some("hello"));
    }

    #[test]
    fn ids_are_scoped_for_chinese() {
        assert_eq!(Lang::En.scoped_id("usage"), "usage");
        assert_eq!(Lang::Zh.scoped_id("usage"), "zh-usage");
    }
}
