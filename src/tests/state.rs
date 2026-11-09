use super::*;

#[test]
fn lanes_are_created_by_drop_and_vanish_when_empty() {
    let mut l = Lanes::default();
    assert_eq!(l.lane_of("a"), INBOX);
    let id = l.move_card("a", Target::New(9));
    l.move_card("b", Target::Lane(id));
    assert_eq!(l.lanes.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Lane 1", "Inbox"]);
    l.move_card("a", Target::Lane(INBOX));
    assert_eq!(l.lanes.len(), 2, "b still holds the lane");
    l.prune(&["a"], 0);
    assert_eq!(l.lanes.len(), 2, "b is only hidden (a filter, a shorter window), so its lane stays");
    l.prune(&["a"], 40 * 86_400);
    assert_eq!(l.lanes.len(), 1, "b unseen for 40 days, so the lane went with it");
    assert!(l.seen.is_empty());
    assert_eq!(l.move_card("a", Target::Lane(99)), INBOX, "a missing lane falls back to Inbox");
    let top = l.move_card("c", Target::New(0));
    let mid = l.move_card("d", Target::New(1));
    assert_eq!(l.lanes.iter().map(|l| l.id).collect::<Vec<_>>(), [top, mid, INBOX], "new lanes go where they're dropped");
}

#[test]
fn lanes_reorder_above_inbox_only() {
    let mut l = Lanes::default();
    let a = l.move_card("a", Target::New(9));
    let b = l.move_card("b", Target::New(9));
    l.reorder(b, 0);
    assert_eq!(l.lanes.iter().map(|l| l.id).collect::<Vec<_>>(), [b, a, INBOX]);
    l.reorder(b, 9);
    assert_eq!(l.lanes.iter().map(|l| l.id).collect::<Vec<_>>(), [a, b, INBOX]);
    l.reorder(INBOX, 0);
    assert_eq!(l.lanes.last().map(|l| l.id), Some(INBOX));
}

#[test]
fn saved_state_and_cache_round_trip_through_sqlite() {
    let dir = std::env::temp_dir().join(format!("prsito-test-{}", std::process::id()));
    let path = dir.join("prsito.db");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(store::load(&path), Saved::default(), "a new database starts empty");
    assert_eq!(store::load_cache(&path), None, "no cache yet");

    let mut saved = Saved {
        settings: store::Settings { days: 3, interval: 300, include_closed: false, filter: "org:x".into(), theme: "black".into(), menu_bar: true },
        ..Default::default()
    };
    let id = saved.boards[0].move_card("a", Target::New(9));
    saved.boards[0].prune(&["a"], 42);
    if let Some(l) = saved.boards[0].lane_mut(id) {
        l.collapsed = true;
    }
    saved.boards[1].move_card("b", Target::New(9));
    store::save(&path, &saved).unwrap();
    assert_eq!(store::load(&path), saved);

    let prs = [vec![pr("OPEN", true, Ci::Pending, Review::Changes)], Vec::new()];
    store::save_cache(&path, &prs).unwrap();
    assert_eq!(store::load_cache(&path), Some(prs));
    assert_eq!(store::load(&path), saved, "the cache leaves settings alone");
    std::fs::remove_dir_all(&dir).unwrap();

    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("state.json"), r#"{"settings":{"days":9},"boards":[{"lanes":[]},{}]}"#).unwrap();
    let imported = store::load(&path);
    assert_eq!(imported.settings.days, 9, "an old state.json is imported");
    assert_eq!(imported.boards[0].lanes.last().map(|l| l.id), Some(INBOX), "Inbox comes back");
    assert_eq!(store::load(&path).settings.days, 9, "and now lives in the database");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn only_a_higher_release_counts_as_newer() {
    use crate::update::newer;
    assert!(newer("0.4.0", "0.3.9"));
    assert!(newer("v0.10.0", "0.9.0"), "numeric, not lexical, and a v prefix is fine");
    assert!(newer("1.0.0", "0.99"));
    assert!(!newer("0.3.0", "0.3.0"));
    assert!(!newer("0.2.0", "0.3.0"));
    assert!(!newer("", "0.3.0"));
    assert!(!newer("nightly", "0.3.0"));
    assert!(!newer("0.4.0", ""));
}
