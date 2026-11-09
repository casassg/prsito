use super::*;

#[test]
fn the_new_lane_zone_shows_only_while_a_card_hovers_between_lanes() {
    let mut rt = runtime(github::MINE);
    assert!(rt.find("newlane").is_none(), "no zone at rest");
    let (x, y) = rt.find("card-casassg-psy-77").unwrap();
    let [_, top, _, _] = rect(&rt, "lane-2");
    rt.pointer_down(x, y);
    for i in 1..=10 {
        rt.pointer_move(x, y + (top - 6.0 - y) * i as f32 / 10.0);
        rt.frame(1.0 / 60.0);
    }
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.drag.as_ref().and_then(|d| d.over), Some(Target::New(1)), "gap above Pipelines");
    assert!(rt.find("newlane").is_some(), "the zone opens there");
    let [_, mid, _, h] = rect(&rt, "lane-1");
    rt.pointer_move(x, mid + h / 2.0);
    rt.frame(1.0 / 60.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.drag.as_ref().and_then(|d| d.over), Some(Target::Lane(1)));
    assert!(rt.find("newlane").is_none(), "and closes once the card is over a lane");
    rt.pointer_up(x, mid + h / 2.0);
    for _ in 0..10 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(rt.state.lanes().lane_of(PSY), 1);
    assert!(rt.find("newlane").is_none(), "nor after the drop");
}

#[test]
fn dropping_above_the_top_lane_starts_a_lane_at_the_top() {
    let mut rt = runtime(github::MINE);
    let [_, top, _, _] = rect(&rt, "lane-1");
    let (x, _) = rt.find("card-casassg-psy-77").unwrap();
    drag_to(&mut rt, "card-casassg-psy-77", x, top - 30.0);
    let id = rt.state.lanes().lane_of(PSY);
    assert_ne!(id, INBOX);
    assert_eq!(rt.state.lanes().lanes[0].id, id, "the new lane is first");
    assert!(rt.find(&format!("lane-{id}")).is_some());
}

#[test]
fn dropping_between_lanes_starts_a_lane_there() {
    let mut rt = runtime(github::MINE);
    let [_, bottom, _, h] = rect(&rt, "lane-1");
    let (x, _) = rt.find("card-casassg-psy-77").unwrap();
    drag_to(&mut rt, "card-casassg-psy-77", x, bottom + h - 4.0);
    let ids: Vec<u64> = rt.state.lanes().lanes.iter().map(|l| l.id).collect();
    let id = rt.state.lanes().lane_of(PSY);
    assert_eq!(ids, [1, id, 2, INBOX], "after the lane whose bottom edge it was dropped on");
}

#[test]
fn dragging_a_card_to_another_lane_moves_it_and_empty_lanes_go() {
    let mut rt = runtime(github::MINE);
    drag(&mut rt, "card-acme-fleet-88", "lane-0");
    assert_eq!(rt.state.lanes().lane_of("https://github.com/acme/fleet/pull/88"), INBOX);
    drag(&mut rt, "card-acme-fleet-91", "lane-2");
    drag(&mut rt, "card-acme-fleet-85", "lane-2");
    assert_eq!(rt.state.lanes().lane_of("https://github.com/acme/fleet/pull/91"), 2);
    assert!(!has(rt.state.lanes(), 1), "Fleet lane emptied and went away");
}

#[test]
fn a_tap_opens_the_pr_and_moves_nothing() {
    let mut rt = runtime(github::MINE);
    let before = rt.state.saved.clone();
    let (x, y) = rt.find("card-casassg-psy-77").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.saved, before);
    assert!(rt.state.drag.is_none());
    assert!(rt.state.error.iter().all(Option::is_none));
}

#[test]
fn dragging_a_lane_header_reorders_lanes() {
    let mut rt = runtime(github::MINE);
    let order = |rt: &Runtime<App>| rt.state.lanes().lanes.iter().map(|l| l.id).collect::<Vec<_>>();
    assert_eq!(order(&rt), [1, 2, INBOX]);
    drag(&mut rt, "lane-2/head/grip", "lane-1/head");
    assert_eq!(order(&rt), [2, 1, INBOX]);
    drag(&mut rt, "lane-0/head/grip", "lane-2/head");
    assert_eq!(order(&rt), [2, 1, INBOX], "Inbox stays put");
}

