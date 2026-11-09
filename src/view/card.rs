use rune::{Color, Drag, El, Transition, Tx, col, el, icon, row, text};

use crate::github::{self, Act, Ci, Pr, Review};
use crate::theme::*;
use crate::App;
use crate::drag::{carries, drag, DragState, Dragging};
use super::columns;
use super::icons::{X, act_icon};
use super::menu::act_name;

fn card_key(pr: &Pr) -> String {
    format!("card-{}-{}", pr.repo.replace('/', "-"), pr.number)
}

pub(super) fn card(s: &App, pr: &Pr, p: Pal) -> El {
    let url = pr.url.clone();
    let lifted = carries(s, &pr.url);
    let selected = s.selected.contains(&pr.url);
    let rect_key = format!("card:{}", pr.url);
    let menu_url = pr.url.clone();
    let grabbed = s.drag.as_ref().is_some_and(|d| d.what == Dragging::Card(pr.url.clone()));
    let body = card_body(s, pr, p);
    let mut body = if lifted && !grabbed { body.key(format!("{}-held", card_key(pr))) } else { body.key(card_key(pr)) };
    if !lifted {
        body = body.matched(format!("fly:{}", pr.url));
    }
    if selected {
        body.border(1.5, p.text.alpha(0.85))
    } else {
        body
    }
        .on_secondary_press(move |s: &mut App, at, tx: &mut Tx| { tx.instant(); s.open_menu(menu_url.clone(), at.x, at.y) })
        .hover(p.card_hover)
        .hover_group()
        .opacity(if lifted { 0.3 } else { 1.0 })
        .on_layout_tx(move |s: &mut App, r, tx| { s.rects.insert(rect_key.clone(), r); tx.live_only(); })
        .on_drag(move |s: &mut App, d: &mut Drag, tx: &mut Tx| drag(s, Dragging::Card(url.clone()), d, tx))
}

pub(super) fn status(pr: &Pr, p: Pal) -> ((Color, &'static str), (Color, &'static str)) {
    let ci = match pr.ci {
        Ci::Pass => (p.go, "CI passed"),
        Ci::Fail => (p.caution, "CI failed"),
        Ci::Pending => (p.caution, "CI running"),
        Ci::None => (p.idle.alpha(0.6), "No CI"),
    };
    let review = match (pr.state.as_str(), pr.review) {
        ("MERGED", _) => (p.merged, "Merged"),
        ("CLOSED", _) => (p.muted, "Closed"),
        (_, Review::Approved) => (p.go, "Approved"),
        (_, Review::Changes) => (p.caution, "Changes requested"),
        (_, Review::None) => (p.muted, "No review yet"),
    };
    (ci, review)
}

/// The X, shown while the card is hovered: one click arms it (it turns red and reads "Close?"), a
/// second within a few seconds closes the PR, or on Reviews removes me as reviewer.
fn close_button(s: &App, pr: &Pr, p: Pal) -> El {
    let armed = s.armed.as_ref().is_some_and(|(u, _)| *u == pr.url);
    let url = pr.url.clone();
    let mine = s.x_act(pr) == Some(Act::Close);
    let (label, confirm, ask) = if mine { ("Close PR", "Confirm close PR", "Close?") } else { ("Remove me as reviewer", "Confirm remove me", "Remove?") };
    let glyph = if armed { text(ask).font(SMALL).color(p.ink) } else { icon(&X, 12.0).color(p.muted) };
    row().key(format!("close-{}", card_key(pr))).h(18.0).min_w(18.0).px(if armed { 7.0 } else { 3.0 }).center().radius(9.0)
        .fill(if armed { p.stop } else { Color::TRANSPARENT }).hover(if armed { p.stop } else { p.text.alpha(0.1) })
        .role("button").label(if armed { confirm } else { label })
        .child(glyph)
        .on_press(move |s: &mut App| s.arm_close(&url))
        .on_hover_change(|s: &mut App, over, _| {
            if !over {
                s.armed = None;
            }
        })
}

