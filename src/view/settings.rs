use rune::tok::motion;
use rune::{Color, El, col, el, row, spacer, text, text_input};

use crate::theme::*;
use crate::App;
use super::glass;

pub(super) const THEMES: [(&str, &str); 3] = [("black", "Dark"), ("white", "White"), ("accent", "Accent")];

pub(super) fn settings(s: &App, p: Pal) -> El {
    let field = |key: &str, edit: &rune::TextEdit| {
        text_input(edit, TITLE).key(key).single_line().w_full().h(30.0).px(10.0).py(6.0)
            .radius(7.0).fill(p.ink).border(1.0, p.line).color(p.text)
            .on_input_change(|s: &mut App, _| s.apply_settings())
            .on_submit(|s: &mut App, _| s.close_settings())
    };
    let label = |s: &str| text(s.to_string()).font(SMALL).color(p.muted);
    let switch = |key: &str, name: &str, on: bool, f: fn(&mut App)| {
        let knob = el().key("knob").size(18.0, 18.0).radius(9.0).fill(p.text).absolute(if on { 18.0 } else { 2.0 }, 2.0);
        row().w_full().gap(8.0).center_y().pt(6.0).child(text(name.to_string()).font(STAGE).color(p.text)).child(spacer())
            .child(el().key(key).role("switch").label(name).checked(on).focusable(true)
                .size(38.0, 22.0).radius(11.0).fill(if on { p.go } else { p.line })
                .motion(motion::SNAPPY).child(knob)
                .on_press(f))
    };
    let mut themes = row().key("themes").w_full().h(30.0).padding(3.0).gap(2.0).radius(8.0).fill(p.ink).border(1.0, p.line);
    for (id, name) in THEMES {
        let picked = s.saved.settings.theme == id;
        themes = themes.child(row().key(format!("theme-{id}")).grow(1.0).h_full().center().radius(6.0)
            .role("radio").label(name).checked(picked)
            .fill(if picked { p.raised } else { Color::TRANSPARENT }).hover(if picked { p.raised } else { p.band })
            .border(1.0, if picked { p.line } else { Color::TRANSPARENT })
            .child(text(name).font(STAGE).color(if picked { p.text } else { p.muted }))
            .on_press(move |s: &mut App| {
                s.saved.settings.theme = id.into();
                s.persist();
            }));
    }
    glass(col().key("settings").pin(Some(62.0), None, None, Some(14.0)).w(320.0).padding(16.0).gap(8.0).radius(16.0), p)
        .child(text("Settings").font(LANE).color(p.text).mb(4.0))
        .child(label("Theme"))
        .child(themes)
        .child(label("Show merged and closed PRs from the last (days)"))
        .child(field("days", &s.days).on_edit(|s: &mut App| &mut s.days))
        .child(label("Refresh every (seconds)"))
        .child(field("interval", &s.interval).on_edit(|s: &mut App| &mut s.interval))
        .child(label("Extra search terms, like org:acme -is:draft"))
        .child(field("filter", &s.filter).on_edit(|s: &mut App| &mut s.filter))
        .child(switch("closed", "Show closed PRs", s.saved.settings.include_closed, App::toggle_closed))
        .child(switch("menubar", "Menu bar item", s.saved.settings.menu_bar, App::toggle_menu_bar))
        .child(row().w_full().gap(8.0).center_y().pt(6.0)
            .child(col().gap(2.0).child(text("Command line").font(STAGE).color(p.text)).child(label("prsito in ~/.local/bin, kept up to date")))
            .child(spacer())
            .child(row().key("install-cli").h(26.0).px(10.0).center().radius(7.0).role("button")
                .label(if s.cli { "Reinstall command line tool" } else { "Install command line tool" })
                .fill(p.ink).border(1.0, p.line).hover(p.band)
                .child(text(if s.cli { "Installed" } else { "Install" }).font(STAGE).color(if s.cli { p.muted } else { p.text }))
                .on_press(|s: &mut App| s.install_cli())))
}
