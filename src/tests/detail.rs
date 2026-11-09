use super::*;

#[test]
fn a_click_opens_the_detail_and_the_scrim_or_escape_closes_it() {
    let mut rt = runtime(github::MINE);
    click(&mut rt, "card-acme-fleet-88");
    assert_eq!(rt.state.detail.as_ref().map(|d| d.url.as_str()), Some(FLEET88));
    assert!(rt.find("detail").is_some() && rt.find("merge").is_some() && rt.find("close").is_some());
    rt.pointer_down(4.0, 4.0);
    rt.pointer_up(4.0, 4.0);
    settle(&mut rt);
    assert!(rt.state.detail.is_none(), "a click outside the card closes it");
    click(&mut rt, "card-acme-fleet-88");
    assert!(on_key(&mut rt.state, &KeyEvent { key: Key::Escape, mods: Default::default() }, &mut Tx::default()));
    assert!(rt.state.detail.is_none());
}

#[test]
fn merge_is_gated_on_mergeability_and_closed_prs_have_no_actions() {
    for (state, blocked) in [("CLEAN", false), ("HAS_HOOKS", false), ("UNSTABLE", false), ("BEHIND", true), ("BLOCKED", true), ("DIRTY", true), ("DRAFT", true), ("UNKNOWN", true)] {
        assert_eq!(github::merge_block(Some(state)).is_some(), blocked, "{state}");
    }
    assert!(github::merge_block(None).is_some(), "still loading");

    let mut rt = runtime(github::MINE);
    rt.state.detail = Some(Detail { url: FLEET88.into(), info: Some(github::Info { merge_state: "CLEAN".into(), ..Default::default() }) });
    settle(&mut rt);
    click(&mut rt, "merge");
    assert!(rt.state.detail.is_none(), "merged, so the detail closes");
    assert_eq!(rt.state.prs[0].iter().find(|p| p.url == FLEET88).unwrap().state, "MERGED");

    rt.state.detail = Some(Detail { url: "https://github.com/acme/pipelines/pull/1199".into(), info: None });
    settle(&mut rt);
    assert!(rt.find("merge").is_none() && rt.find("close").is_none() && rt.find("open").is_some(), "merged PR keeps only Open");
}

#[test]
fn rebase_only_shows_when_behind_and_a_blocked_merge_does_nothing() {
    let mut rt = runtime(github::MINE);
    let set = |rt: &mut Runtime<App>, ms: &str| {
        rt.state.detail = Some(Detail { url: FLEET88.into(), info: Some(github::Info { merge_state: ms.into(), ..Default::default() }) });
        settle(rt);
    };
    set(&mut rt, "CLEAN");
    assert!(rt.find("rebase").is_none());
    set(&mut rt, "BEHIND");
    assert!(rt.find("rebase").is_some());
    click(&mut rt, "merge");
    assert_eq!(rt.state.prs[0].iter().find(|p| p.url == FLEET88).unwrap().state, "OPEN", "behind PRs don't merge");
    click(&mut rt, "rebase");
    assert!(rt.state.detail.is_some(), "rebase keeps the detail open to show the new state");
}

#[test]
fn publish_shows_only_on_drafts_and_marks_them_ready() {
    let wiki = "https://github.com/casassg/wiki/pull/412";
    let mut rt = runtime(github::MINE);
    rt.state.detail = Some(Detail { url: FLEET88.into(), info: None });
    settle(&mut rt);
    assert!(rt.find("publish").is_none(), "not a draft");
    rt.state.detail = Some(Detail { url: wiki.into(), info: None });
    settle(&mut rt);
    click(&mut rt, "publish");
    assert!(!rt.state.prs[0].iter().find(|p| p.url == wiki).unwrap().draft);
    assert!(rt.state.detail.is_some() && rt.find("publish").is_none(), "the detail stays, without the button");
}