pub(super) fn card_body(s: &App, pr: &Pr, p: Pal) -> El {
    let (ci, review) = status(pr, p);
    let ci_failed_why = s.board() == github::MINE && pr.ci == Ci::Fail && pr.state == "OPEN";
    let ci_says = ci_failed_why && pr.review != Review::Changes;
    let dot = match () {
        _ if pr.state != "OPEN" => review.0.alpha(0.6),
        _ if ci_says || pr.review == Review::None => ci.0,
        _ => review.0,
    };
    let foot = row().key("meta").w_full().gap(8.0).center_y()
        .child(el().key("ci").size(7.0, 7.0).radius(3.5).fill(dot).label(ci.1).help(format!("{} · {}", ci.1, review.1)))
        .child(text(if ci_says { "CI failed" } else { review.1 })
            .key("review").font(SMALL).color(p.muted)
            .truncate().shrink().min_w(0.0));
    let mut extra = row().key("extra").pin(None, Some(0.0), Some(0.0), Some(0.0)).pl(12.0).gap(8.0).center_y()
        .fill(p.card_hover).group_opacity(0.0, 1.0)
        .child(text(format!("+{} −{}", pr.additions, pr.deletions)).key("diff").font(SMALL).color(p.muted.alpha(0.8)))
        .child(text(github::age(s.now, pr.updated)).key("age").font(SMALL).color(p.muted));
    if pr.state == "OPEN" && s.x_act(pr).is_some() {
        extra = extra.child(close_button(s, pr, p));
    }
    let top = row().key("top").w_full().h(16.0).center_y()
        .child(text(format!("{} #{}", pr.repo.rsplit('/').next().unwrap_or(&pr.repo), pr.number)).font(SMALL).color(p.muted.alpha(0.8))
            .truncate().grow(1.0).min_w(0.0).help(pr.repo.clone()))
        .child(extra);
    col().padding(14.0).gap(8.0).radius(10.0).fill(p.card).role("link")
        .label(format!("{}#{} {}", pr.repo, pr.number, pr.title))
        .border(1.0, p.edge).shadow(3.0, 12.0, p.shade)
        .child(top)
        .child(text(pr.title.clone()).key("title").font(TITLE).color(if pr.draft { p.text.alpha(0.72) } else { p.text }).wrap().max_lines(2))
        .child(foot)
}

/// What follows the pointer. The other carried cards fly in from their places to stack under
/// it and trail it on a spring; on the drop they fly to where they land.
pub(super) fn ghost(s: &App, d: &DragState, p: Pal) -> El {
    let [x, y, w, _] = d.origin;
    let body = match &d.what {
        Dragging::Card(url) => match s.prs[s.board()].iter().find(|pr| &pr.url == url) {
            Some(pr) => card_body(s, pr, p).fill(p.card_hover),
            None => el(),
        },
        Dragging::Lane(id) => {
            let name = s.lanes().lanes.iter().find(|l| l.id == *id).map(|l| l.name.clone()).unwrap_or_default();
            row().h(46.0).px(16.0).center_y().radius(14.0).fill(p.raised)
                .child(text(name).font(LANE).color(p.text))
        }
    };
    let n = match &d.what {
        Dragging::Card(url) if s.selected.contains(url) => s.selected.len(),
        _ => 1,
    };
    let (gx, gy) = (x + d.dx, y + d.dy);
    let mut ghost = el().key("ghost").absolute(gx, gy).w(w).instant_changes()
        .child(body.w(w).rotate(-0.025).shadow(14.0, 32.0, Color::BLACK.alpha(0.55)).opacity(0.96));
    if n > 1 {
        ghost = ghost.child(row().key("count").absolute(w - 12.0, -10.0).size(22.0, 22.0).center().radius(11.0).fill(p.text)
            .child(text(n.to_string()).font(SMALL).color(p.ink)));
    }
    let Dragging::Card(url) = &d.what else { return ghost };
    if n == 1 {
        return ghost;
    }
    let mut carry = el().key("carry").absolute(0.0, 0.0);
    let behind: Vec<&Pr> = s.prs[s.board()].iter().filter(|pr| s.selected.contains(&pr.url) && &pr.url != url).collect();
    for (i, pr) in behind.into_iter().enumerate().rev() {
        let k = (i + 1).min(2) as f32;
        carry = carry.child(card_body(s, pr, p).key(format!("behind-{i}")).matched(format!("fly:{}", pr.url))
            .fill(p.card_hover).absolute(gx + 6.0 * k, gy + 6.0 * k).w(w).rotate(0.02 * k).opacity(0.85));
    }
    carry.child(ghost)
}