#[test]
fn carried_cards_offer_actions_for_what_they_are() {
    let mut rt = runtime(github::MINE);
    assert!(rt.find("actions-rail").is_none(), "no pills at rest");
    assert_eq!(lift(&mut rt, "card-casassg-wiki-412", None), ["publish", "close"]);
    assert_eq!(lift(&mut rt, "card-acme-fleet-88", None), ["draft", "merge"], "published PRs offer draft, not close");
    assert_eq!(lift(&mut rt, "card-acme-fleet-85", None), Vec::<&str>::new(), "closed PRs offer nothing");
    assert!(rt.find("actions-rail").is_none() && rt.state.drag.is_none(), "pills go with the drop");
    assert_eq!(find_pr(&rt, 0, FLEET88).unwrap().state, "OPEN", "a drop elsewhere runs nothing");

    let mut rt = runtime(github::REVIEWS);
    assert_eq!(lift(&mut rt, "card-acme-fleet-5501", None), ["approve", "changes", "uninvolve"], "to review: no merge, no dismiss yet");
    assert_eq!(lift(&mut rt, "card-acme-pipelines-5490", None), ["approve", "uninvolve", "dismiss"], "waiting on author: dismiss my review");
    assert_eq!(lift(&mut rt, "card-acme-serving-5470", None), ["merge", "dismiss"], "approved: merge or dismiss approval");
}

#[test]
fn dropping_on_a_pill_runs_it() {
    let mut rt = runtime(github::MINE);
    lift(&mut rt, "card-casassg-wiki-412", Some("publish"));
    assert!(!find_pr(&rt, 0, WIKI).unwrap().draft);
    lift(&mut rt, "card-acme-fleet-88", Some("merge"));
    assert_eq!(find_pr(&rt, 0, FLEET88).unwrap().state, "MERGED");
    lift(&mut rt, "card-acme-fleet-91", Some("draft"));
    assert!(find_pr(&rt, 0, "https://github.com/acme/fleet/pull/91").unwrap().draft, "published goes back to draft");

    let mut rt = runtime(github::REVIEWS);
    lift(&mut rt, "card-acme-fleet-5501", Some("approve"));
    let pr = find_pr(&rt, 1, R5501).unwrap();
    assert_eq!((pr.my_review.as_deref(), pr.requested), (Some("APPROVED"), false));
    assert_eq!(github::column(github::REVIEWS, pr), 2, "into Approved");
    lift(&mut rt, "card-acme-serving-5470", Some("dismiss"));
    let pr = find_pr(&rt, 1, "https://github.com/acme/serving/pull/5470").unwrap();
    assert_eq!((pr.my_review.as_deref(), github::column(github::REVIEWS, pr)), (None, 0), "a dismissed approval goes back to review");
}

#[test]
fn requesting_changes_asks_for_a_comment_first() {
    let mut rt = runtime(github::REVIEWS);
    lift(&mut rt, "card-acme-fleet-5501", Some("changes"));
    assert_eq!(rt.state.note.as_deref(), Some(R5501));
    assert!(rt.find("note-input").is_some());
    assert!(on_key(&mut rt.state, &KeyEvent { key: Key::Escape, mods: Default::default() }, &mut Tx::default()));
    assert!(rt.state.note.is_none() && find_pr(&rt, 1, R5501).unwrap().requested, "escape sends nothing");
    settle(&mut rt);

    lift(&mut rt, "card-acme-fleet-5501", Some("changes"));
    rt.state.send_changes();
    assert!(rt.state.note.is_some(), "a blank comment keeps asking");
    rt.state.note_text = TextEdit::new("please add a test");
    rt.state.send_changes();
    let pr = find_pr(&rt, 1, R5501).unwrap();
    assert_eq!((pr.my_review.as_deref(), pr.requested), (Some("CHANGES_REQUESTED"), false));
    assert!(rt.state.note.is_none());
}

fn urls(ids: &[&str]) -> std::collections::HashSet<String> {
    ids.iter().map(|p| format!("https://github.com/{p}")).collect()
}

fn cmd_click(rt: &mut Runtime<App>, key: &str) {
    let cmd = rune::Mods { cmd: true, ..Default::default() };
    rt.physical_input(rune::PhysicalInput::Modifiers(cmd));
    click(rt, key);
    rt.physical_input(rune::PhysicalInput::Modifiers(rune::Mods::default()));
}

