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
