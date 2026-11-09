use rune::tok::TextStyle;
use rune::{Color, El, el};

use crate::github;

/// The signal box: ink-blue surfaces, ivory text, and signal colours only where they carry a
/// status.
#[derive(Clone, Copy)]
pub(crate) struct Pal {
    pub(crate) ink: Color,
    pub(crate) band: Color,
    pub(crate) card: Color,
    pub(crate) card_hover: Color,
    pub(crate) raised: Color,
    pub(crate) line: Color,
    pub(crate) text: Color,
    pub(crate) muted: Color,
    pub(crate) go: Color,
    pub(crate) caution: Color,
    pub(crate) stop: Color,
    pub(crate) merged: Color,
    pub(crate) idle: Color,
    pub(crate) edge: Color,
    pub(crate) shade: Color,
}

fn hex(s: &str) -> Color {
    rune::color::hex(s).unwrap_or(Color::BLACK)
}

/// The palette for a theme setting: "accent" tints the dark surfaces with the system accent
/// colour, and an unset theme follows the system's dark mode until the first frame saves one.
pub(crate) fn pal(theme: &str, system_dark: bool, accent: Color) -> Pal {
    let tint = |base: &str, k: f32| hex(base).mix(accent, k);
    match theme {
        "accent" => Pal {
            ink: tint("#0e0f12", 0.025),
            band: tint("#141519", 0.035),
            card: tint("#1b1c21", 0.045),
            card_hover: tint("#212229", 0.055),
            raised: tint("#24252c", 0.06),
            line: tint("#2a2b33", 0.07),
            text: hex("#ecebe6"),
            muted: tint("#8d8f98", 0.05),
            go: hex("#5fc38f"),
            caution: hex("#e9b44c"),
            stop: hex("#ea6a55"),
            merged: hex("#a98cf2"),
            idle: tint("#6f717b", 0.06),
            edge: Color::TRANSPARENT,
            shade: Color::BLACK.alpha(0.22),
        },
        "black" => Pal {
            ink: hex("#000000"),
            band: hex("#0b0b0b"),
            card: hex("#1a1a1c"),
            card_hover: hex("#232326"),
            raised: hex("#1e1e21"),
            line: hex("#262626"),
            text: hex("#ededed"),
            muted: hex("#8c8c8c"),
            go: hex("#5fc38f"),
            caution: hex("#e9b44c"),
            stop: hex("#ea6a55"),
            merged: hex("#a98cf2"),
            idle: hex("#6b6b6b"),
            edge: Color::TRANSPARENT,
            shade: Color::BLACK.alpha(0.6),
        },
        "white" => pal_white(),
        _ if system_dark => pal("black", true, accent),
        _ => pal_white(),
    }
}

fn pal_white() -> Pal {
    Pal {
        ink: hex("#ffffff"),
        band: hex("#f3f3f4"),
        card: hex("#ffffff"),
        card_hover: hex("#f6f6f7"),
        raised: hex("#ffffff"),
        line: hex("#e4e4e7"),
        text: hex("#18181b"),
        muted: hex("#71717a"),
        go: hex("#1f9d5c"),
        caution: hex("#b7791f"),
        stop: hex("#d1432f"),
        merged: hex("#7c5cd6"),
        idle: hex("#a1a1aa"),
        edge: hex("#e7e7ea"),
        shade: Color::BLACK.alpha(0.05),
    }
}

pub(crate) const STAGE: TextStyle = TextStyle::new(12.5, 560.0).lh(16.0);
pub(crate) const LANE: TextStyle = TextStyle::new(13.5, 640.0).lh(18.0);
pub(crate) const TITLE: TextStyle = TextStyle::new(13.5, 520.0).lh(18.5);
pub(crate) const SMALL: TextStyle = TextStyle::new(11.5, 500.0).lh(15.0).features("'tnum' 1");

/// The signal colour of each column.
pub(crate) fn signal(p: Pal, board: usize, column: usize) -> Color {
    let mine = [p.idle, p.caution, p.stop, p.go, p.merged];
    let reviews = [p.caution, p.idle, p.go, p.merged];
    if board == github::MINE { mine[column] } else { reviews[column] }
}

pub(crate) fn lamp(key: &str, color: Color, size: f32) -> El {
    el().key(key).size(size, size).radius(size / 2.0).fill(color).shadow(0.0, size * 1.4, color.alpha(0.55))
}
