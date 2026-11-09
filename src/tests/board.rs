use super::*;

#[test]
fn cmd_f_puts_the_caret_in_search_and_selects_what_is_there() {
    let mut rt = runtime(github::MINE);
    rt.on_key = Some(on_key);
    let key = |key| KeyEvent { key, mods: Default::default() };
    rt.key(key(Key::Command('f')));
    rt.frame(1.0 / 60.0);
    rt.key(key(Key::Text("fleet".into())));
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.search.text, "fleet", "typing goes to the search");
    assert_eq!(rt.state.visible(github::MINE).len(), 3);
    rt.key(key(Key::Command('f')));
    rt.frame(1.0 / 60.0);
    rt.key(key(Key::Text("wiki".into())));
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.search.text, "wiki", "a second ⌘F replaces the old query");
    rt.key(key(Key::Escape));
    rt.frame(1.0 / 60.0);
    assert!(rt.state.search.text.is_empty() && rt.state.search.focused, "Escape clears first");
    rt.key(key(Key::Escape));
    rt.frame(1.0 / 60.0);
    assert!(!rt.state.search.focused, "then lets go, so the field shrinks back");
}

#[test]
fn search_filters_by_repo_number_title_and_author() {
    let mut rt = runtime(github::REVIEWS);
    let mut n = |q: &str| {
        rt.state.search = TextEdit::new(q);
        rt.state.visible(github::REVIEWS).iter().map(|p| p.number).collect::<Vec<_>>()
    };
    assert_eq!(n("").len(), 5, "an empty search shows everything");
    assert_eq!(n("  ").len(), 5, "so does a blank one");
    assert_eq!(n("SERVING"), [5502, 5470], "repo, case blind");
    assert_eq!(n("#5490"), [5490], "number");
    assert_eq!(n("warmup"), [5502], "title");
    assert_eq!(n("alice"), [5501, 5490], "author");
    assert_eq!(n("fleet alice quotas"), [5501], "every word must match");
    assert!(n("nope").is_empty());

    rt.update(|s, _| s.search = TextEdit::new("warmup"));
    rt.frame(1.0 / 60.0);
    assert!(rt.find("card-acme-serving-5502").is_some());
    assert!(rt.find("card-acme-serving-5470").is_none(), "non matches leave the board at once");
    rt.update(|s, _| s.tab = github::MINE);
    rt.update(|s, _| s.search = TextEdit::new("kueue"));
    rt.frame(1.0 / 60.0);
    assert!(rt.find("lane-1").is_some() && rt.find("lane-2").is_none(), "lanes without a match hide while searching");
    rt.update(|s, _| s.tab = github::REVIEWS);
    rt.update(|s, _| s.search = TextEdit::new(""));
    rt.frame(1.0 / 60.0);
    assert!(rt.find("card-acme-serving-5470").is_some(), "clearing brings them back");
}

#[test]
fn collapse_and_rename_a_lane() {
    let mut rt = runtime(github::MINE);
    let (x, y) = rt.find("lane-1/head/chevron").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert!(rt.state.lanes().lanes[0].collapsed);
    assert!(rt.find("card-acme-fleet-91").is_none(), "collapsed lane hides its cards");

    rt.state.start_rename(1);
    rt.frame(1.0 / 60.0);
    rt.state.rename = TextEdit::new("Infra");
    rt.state.finish_rename();
    assert_eq!(rt.state.lanes().lanes[0].name, "Infra");
    rt.state.start_rename(1);
    rt.state.rename = TextEdit::new("   ");
    rt.state.finish_rename();
    assert_eq!(rt.state.lanes().lanes[0].name, "Infra", "blank names are ignored");
}

#[test]
fn reviews_tab_has_its_own_columns_and_lanes() {
    let rt = runtime(github::REVIEWS);
    assert!(rt.find("col-3").is_some() && rt.find("col-4").is_none());
    assert_eq!(rt.state.lanes().lanes.len(), 1, "reviews start with only Inbox");
    assert!(rt.find("card-acme-serving-5470").is_some());
}

#[test]
fn landing_a_fetch_keeps_hidden_lanes_and_errors_keep_cards() {
    let mut s = App::new(Saved::default(), None, false);
    fixture(&mut s, github::MINE);
    let keep: Vec<Pr> = s.prs[0].iter().filter(|p| !p.url.contains("fleet")).cloned().collect();
    s.land(github::MINE, Ok(keep));
    assert!(has(s.lanes(), 1), "Fleet PRs are only hidden, their lane stays");
    assert!(has(s.lanes(), 2));
    let n = s.prs[0].len();
    s.land(github::MINE, Err("boom".into()));
    s.land(github::REVIEWS, Ok(Vec::new()));
    assert_eq!((s.prs[0].len(), s.error[0].as_deref()), (n, Some("boom")), "a Reviews success keeps the Mine error");
}

#[test]
fn done_prs_stack_fan_on_hover_and_list_on_click() {
    let mut rt = runtime(github::MINE);
    let stack = "lane-2/cells/cell-4/stack";
    assert!(rt.find(stack).is_some());
    assert!(rt.find("card-acme-pipelines-1188").is_none(), "stacked PRs are only slivers");
    assert!(rt.find("lane-1/cells/cell-4/stack").is_none(), "a single done PR is a plain card");
    assert!(rt.find("card-acme-fleet-85").is_some());
    let (x, y) = rt.find(stack).unwrap();
    rt.pointer_move(x, y);
    settle(&mut rt);
    assert_eq!(rt.state.fanned, Some((github::MINE, 2)));
    click(&mut rt, stack);
    assert!(rt.find(stack).is_none());
    assert!(rt.find("card-acme-pipelines-1188").is_some() && rt.find("card-acme-pipelines-1180").is_some());
    assert!(rt.state.detail.is_none(), "expanding doesn't open a PR");
    click(&mut rt, "lane-2/cells/cell-4/fold");
    assert!(rt.find(stack).is_some() && rt.find("card-acme-pipelines-1188").is_none());
    rt.state.tab = github::REVIEWS;
    settle(&mut rt);
    assert!(rt.find("fold").is_none(), "each board keeps its own stacks");
}

#[test]
fn a_stale_fetch_landing_after_a_close_keeps_the_pr_closed() {
    let mut s = App::new(Saved::default(), None, false);
    fixture(&mut s, github::MINE);
    let stale = s.prs[0].clone();
    s.arm_close(FLEET88);
    s.arm_close(FLEET88);
    s.land(0, Ok(stale.clone()));
    let fleet = |s: &App| s.prs[0].iter().find(|p| p.url == FLEET88).unwrap().state.clone();
    assert_eq!(fleet(&s), "CLOSED", "the fetch started before the close");
    let mut reopened = stale;
    reopened.iter_mut().find(|p| p.url == FLEET88).unwrap().updated += 60;
    s.land(0, Ok(reopened));
    assert_eq!(fleet(&s), "OPEN", "a newer copy from GitHub wins");
}
