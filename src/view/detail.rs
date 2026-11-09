use rune_kit::{Markdown, MarkdownStyle};
use rune::{Color, El, ScrollSpec, Tx, col, el, row, spacer, text, text_input};

use crate::github::{self, Act, Pr};
use crate::theme::*;
use crate::{App, Detail};
use super::{action_button, glass};
use super::icons::*;
use super::card::status;

pub(super) fn detail(s: &App, d: &Detail, pr: &Pr, p: Pal) -> El {
    let open = pr.state == "OPEN";
    let busy = s.acting.as_ref().filter(|(u, _)| *u == pr.url).map(|(_, a)| *a);
    let idle = s.acting.is_none();
    let (ci, review) = status(pr, p);
    let info = d.info.as_ref();
    let block = github::merge_block(info.map(|i| i.merge_state.as_str()));
    let armed = s.armed.as_ref().is_some_and(|(u, _)| *u == pr.url);
    let (url, link) = (pr.url.clone(), pr.url.clone());
    let link_text = |key: String, label: String, to: String, color: Color| {
        text(label).key(key).font(SMALL).color(color).truncate().min_w(0.0).role("link")
            .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                if let Err(e) = tx.open_url(&to) {
                    s.error[s.board()] = Some(format!("can't open {to}: {e:?}"));
                }
            })
    };
    let quiet = p.text.alpha(0.07);
    let label = |act: Act, idle_label: &'static str, busy_label: &'static str| if busy == Some(act) { busy_label } else { idle_label };

    let mut buttons = row().key("actions").w_full().gap(8.0).center_y()
        .child(action_button("open", &EXTERNAL, "Open in GitHub", true, quiet, p.text).on_press_tx(move |s: &mut App, tx: &mut Tx| {
            if let Err(e) = tx.open_url(&link) {
                s.error[s.board()] = Some(format!("can't open {link}: {e:?}"));
            }
        }))
        .child(spacer());
    if open {
        if info.is_some_and(|i| i.merge_state == "BEHIND") {
            let u = url.clone();
            buttons = buttons.child(action_button("rebase", &REBASE, label(Act::Rebase, "Rebase from main", "Rebasing…"), idle, quiet, p.text)
                .on_press(move |s: &mut App| s.run(Act::Rebase, &u)));
        }
        if pr.draft {
            let u = url.clone();
            buttons = buttons.child(action_button("publish", &SEND, label(Act::Publish, "Publish PR", "Publishing…"), idle, quiet, p.text)
                .on_press(move |s: &mut App| s.run(Act::Publish, &u)));
        }
        let can_merge = idle && block.is_none();
        let u = url.clone();
        let mut merge = action_button("merge", &MERGE, label(Act::Merge, "Merge", "Merging…"), can_merge, if can_merge { p.text } else { quiet }, if can_merge { p.ink } else { p.text });
        if can_merge {
            merge = merge.on_press(move |s: &mut App| s.run(Act::Merge, &u));
        }
        let u = url.clone();
        let close = match s.x_act(pr) {
            Some(Act::Close) => Some(if armed { "Click again to close" } else { label(Act::Close, "Close PR", "Closing…") }),
            Some(_) => Some(if armed { "Click again to remove me" } else { label(Act::Uninvolve, "Remove me", "Removing…") }),
            None => None,
        };
        if let Some(close) = close {
            buttons = buttons.child(action_button("close", if s.x_act(pr) == Some(Act::Close) { &X } else { &USER_MINUS }, close, idle, if armed { p.stop } else { quiet }, if armed { p.ink } else { p.text }).on_press(move |s: &mut App| s.arm_close(&u)));
        }
        buttons = buttons.child(merge);
    }

    let body = match info.map(|i| i.body.as_str()) {
        None => text("Loading description").font(TITLE).color(p.muted),
        Some("") => text("No description").font(TITLE).color(p.muted),
        Some(b) => Markdown::new(b).selectable(true)
            .on_link_tx(|s: &mut App, url: &str, tx: &mut Tx| {
                if let Err(e) = tx.open_url(url) {
                    s.error[s.board()] = Some(format!("can't open {url}: {e:?}"));
                }
            })
            .render(&MarkdownStyle::neutral(true).text(p.text.alpha(0.85)).muted(p.muted).link(p.text)
                .inline_code_bg(p.band).code_block(p.band, p.line).quote_rule(p.line).rule(p.line).body_font(TITLE)
                .diagram(p.card, p.line, p.muted).diagram_surface(p.band, p.line, p.card).diagram_note(p.card_hover, p.line)),
    };
    let mut people = col().key("people").w_full().gap(4.0);
    for (i, c) in info.into_iter().flat_map(|i| i.checks.iter()).enumerate() {
        let (color, state) = if c.running { (p.caution, "running") } else { (p.stop, "failed") };
        people = people.child(row().key(format!("check-{i}")).w_full().gap(6.0).center_y()
            .child(el().size(7.0, 7.0).radius(3.5).fill(color))
            .child(link_text(format!("check-link-{i}"), c.name.clone(), c.url.clone(), p.text.alpha(0.85)).shrink())
            .child(text(state).font(SMALL).color(p.muted)));
    }
    if let Some(i) = info.filter(|i| !i.approvers.is_empty()) {
        people = people.child(row().key("approvers").w_full().gap(6.0).center_y()
            .child(el().size(7.0, 7.0).radius(3.5).fill(p.go))
            .child(text(format!("Approved by {}", i.approvers.join(", "))).font(SMALL).color(p.muted).wrap()));
    }
    let mut card = glass(col().key("detail").w(620.0).padding(20.0).gap(12.0).radius(16.0), p)
        .role("dialog").label(format!("{}#{} {}", pr.repo, pr.number, pr.title))
        .on_press(|_: &mut App| {})
        .child(row().w_full().gap(8.0).center_y()
            .child(text(format!("{} #{}", pr.repo, pr.number)).font(SMALL).color(p.muted).grow(1.0).min_w(0.0).truncate())
            .child(text(github::age(s.now, pr.updated)).font(SMALL).color(p.muted)))
        .child(text(pr.title.clone()).key("title").font(LANE).color(p.text).wrap())
        .child(link_text("link".into(), pr.url.clone(), pr.url.clone(), p.muted).w_full())
        .child(row().w_full().gap(8.0).center_y()
            .child(el().size(7.0, 7.0).radius(3.5).fill(ci.0))
            .child(text(ci.1).font(SMALL).color(p.muted))
            .child(text("·").font(SMALL).color(p.muted))
            .child(text(review.1).font(SMALL).color(p.muted))
            .child(spacer())
            .child(text(format!("+{}", pr.additions)).font(SMALL).color(p.muted))
            .child(text(format!("−{}", pr.deletions)).font(SMALL).color(p.muted)))
        .child(people)
        .child(col().key("body").w_full().max_h(320.0).scroll(ScrollSpec { stick_bottom: false }).child(body))
        .child(buttons);
    if open && let Some(why) = block {
        card = card.child(text(format!("Merge is off: {}", why.to_lowercase())).font(SMALL).color(p.caution));
    }
    el().key("scrim").pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).center().fill(Color::BLACK.alpha(0.35))
        .on_press(|s: &mut App| {
            s.detail = None;
            s.armed = None;
        })
        .child(card)
}

