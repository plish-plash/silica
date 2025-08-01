use std::{borrow::Cow, path::Path};

pub use fluent_bundle::FluentArgs;
use fluent_bundle::{FluentBundle, FluentMessage, FluentResource};
use unic_langid::LanguageIdentifier;

use crate::GameError;

pub struct Message<'a>(&'a str, Option<FluentMessage<'a>>);

pub struct Localization(FluentBundle<FluentResource>);

impl Localization {
    const FALLBACK_LOCALE: &str = "en-US";

    fn create_resource(source: String) -> FluentResource {
        match FluentResource::try_new(source) {
            Ok(res) => res,
            Err((res, errors)) => {
                for error in errors {
                    log::error!("{} ({:?})", error, error.pos);
                }
                res
            }
        }
    }
    fn load_resource(
        locale: LanguageIdentifier,
    ) -> Result<(LanguageIdentifier, FluentResource), GameError> {
        let path = format!("locale/{locale}.ftl");
        let asset_path = Path::new("assets").join(&path);
        if let Ok(source) = std::fs::read_to_string(asset_path) {
            log::debug!("Loading translations from {path}");
            return Ok((locale, Self::create_resource(source)));
        }
        log::warn!(
            "No translations for {}, falling back to {}",
            locale,
            Self::FALLBACK_LOCALE
        );
        let path = format!("locale/{}.ftl", Self::FALLBACK_LOCALE);
        let asset_path = Path::new("assets").join(&path);
        if let Ok(source) = std::fs::read_to_string(asset_path) {
            log::debug!("Loading translations from {path}");
            return Ok((
                Self::FALLBACK_LOCALE.parse().unwrap(),
                Self::create_resource(source),
            ));
        }
        Err(GameError::from_string(
            "Failed to load any translation resources.".to_string(),
        ))
    }

    pub fn load() -> Result<Self, GameError> {
        let locale = silica_env::get_locale().parse()?;
        let (locale, resource) = Self::load_resource(locale)?;
        let mut bundle = FluentBundle::new(vec![locale]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .expect("failed to add translation resource to bundle");
        Ok(Localization(bundle))
    }

    pub fn message<'a>(&'a self, id: &'a str) -> Message<'a> {
        Message(id, self.0.get_message(id))
    }
    pub fn format_value<'a>(
        &'a self,
        message: &Message<'a>,
        args: Option<&FluentArgs>,
    ) -> Cow<'a, str> {
        let id = message.0;
        match message.1.as_ref() {
            Some(message) => {
                if let Some(pattern) = message.value() {
                    let mut errors = Vec::new();
                    let result = self.0.format_pattern(pattern, args, &mut errors);
                    for error in errors {
                        log::error!("{error}");
                    }
                    result
                } else {
                    log::error!("Translation message \"{id}\" has no value");
                    id.into()
                }
            }
            None => {
                log::error!("Missing translation for \"{id}\"");
                id.into()
            }
        }
    }
    pub fn format_attribute<'a>(
        &'a self,
        message: &Message<'a>,
        key: &str,
        args: Option<&FluentArgs>,
    ) -> Cow<'a, str> {
        let id = message.0;
        match message.1.as_ref() {
            Some(message) => {
                if let Some(pattern) = message.get_attribute(key) {
                    let mut errors = Vec::new();
                    let result = self.0.format_pattern(pattern.value(), args, &mut errors);
                    for error in errors {
                        log::error!("{error}");
                    }
                    result
                } else {
                    log::error!("Translation message \"{id}\" has no \"{key}\" attribute");
                    id.into()
                }
            }
            None => {
                log::error!("Missing translation for \"{id}\"");
                id.into()
            }
        }
    }
    pub fn value<'a>(&'a self, id: &'a str, args: Option<&FluentArgs>) -> Cow<'a, str> {
        let message = self.message(id);
        self.format_value(&message, args)
    }
}
