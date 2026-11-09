use rune::tok::motion;
use rune::{Color, El, Icon, LoopSpec, col, el, icon, row, spacer};

use super::icons::*;
use crate::github::{self, Pr};
use crate::theme::*;
use crate::App;

/// A rail entry: a board (the data source, its columns and lanes) seen through an optional
/// filter, so subviews like "Mine, approved" share their board's fetch and lanes.
pub(crate) struct ViewSpec {
    pub(crate) name: &'static str,
    pub(crate) icon: &'static Icon,
    pub(crate) board: usize,
    pub(crate) filter: Option<fn(&Pr) -> bool>,
}

pub(crate) static VIEWS: &[ViewSpec] = &[
    ViewSpec { name: "Mine", icon: &BRANCH, board: github::MINE, filter: None },
    ViewSpec { name: "Reviews", icon: &EYE, board: github::REVIEWS, filter: None },
];

const W: f32 = 52.0;
const BTN: f32 = 32.0;
const GAP: f32 = 6.0;

pub(super) fn rail(s: &App, p: Pal, under_lights: bool) -> El {
    let mut views = col().key("tabs").gap(GAP)
        .child(el().key("tab-indicator").absolute(0.0, s.tab as f32 * (BTN + GAP)).size(BTN, BTN).radius(8.0)
            .fill(p.raised).shadow(1.0, 3.0, Color::BLACK.alpha(0.3)).motion(motion::SNAPPY));
    for (i, v) in VIEWS.iter().enumerate() {
        let on = s.tab == i;
        views = views.child(row().key(format!("tab-{i}")).size(BTN, BTN).center().radius(8.0)
            .hover(if on { Color::TRANSPARENT } else { p.raised.alpha(0.5) })
            .role("tab").label(v.name).help(v.name).checked(on)
            .child(icon(v.icon, 17.0).color(if on { p.text } else { p.muted }))
            .on_press(move |s: &mut App| {
                s.tab = i;
                s.drag = None;
                s.renaming = None;
            }));
    }
    let pending = s.live && s.pending.iter().any(|x| *x);
    let mut spin = icon(&REFRESH, 16.0).color(if pending { p.text } else { p.muted });
    if pending {
        spin = spin.looping(LoopSpec::rotation(0.9));
    }
    let refresh = row().key("refresh").size(BTN, BTN).center().radius(8.0).hover(p.raised)
        .role("button").label(if pending { "Refreshing" } else { "Refresh" }).help("Refresh (⌘R)")
        .child(spin)
        .on_press(|s: &mut App| s.refresh());
    let gear = row().key("gear").size(BTN, BTN).center().radius(8.0)
        .fill(if s.settings_open { p.raised } else { Color::TRANSPARENT }).hover(p.raised)
        .role("button").label("Settings").help("Settings")
        .child(icon(&GEAR, 16.0).color(if s.settings_open { p.text } else { p.muted }))
        .on_press(|s: &mut App| {
            if s.settings_open {
                s.close_settings();
            } else {
                s.settings_open = true;
            }
        });
    row().key("rail").h_full()
        .child(col().w(W).h_full().pt(if under_lights { 40.0 } else { 16.0 }).pb(14.0).gap(18.0).items_center().fill(p.band)
            .child(icon(&LOGO, 22.0).key("logo").color(p.text).label("prsito"))
            .child(views)
            .child(spacer())
            .child(col().gap(GAP).child(refresh).child(gear)))
        .child(el().key("edge").w(1.0).h_full().fill(p.line))
}