/// The comment a changes request needs, asked for when a card is dropped on it.
pub(super) fn note(edit: &rune::TextEdit, p: Pal) -> El {
    let quiet = p.text.alpha(0.07);
    let card = glass(col().key("note").w(460.0).padding(18.0).gap(12.0).radius(16.0), p)
        .role("dialog").label("Request changes")
        .on_press(|_: &mut App| {})
        .child(text("Request changes").font(LANE).color(p.text))
        .child(text_input(edit, TITLE).key("note-input").single_line().w_full().h(30.0).px(10.0).py(6.0)
            .radius(7.0).fill(p.ink).border(1.0, p.line).color(p.text)
            .placeholder("What needs to change?", p.muted)
            .on_edit(|s: &mut App| &mut s.note_text)
            .on_input_key(|s: &mut App, ev: &rune::KeyEvent, _: &mut Tx| {
                if ev.key != rune::Key::Escape { return false }
                s.note = None;
                true
            })
            .on_submit(|s: &mut App, _| s.send_changes()))
        .child(row().w_full().gap(8.0).center_y().child(spacer())
            .child(action_button("note-cancel", &X, "Cancel", true, quiet, p.text).on_press(|s: &mut App| s.note = None))
            .child(action_button("note-send", &COMMENT, "Request changes", true, p.text, p.ink).on_press(|s: &mut App| s.send_changes())));
    el().key("note-scrim").pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).center().fill(Color::BLACK.alpha(0.35))
        .on_press(|s: &mut App| s.note = None)
        .child(card)
}
