use rune::tok::motion;
use rune::{Color, Drag, El, ScrollSpec, Tx, col, el, row, text, text_input};

use crate::github::{self, Pr};
use crate::store::{INBOX, Lane, Target};
use crate::theme::*;
use crate::App;
use crate::drag::{drag, Dragging};
use super::columns;
use super::card::{card, card_body};

/// The row of signals over the board: a lamp and name per column over a thin rail in the
/// column's colour, all dim where the column is empty.
pub(super) fn gantry(s: &App, p: Pal) -> El {
    let cols = columns(s);
    let mut lit = vec![false; cols.len()];
    for pr in s.visible(s.tab) {
        lit[github::column(s.board(), pr)] = true;
    }
    let mut r = row().key("headers").w_full().px(36.0).pt(8.0).pb(8.0).gap(16.0);
    for (i, name) in cols.iter().enumerate() {
        let c = signal(p, s.board(), i);
        r = r.child(col().key(format!("col-{i}")).grow(1.0).min_w(0.0).gap(10.0)
            .child(row().w_full().gap(8.0).center_y()
                .child(lamp("lamp", if lit[i] { c.alpha(0.8) } else { c.alpha(0.2) }, 6.0))
                .child(text(*name).font(STAGE).color(if lit[i] { p.muted } else { p.muted.alpha(0.45) }).truncate().shrink().min_w(0.0)))
            .child(el().key("rail").w_full().h(1.0).fill(c.alpha(if lit[i] { 0.45 } else { 0.12 }))));
    }
    r
}

pub(super) fn board(s: &App, p: Pal) -> El {
    let cols = columns(s);
    let lanes = s.lanes();
    let visible = s.visible(s.tab);
    let new_at = s.drag.as_ref().and_then(|d| match d.over {
        Some(Target::New(i)) => Some(i),
        _ => None,
    });
    let mut list = col().key("board").w_full().px(28.0).pb(40.0).gap(8.0);
    let searching = !s.search.text.trim().is_empty();
    for (i, lane) in lanes.lanes.iter().enumerate() {
        if new_at == Some(i) {
            list = list.child(new_lane_zone(i, p));
        }
        let mut cells: Vec<Vec<&Pr>> = vec![Vec::new(); cols.len()];
        for pr in &visible {
            if lanes.lane_of(&pr.url) == lane.id {
                cells[github::column(s.board(), pr)].push(pr);
            }
        }
        if searching && cells.iter().all(Vec::is_empty) {
            continue;
        }
        list = list.child(lane_el(s, lane, cells, p));
    }
    list.scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0)
        .on_layout_tx(|s: &mut App, r, tx| { s.rects.insert("board".into(), r); tx.live_only(); })
}

/// Where a carried card would start a new lane: only while a card hovers a lane's edge or the
/// gap between two lanes.
fn new_lane_zone(at: usize, p: Pal) -> El {
    row().key("newlane").w_full().h(52.0).center().radius(12.0).fill(p.band)
        .border(1.0, p.text.alpha(0.5)).border_dash(6.0, 5.0)
        .on_layout_tx(move |s: &mut App, r, tx| { s.rects.insert(format!("new:{at}"), r); tx.live_only(); })
        .child(text("Drop to start a new lane").font(STAGE).color(p.text))
}

