use std::rc::Rc;

use gpui_kit::{
    App,
    component::{ActiveTheme as _, Theme, ThemeConfig, ThemeMode},
};
use serde_json::Value;

use crate::settings::{Fields, settings_file};

const THEME_FIELD: &str = "theme";
const ARDOISE: &str = include_str!("ardoise.json");
const NEIGE: &str = include_str!("neige.json");

fn bundled(json: &str) -> Rc<ThemeConfig> {
    Rc::new(serde_json::from_str(json).expect("the bundled themes are valid"))
}

fn read(fields: &Fields) -> ThemeMode {
    match fields.get(THEME_FIELD).and_then(Value::as_str) {
        Some(name) if name == ThemeMode::Light.name() => ThemeMode::Light,
        _ => ThemeMode::Dark,
    }
}

fn write(mode: ThemeMode, fields: &mut Fields) {
    fields.insert(THEME_FIELD.into(), mode.name().into());
}

pub(crate) fn install_theme(cx: &mut App) {
    let mode = settings_file().map_or(ThemeMode::Dark, |file| read(&file.read()));
    Theme::update(cx, |theme| {
        theme.dark_theme = bundled(ARDOISE);
        theme.light_theme = bundled(NEIGE);
    });
    Theme::change(mode, None, cx);
}

pub(crate) fn current_theme(cx: &App) -> ThemeMode {
    cx.theme().mode
}

pub(crate) fn switch_theme(mode: ThemeMode, cx: &mut App) {
    if mode == current_theme(cx) {
        return;
    }
    Theme::change(mode, None, cx);
    if let Some(file) = settings_file() {
        let _ = file.update(|fields| write(mode, fields));
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::{ThemeMode, try_parse_color};
    use serde_json::{Value, json};

    use super::{ARDOISE, Fields, NEIGE, bundled, read, write};

    fn fields(value: Value) -> Fields {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn ardoise_is_the_dark_theme_and_neige_the_light_one() {
        assert_eq!(bundled(ARDOISE).mode, ThemeMode::Dark);
        assert_eq!(bundled(NEIGE).mode, ThemeMode::Light);
    }

    #[test]
    fn every_color_of_both_themes_parses() {
        for json in [ARDOISE, NEIGE] {
            let theme: Value = serde_json::from_str(json).unwrap();
            for (key, color) in theme["colors"].as_object().unwrap() {
                let color = color.as_str().unwrap();
                assert!(try_parse_color(color).is_ok(), "{key}: {color}");
            }
        }
    }

    #[test]
    fn the_theme_reads_back_from_the_settings_file() {
        let mut saved = fields(json!({ "zoom": 125 }));
        write(ThemeMode::Light, &mut saved);
        assert_eq!(saved, fields(json!({ "zoom": 125, "theme": "light" })));
        assert_eq!(read(&saved), ThemeMode::Light);
        write(ThemeMode::Dark, &mut saved);
        assert_eq!(read(&saved), ThemeMode::Dark);
    }

    #[test]
    fn a_missing_or_unknown_theme_reads_as_dark() {
        for saved in [json!({}), json!({ "theme": "sepia" }), json!({ "theme": 1 })] {
            assert_eq!(read(&fields(saved)), ThemeMode::Dark);
        }
    }
}
