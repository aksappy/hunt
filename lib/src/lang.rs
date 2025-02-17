#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    ENGLISH
}

impl Language {
    pub fn to(&self) -> stop_words::LANGUAGE {
        match self {
            Language::ENGLISH => stop_words::LANGUAGE::English,
        }
    }
}
