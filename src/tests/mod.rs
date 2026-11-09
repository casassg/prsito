use super::*;
use crate::github::{Ci, Review};
use crate::store::{INBOX, Lanes};
use rune::{Env, Runtime};

mod board;
mod card;
mod cli;
mod detail;
mod drag;
mod fetch;
mod menu;
mod settings;
mod state;

fn pr(state: &str, draft: bool, ci: Ci, review: Review) -> Pr {
    Pr {
        id: "i".into(),
        url: "u".into(),
        repo: "o/r".into(),
        number: 1,
        title: "t".into(),
        author: "me".into(),
        draft,
        state: state.into(),
        updated: 0,
        closed: 0,
        additions: 0,
        deletions: 0,
        ci,
        review,
        my_review: None,
        requested: false,
    }
}

fn runtime(tab: usize) -> Runtime<App> {
    let mut s = App::new(Saved::default(), None, false);
    fixture(&mut s, tab);
    let mut rt = Runtime::new(s, view::view, (1180.0, 760.0), Env { system_dark: true, scale: 2.0 });
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
    rt
}

fn drag_to(rt: &mut Runtime<App>, from: &str, tx: f32, ty: f32) {
    let (x, y) = rt.find(from).unwrap();
    rt.pointer_down(x, y);
    for i in 1..=10 {
        let k = i as f32 / 10.0;
        rt.pointer_move(x + (tx - x) * k, y + (ty - y) * k);
        rt.frame(1.0 / 60.0);
    }
    for _ in 0..3 {
        rt.pointer_move(tx, ty);
        rt.frame(1.0 / 60.0);
    }
    rt.pointer_up(tx, ty);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

/// Drags onto an element, following it as the board moves under the pointer (a zone opening
/// in a gap the card crosses shifts the lanes below it).
fn drag(rt: &mut Runtime<App>, from: &str, to: &str) {
    let (x, y) = rt.find(from).unwrap();
    rt.pointer_down(x, y);
    for i in 1..=12 {
        let (tx, ty) = rt.find(to).unwrap();
        let k = (i as f32 / 10.0).min(1.0);
        rt.pointer_move(x + (tx - x) * k, y + (ty - y) * k);
        rt.frame(1.0 / 60.0);
    }
    let (tx, ty) = rt.find(to).unwrap();
    rt.pointer_up(tx, ty);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

fn rect(rt: &Runtime<App>, key: &str) -> [f32; 4] {
    rt.presented(key).unwrap().0
}

fn has(l: &Lanes, id: u64) -> bool {
    l.lanes.iter().any(|l| l.id == id)
}

const PSY: &str = "https://github.com/casassg/psy/pull/77";

fn settle(rt: &mut Runtime<App>) {
    rt.invalidate();
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
}

fn click(rt: &mut Runtime<App>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
}

const FLEET88: &str = "https://github.com/acme/fleet/pull/88";

/// Lifts a card, shows which action pills it offers, then (if `onto`) drops it on one, else
/// back where it started.
fn lift(rt: &mut Runtime<App>, card: &str, onto: Option<&str>) -> Vec<&'static str> {
    let (x, y) = rt.find(card).unwrap();
    rt.pointer_down(x, y);
    for i in 1..=4 {
        rt.pointer_move(x + 5.0 * i as f32, y);
        rt.frame(1.0 / 60.0);
    }
    rt.frame(1.0 / 60.0);
    let shown = ["publish", "draft", "merge", "close", "approve", "changes", "uninvolve", "dismiss"].into_iter()
        .filter(|a| rt.find(&format!("act-{a}")).is_some()).collect();
    // Aim at a point of the zone outside the card's own rect, which is a dead spot.
    let (tx, ty) = match onto {
        Some(a) => {
            let [ox, oy, ow, oh] = rt.state.drag.as_ref().unwrap().origin;
            let [zx, zy, zw, zh] = *rt.state.rects.get(&format!("act:{a}")).unwrap_or_else(|| panic!("no act:{a}"));
            [(zx + zw / 2.0, zy + zh / 2.0), (zx + 12.0, zy + 10.0), (zx + zw - 12.0, zy + 10.0), (zx + 12.0, zy + zh - 10.0), (zx + zw - 12.0, zy + zh - 10.0)]
                .into_iter()
                .find(|&(px, py)| !(px >= ox && px <= ox + ow && py >= oy && py <= oy + oh))
                .unwrap_or_else(|| panic!("act:{a} fully under the carried card"))
        }
        None => (x + 2.0, y),
    };
    for i in 1..=8 {
        let k = i as f32 / 8.0;
        rt.pointer_move(x + 20.0 + (tx - x - 20.0) * k, y + (ty - y) * k);
        rt.frame(1.0 / 60.0);
    }
    rt.pointer_up(tx, ty);
    for _ in 0..10 {
        rt.frame(1.0 / 60.0);
    }
    shown
}

fn find_pr<'a>(rt: &'a Runtime<App>, board: usize, url: &str) -> Option<&'a Pr> {
    rt.state.prs[board].iter().find(|p| p.url == url)
}

const WIKI: &str = "https://github.com/casassg/wiki/pull/412";
const R5501: &str = "https://github.com/acme/fleet/pull/5501";