#[test]
fn a_marquee_over_empty_space_selects_the_cards_it_touches() {
    let mut rt = runtime(github::MINE);
    let [cx, cy, cw, ch] = rt.state.rects[&format!("card:{FLEET88}")];
    let [bx, by, bw, _] = rt.state.rects["board"];
    let (sx, sy) = (bx + bw - 6.0, by + 2.0);
    rt.pointer_down(sx, sy);
    for i in 1..=8 {
        let k = i as f32 / 8.0;
        rt.pointer_move(sx + (cx + cw / 2.0 - sx) * k, sy + (cy + ch / 2.0 - sy) * k);
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.find("marquee").is_some());
    assert!(rt.state.selected.contains(FLEET88), "{:?}", rt.state.selected);
    assert!(!rt.state.selected.contains(PSY));
    rt.pointer_up(cx + cw / 2.0, cy + ch / 2.0);
    settle(&mut rt);
    assert!(rt.state.marquee.is_none() && rt.find("marquee").is_none(), "{:?}", rt.state.marquee);
    assert!(!rt.state.selected.is_empty());
    let (x, y) = (bx + bw - 6.0, by + 2.0);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert!(rt.state.selected.is_empty(), "a background click clears");
}

#[test]
fn cmd_click_toggles_selection_and_plain_click_opens() {
    let mut rt = runtime(github::MINE);
    cmd_click(&mut rt, "card-acme-fleet-88");
    cmd_click(&mut rt, "card-casassg-psy-77");
    assert_eq!(rt.state.selected, urls(&["acme/fleet/pull/88", "casassg/psy/pull/77"]));
    assert!(rt.state.detail.is_none());
    cmd_click(&mut rt, "card-casassg-psy-77");
    assert_eq!(rt.state.selected, urls(&["acme/fleet/pull/88"]));
    click(&mut rt, "card-acme-fleet-91");
    assert!(rt.state.selected.is_empty() && rt.state.detail.is_some());
}

#[test]
fn dragging_a_selected_card_carries_the_selection() {
    let mut rt = runtime(github::MINE);
    cmd_click(&mut rt, "card-acme-fleet-88");
    cmd_click(&mut rt, "card-acme-fleet-91");
    drag(&mut rt, "card-acme-fleet-88", "lane-2");
    let l = rt.state.lanes();
    assert_eq!((l.lane_of(FLEET88), l.lane_of("https://github.com/acme/fleet/pull/91")), (2, 2));
    assert!(rt.state.selected.is_empty());
}

#[test]
fn carrying_a_selection_to_a_new_lane_keeps_it_together() {
    let mut rt = runtime(github::MINE);
    cmd_click(&mut rt, "card-acme-fleet-88");
    cmd_click(&mut rt, "card-casassg-psy-77");
    let [x, y, w, _] = rt.state.rects["lane:2"];
    drag_to(&mut rt, "card-acme-fleet-88", x + w / 2.0, y - 4.0);
    let l = rt.state.lanes();
    assert_ne!(l.lane_of(FLEET88), 2);
    assert_eq!(l.lane_of(FLEET88), l.lane_of(PSY));
}

#[test]
fn dragging_an_unselected_card_drops_the_selection() {
    let mut rt = runtime(github::MINE);
    cmd_click(&mut rt, "card-acme-fleet-88");
    drag(&mut rt, "card-acme-fleet-91", "lane-2");
    let l = rt.state.lanes();
    assert_eq!(l.lane_of("https://github.com/acme/fleet/pull/91"), 2);
    assert_eq!(l.lane_of(FLEET88), 1);
    assert!(rt.state.selected.is_empty());
}

#[test]
fn a_cancelled_multi_drag_changes_nothing() {
    let mut rt = runtime(github::MINE);
    cmd_click(&mut rt, "card-acme-fleet-88");
    cmd_click(&mut rt, "card-acme-fleet-91");
    let before = rt.state.saved.clone();
    let (x, y) = rt.find("card-acme-fleet-88").unwrap();
    rt.pointer_down(x, y);
    let (tx, ty) = rt.find("lane-2").unwrap();
    for i in 1..=8 {
        let k = i as f32 / 8.0;
        rt.pointer_move(x + (tx - x) * k, y + (ty - y) * k);
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.state.drag.is_some());
    rt.key(rune::KeyEvent { key: rune::Key::Escape, mods: rune::Mods::default() });
    rt.pointer_up(tx, ty);
    for _ in 0..5 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(rt.state.saved, before);
}
