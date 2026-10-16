use std::sync::Arc;

use egui::epaint::text::{FontTweak, VariationCoords};
use egui::{FontData, FontDefinitions, FontFamily, FontId};

const NUNITO: &[u8] = include_bytes!("../../../assets/fonts/nunito/Nunito[wght].ttf");
const NOTO_SANS_JP: &[u8] =
    include_bytes!("../../../assets/fonts/noto-sans-jp/NotoSansJP[wght].ttf");

const REGULAR_WEIGHT: f32 = 400.0;
const BOLD_WEIGHT: f32 = 700.0;

const BOLD_FAMILY: &str = "bold";

pub fn bold_family() -> FontFamily {
    FontFamily::Name(BOLD_FAMILY.into())
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, bold_family())
}

/// 同梱フォントを登録する。どちらも可変フォントで、既定のウェイトが最も細い値なので、
/// 標準と太字のウェイトを明示して別のフォントとして登録する。
pub fn install(ctx: &egui::Context) {
    let faces = [
        ("nunito-regular", NUNITO, REGULAR_WEIGHT),
        ("nunito-bold", NUNITO, BOLD_WEIGHT),
        ("noto-sans-jp-regular", NOTO_SANS_JP, REGULAR_WEIGHT),
        ("noto-sans-jp-bold", NOTO_SANS_JP, BOLD_WEIGHT),
    ];

    let mut fonts = FontDefinitions::default();
    for (key, bytes, weight) in faces {
        let tweak = FontTweak {
            coords: VariationCoords::new([(b"wght", weight)]),
            ..Default::default()
        };
        fonts.font_data.insert(
            key.to_owned(),
            Arc::new(FontData::from_static(bytes).tweak(tweak)),
        );
    }

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .splice(
            0..0,
            [
                "nunito-regular".to_owned(),
                "noto-sans-jp-regular".to_owned(),
            ],
        );
    fonts.families.insert(
        bold_family(),
        vec!["nunito-bold".to_owned(), "noto-sans-jp-bold".to_owned()],
    );

    ctx.set_fonts(fonts);
}