/// The drop zones while a card is carried: faded slabs over the card's own lane, snapped to the
/// board's columns. Fewer actions than columns spread over the extra ones, so edges always land
/// on column edges. Labels sit at the middle of the lane's visible part, so a lane taller than
/// the window still reads. The card's own column stays open (dropping where it already sits
/// should do nothing), which can split a zone in two. Other lanes stay plain lane drops.
pub(super) fn action_zones(s: &App, pr: &Pr, d: &DragState, pills: &[(Act, &'static str)], p: Pal) -> Option<El> {
    let lane = s.lanes().lane_of(&pr.url);
    let [x, y, w, h] = *s.rects.get(&format!("lane:{lane}"))?;
    // The lane's cells: px(8) padding and 16 between columns, each an equal share.
    let n = columns(s).len().max(pills.len());
    let (ix, iw) = (x + 8.0, w - 16.0);
    let cw = (iw - 16.0 * (n as f32 - 1.0)) / n as f32;
    let [_, by, _, bh] = s.rects.get("board").copied().unwrap_or([x, y, w, h]);
    let mid = (y.max(by) + (y + h).min(by + bh)) / 2.0;
    let (zy, zh) = (8.0, h - 16.0);
    let ly = (mid - y - zy - 9.0).clamp(10.0, (zh - 28.0).max(10.0));
    let mut rail = el().key("actions-rail").absolute(x, y).size(w, h).transition(Transition::none()).instant_changes();
    // Each column joins the zone whose anchor is nearest, so a zone sits on the column the
    // action would send the card to and the extras fold into whichever is closest.
    let home = github::column(s.board(), pr);
    // Zones read literally: each action sits on the columns the drop would send the card to
    // (dismissing an approval reaches back over both earlier columns); the card's own column
    // and any column left unclaimed stay plain lane drops.
    let fixed: fn(Act) -> &'static [usize] = if s.board() == github::MINE {
        |act| match act {
            Act::Draft => &[0],
            Act::Publish => &[1, 2, 3],
            Act::Close | Act::Merge => &[4],
            _ => &[],
        }
    } else {
        |act| match act {
            Act::Dismiss => &[0, 1],
            Act::RequestChanges => &[1],
            Act::Approve => &[2],
            Act::Uninvolve | Act::Merge => &[3],
            _ => &[],
        }
    };
    for &(act, label) in pills {
        let cols: Vec<usize> = fixed(act).iter().copied().filter(|&c| c != home && c < n).collect();
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for &c in &cols {
            match runs.last_mut() {
                Some((_, last)) if *last + 1 == c => *last = c,
                _ => runs.push((c, c)),
            }
        }
        for (r, &(first, last)) in runs.iter().enumerate() {
            let span = last - first + 1;
            let zx = ix - x + first as f32 * (cw + 16.0);
            let zw = span as f32 * cw + 16.0 * (span as f32 - 1.0);
            let name = act_name(act);
            let on = d.act == Some(act);
            let c = if on && matches!(act, Act::Close) { p.stop } else { p.text };
            let (key, rect) = if r == 0 {
                (format!("act-{name}"), format!("act:{name}"))
            } else {
                (format!("act-{name}-{r}"), format!("act:{name}:{r}"))
            };
            rail = rail.child(el().key(key).absolute(zx, zy).size(zw, zh).radius(12.0)
                .fill(if on { p.raised.mix(c, 0.08).alpha(0.92) } else { p.ink.alpha(0.82) })
                .backdrop_blur(12.0)
                .border(1.5, if on { c.alpha(0.7) } else { p.text.alpha(0.16) }).border_dash(6.0, 5.0).transition(Transition::none())
                .on_layout_tx(move |s: &mut App, r, tx| { s.rects.insert(rect.clone(), r); tx.live_only(); })
                .child(row().absolute(0.0, ly).w(zw).h(18.0).gap(8.0).center()
                    .child(icon(act_icon(act), 16.0).color(if on { c } else { p.muted }))
                    .child(text(label).font(LANE).color(if on { c } else { p.muted }).truncate().min_w(0.0))));
        }
    }
    Some(rail)
}