fn lane_el(s: &App, lane: &Lane, cells: Vec<Vec<&Pr>>, p: Pal) -> El {
    let id = lane.id;
    let inbox = id == INBOX;
    let over = s.drag.as_ref().is_some_and(|d| d.over == Some(Target::Lane(id)) && d.what != Dragging::Lane(id));
    let lifted = s.drag.as_ref().is_some_and(|d| d.what == Dragging::Lane(id));
    let chevron = row().key("chevron").size(20.0, 20.0).center().radius(5.0).hover(p.raised)
        .child(text(if lane.collapsed { "▸" } else { "▾" }).font(STAGE).color(p.muted))
        .on_press(move |s: &mut App| {
            if let Some(l) = s.lanes_mut().lane_mut(id) {
                l.collapsed = !l.collapsed;
            }
            s.persist();
        });
    let name: El = match s.renaming {
        Some(rid) if rid == id => text_input(&s.rename, LANE).key("rename").single_line().w(260.0)
            .px(6.0).py(2.0).radius(6.0).fill(p.raised).color(p.text)
            .on_edit(|s: &mut App| &mut s.rename)
            .on_input_key(|s: &mut App, ev: &rune::KeyEvent, _: &mut Tx| {
                if ev.key != rune::Key::Escape { return false }
                s.renaming = None;
                true
            })
            .on_submit(|s: &mut App, _| s.finish_rename()),
        _ => text(lane.name.clone()).font(LANE).color(if inbox { p.muted } else { p.text })
            .help(if inbox { "PRs you haven't sorted. Drag one out to start a lane." } else { "Double click to rename, drag to reorder" }),
    };
    let mut handle = row().key("grip").grow(1.0).h_full().gap(10.0).center_y().child(name);
    if lane.collapsed {
        for (i, _) in cells.iter().enumerate().filter(|(_, c)| !c.is_empty()) {
            handle = handle.child(lamp(&format!("mini-{i}"), signal(p, s.board(), i).alpha(0.7), 5.0));
        }
    }
    let handle = handle.on_drag(move |s: &mut App, d: &mut Drag, tx: &mut Tx| drag(s, Dragging::Lane(id), d, tx));
    let header = row().key("head").w_full().h(28.0).gap(6.0).center_y().child(chevron).child(handle);
    let mut body = col().key(format!("lane-{id}")).w_full().px(8.0).pt(12.0).pb(if lane.collapsed { 8.0 } else { 24.0 }).gap(12.0).radius(14.0)
        .fill(if over { p.band } else { Color::TRANSPARENT })
        .border(1.0, if over { p.text.alpha(0.25) } else { Color::TRANSPARENT })
        .opacity(if lifted { 0.35 } else { 1.0 })
        .on_layout_tx(move |s: &mut App, r, tx| { s.rects.insert(format!("lane:{id}"), r); tx.live_only(); })
        .child(header);
    if !lane.collapsed {
        let done = cells.len() - 1;
        let open = s.open_stacks.contains(&(s.board(), id));
        let mut cols = row().key("cells").w_full().gap(16.0).items_start();
        for (i, prs) in cells.into_iter().enumerate() {
            let mut cell = col().key(format!("cell-{i}")).grow(1.0).min_w(0.0).gap(12.0).min_h(8.0);
            if i == done && prs.len() > 1 && !open {
                cell = cell.child(deck(s, id, &prs, p));
            } else {
                if i == done && prs.len() > 1 {
                    cell = cell.child(row().key("fold").h(20.0).px(6.0).gap(6.0).center_y().radius(5.0).hover(p.band)
                        .role("button").label("Stack done PRs")
                        .child(text("▴ Collapse").font(SMALL).color(p.muted))
                        .on_press(move |s: &mut App| { s.open_stacks.remove(&(s.board(), id)); }));
                }
                for pr in prs {
                    cell = cell.child(card(s, pr, p));
                }
            }
            cols = cols.child(cell);
        }
        body = body.child(cols);
    }
    body
}

/// Merged and closed PRs as a deck: the newest on top, a sliver for each of the next few under
/// it that opens a little while hovered, and a click lists them all.
fn deck(s: &App, lane: u64, prs: &[&Pr], p: Pal) -> El {
    let k = (s.board(), lane);
    let fanned = s.fanned == Some(k);
    let mut d = col().key("stack").w_full().role("button").label(format!("Show {} done PRs", prs.len()))
        .hover_group().motion(motion::SNAPPY)
        .on_hover_change(move |s: &mut App, over, _| {
            if over {
                s.fanned = Some(k);
            } else if s.fanned == Some(k) {
                s.fanned = None;
            }
        })
        .on_press(move |s: &mut App| {
            s.open_stacks.insert(k);
            s.fanned = None;
        })
        .child(card_body(s, prs[0], p).fill(if fanned { p.card_hover } else { p.card }));
    for (j, pr) in prs[1..].iter().take(3).enumerate() {
        let inset = 8.0 * (j + 1) as f32;
        d = d.child(row().key(format!("peek-{j}")).h(if fanned { 24.0 } else { 7.0 }).mx(inset).px(10.0).center_y()
            .fill(p.card.mix(p.ink, 0.3 + 0.2 * j as f32)).border(1.0, p.line.alpha(0.5)).clip_corners([0.0, 0.0, 6.0, 6.0])
            .shadow(2.0, 8.0, p.shade.alpha(p.shade.a * 0.7))
            .child(text(format!("#{} {}", pr.number, pr.title)).font(SMALL).color(p.muted)
                .truncate().min_w(0.0).group_opacity(0.0, 1.0)));
    }
    d
}
