use rune::{Color, Cx, El, Icon, col, icon, row, text};

use crate::github;
use crate::theme::*;
use crate::App;
use crate::drag::Dragging;
use board::{board, gantry};
use card::{action_zones, ghost};
use detail::{detail, note};
use menu::{actions, menu};
use settings::{THEMES, settings};
use rail::rail;
pub(crate) use rail::VIEWS;
use toolbar::toolbar;

mod board;
mod card;
mod detail;
pub(crate) mod icons;
pub(crate) mod menu;
mod rail;
mod settings;
mod toolbar;

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    let dark = cx.system_dark();
    let p = pal(&s.saved.settings.theme, dark, cx.accent());
    let mut root = row().key("root").w_full().h_full().fill(p.ink)
        .on_layout_tx(move |s: &mut App, r, tx| {
            s.rects.insert("root".into(), r);
            if !THEMES.iter().any(|t| t.0 == s.saved.settings.theme) {
                s.saved.settings.theme = if dark { "black" } else { "white" }.into();
                s.persist();
            }
            tx.live_only();
        })
        .child(rail(s, p, cx.titlebar_inset() > 0.0))
        .child(col().key("stage").grow(1.0).min_w(0.0).h_full()
            .child(toolbar(s, p))
            .child(gantry(s, p))
            .child(board(s, p)));
    if let Some(d) = &s.drag {
        if let Dragging::Card(url) = &d.what && let Some(pr) = s.prs[s.board()].iter().find(|pr| &pr.url == url) {
            let pills = actions(s, pr);
            if !pills.is_empty() && let Some(zones) = action_zones(s, pr, d, &pills, p) {
                root = root.child(zones);
            }
        }
        root = root.child(ghost(s, d, p));
    }
    if s.note.is_some() {
        root = root.child(note(&s.note_text, p));
    }
    if let Some(d) = &s.detail && let Some(pr) = s.prs.iter().flatten().find(|pr| pr.url == d.url) {
        root = root.child(detail(s, d, pr, p));
    }
    if let Some(m) = &s.menu && let Some(pr) = s.prs.iter().flatten().find(|pr| pr.url == m.url) {
        root = root.child(menu(s, m, pr, p));
    }
    if s.settings_open {
        root = root.child(settings(s, p));
    }
    root
}

/// A button with an icon: `fill` is the resting surface (a faint wash of the text colour for
/// the quiet ones), and hover leans it towards `fg`.
fn action_button(key: &str, glyph: &'static Icon, label: &str, on: bool, fill: Color, fg: Color) -> El {
    let fg = if on { fg } else { fg.alpha(0.4) };
    row().key(key).h(30.0).pl(10.0).pr(12.0).gap(6.0).center().radius(8.0).role("button").label(label.to_string())
        .fill(fill).hover(if on { fill.mix(fg, 0.12) } else { fill })
        .disabled(!on)
        .child(icon(glyph, 14.0).color(fg))
        .child(text(label.to_string()).font(STAGE).color(fg))
}

/// The frosted surface popovers and dialogs sit on.
fn glass(el: El, p: Pal) -> El {
    el.fill(p.raised.alpha(0.78)).backdrop_blur(24.0).border(1.0, p.text.alpha(0.08))
        .shadow(16.0, 40.0, Color::BLACK.alpha(0.35))
}

fn columns(s: &App) -> &'static [&'static str] {
    github::COLUMNS[s.board()]
}
