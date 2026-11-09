use rune::{Color, El, Icon, Transition, Tx, col, el, icon, row, text};

use super::glass;
use super::icons::*;
use crate::github::{self, Act, Pr};
use crate::store::Target;
use crate::theme::*;
use crate::{App, Menu};

pub(super) fn menu(s: &App, m: &Menu, pr: &Pr, p: Pal) -> El {
    let open = pr.state == "OPEN";
    let idle = s.acting.is_none();
    let block = github::merge_block(m.merge_state.as_deref());
    let item = |key: &str, glyph: Option<&'static Icon>, label: &str, on: bool, color: Color| {
        let fg = if on { color } else { color.alpha(0.4) };
        let mark = match glyph {
            Some(g) => icon(g, 14.0).color(if on && color == p.text { p.muted } else { fg }),
            None => el().size(14.0, 14.0),
        };
        row().key(key).w_full().h(28.0).px(8.0).gap(8.0).center_y().radius(6.0).role("menuitem").label(label.to_string())
            .hover(if on { p.text.alpha(0.08) } else { Color::TRANSPARENT }).disabled(!on)
            .child(mark)
            .child(text(label.to_string()).font(STAGE).color(fg).truncate().min_w(0.0))
    };
    let behind = m.merge_state.as_deref() == Some("BEHIND");
    let actions = if open { behind as usize + pr.draft as usize + 1 + s.x_act(pr).is_some() as usize } else { 0 };
    let rows = 2 + actions + s.lanes().lanes.len() + 1;
    // Rows, the gaps between them, the divider and the "Move to" header, inside the padding.
    let h = 10.0 + rows as f32 * 28.0 + (rows + 1) as f32 + 9.0 + 20.0;
    let (x, y) = match s.rects.get("root") {
        Some(&[_, _, w, wh]) => (m.x.min(w - 228.0).max(8.0), m.y.min(wh - h - 8.0).max(8.0)),
        None => (m.x, m.y),
    };
    let (link, copied) = (pr.url.clone(), pr.url.clone());
    let mut list = glass(col().key("menu").absolute(x, y).w(220.0).padding(5.0).gap(1.0).radius(12.0), p).transition(Transition::none())
        .on_press(|_: &mut App| {})
        .child(item("m-open", Some(&EXTERNAL), "Open in GitHub", true, p.text).on_press_tx(move |s: &mut App, tx: &mut Tx| {
            s.menu = None;
            if let Err(e) = tx.open_url(&link) {
                s.error[s.board()] = Some(format!("can't open {link}: {e:?}"));
            }
        }))
        .child(item("m-copy", Some(&COPY), "Copy URL", true, p.text).on_press_tx(move |s: &mut App, tx: &mut Tx| {
            s.menu = None;
            tx.copy(copied.clone());
        }));
    if open {
        if behind {
            let u = pr.url.clone();
            list = list.child(item("m-rebase", Some(&REBASE), "Rebase from main", idle, p.text).on_press(move |s: &mut App| s.run(Act::Rebase, &u)));
        }
        if pr.draft {
            let u = pr.url.clone();
            list = list.child(item("m-publish", Some(&SEND), "Publish PR", idle, p.text).on_press(move |s: &mut App| s.run(Act::Publish, &u)));
        }
        let can_merge = idle && block.is_none();
        let mut merge = item("m-merge", Some(&MERGE), "Merge (squash)", can_merge, p.text).help(block.unwrap_or("Squash and merge"));
        if can_merge {
            let u = pr.url.clone();
            merge = merge.on_press(move |s: &mut App| s.run(Act::Merge, &u));
        }
        let u = pr.url.clone();
        let armed = s.armed.as_ref().is_some_and(|(a, _)| *a == pr.url);
        list = list.child(merge);
        match s.x_act(pr) {
            Some(Act::Close) => list = list.child(item("m-close", Some(&X), if armed { "Click again to close" } else { "Close PR" }, idle, if armed { p.stop } else { p.text })
                .on_press(move |s: &mut App| s.arm_close(&u))),
            Some(_) => list = list.child(item("m-close", Some(&USER_MINUS), if armed { "Click again to remove me" } else { "Remove me as reviewer" }, idle, if armed { p.stop } else { p.text })
                .on_press(move |s: &mut App| s.arm_close(&u))),
            None => {}
        }
    }
    let here = s.lanes().lane_of(&pr.url);
    list = list.child(el().w_full().h(1.0).my(4.0).fill(p.text.alpha(0.08)))
        .child(row().w_full().h(20.0).px(8.0).center_y().child(text("Move to").font(SMALL).color(p.muted)));
    for lane in &s.lanes().lanes {
        let (u, id) = (pr.url.clone(), lane.id);
        let mut row = item(&format!("m-lane-{id}"), (id == here).then_some(&CHECK), &lane.name, true, p.text);
        if id == here {
            list = list.child(row.disabled(true).hover(Color::TRANSPARENT));
            continue;
        }
        row = row.on_press(move |s: &mut App| {
            s.menu = None;
            s.lanes_mut().move_card(&u, Target::Lane(id));
            s.persist();
        });
        list = list.child(row);
    }
    let u = pr.url.clone();
    list = list.child(item("m-newlane", Some(&PLUS), "New lane", true, p.text).on_press(move |s: &mut App| {
        s.menu = None;
        let at = s.lanes().lanes.len() - 1;
        let id = s.lanes_mut().move_card(&u, Target::New(at));
        s.persist();
        s.start_rename(id);
    }));
    let dismiss = |s: &mut App| {
        s.menu = None;
        s.armed = None;
    };
    el().key("menu-scrim").pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).transition(Transition::none()).instant_changes()
        .on_press(dismiss)
        .on_secondary_press(move |s: &mut App, _, _| dismiss(s))
        .child(list)
}

pub(crate) fn act_name(act: Act) -> &'static str {
    match act {
        Act::Merge => "merge",
        Act::Rebase => "rebase",
        Act::Close => "close",
        Act::Publish => "publish",
        Act::Draft => "draft",
        Act::Approve => "approve",
        Act::RequestChanges => "changes",
        Act::Uninvolve => "uninvolve",
        Act::Dismiss => "dismiss",
    }
}

/// The pills a carried card can be dropped on: only for open PRs, and on Reviews the X's
/// remove-me instead of closing someone else's PR.
pub(crate) fn actions(s: &App, pr: &Pr) -> Vec<(Act, &'static str)> {
    if pr.state != "OPEN" {
        return Vec::new();
    }
    if s.board() == github::REVIEWS {
        return match github::column(s.board(), pr) {
            0 => vec![(Act::Approve, "Approve"), (Act::RequestChanges, "Request changes"), (Act::Uninvolve, "Unassign")],
            1 => vec![(Act::Dismiss, "Dismiss my review"), (Act::Approve, "Approve"), (Act::Uninvolve, "Unassign")],
            2 => vec![(Act::Dismiss, "Dismiss approval"), (Act::Merge, "Merge")],
            _ => Vec::new(),
        };
    }
    match pr.draft {
        true => vec![(Act::Publish, "Publish it"), (Act::Close, "Close")],
        false => vec![(Act::Draft, "Make draft"), (Act::Merge, "Merge")],
    }
}
