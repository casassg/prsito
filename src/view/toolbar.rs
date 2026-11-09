use rune::{El, Tx, el, row, spacer, text, text_input};

use crate::theme::*;
use crate::App;

pub(super) fn toolbar(s: &App, p: Pal) -> El {
    let b = s.board();
    let status = match &s.error[b] {
        Some(e) => Some(text(e.clone()).font(SMALL).color(p.stop).truncate().max_w(240.0)),
        None if s.live && !s.loaded[b] => {
            Some(text(format!("Loading from GitHub, {} so far", s.prs[b].len())).font(SMALL).color(p.muted))
        }
        None => None,
    };
    let open = s.search.focused || !s.search.text.is_empty();
    let search = text_input(&s.search, TITLE).key("search").single_line().w(if open { 360.0 } else { 180.0 }).h(28.0).px(10.0).py(5.0)
        .radius(7.0).fill(p.band).border(1.0, p.line).color(p.text)
        .placeholder(if open { "Search repo, #, title, author" } else { "Search  ⌘F" }, p.muted)
        .on_edit(|s: &mut App| &mut s.search)
        .on_input_key(|s: &mut App, ev: &rune::KeyEvent, tx: &mut Tx| {
            if ev.key != rune::Key::Escape { return false }
            if s.search.text.is_empty() {
                tx.blur();
            } else {
                s.search = rune::TextEdit::new("");
                s.search.focused = true;
            }
            true
        });
    let mut bar = row().key("toolbar").w_full().h(56.0).px(20.0).gap(16.0).center_y().child(spacer());
    if let Some(status) = status {
        bar = bar.child(status.key("status"));
    }
    el().w_full().h(56.0)
        .child(bar)
        .child(row().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).center().child(search))
}
