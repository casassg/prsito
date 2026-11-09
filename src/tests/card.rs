use super::*;

#[test]
fn the_x_shows_on_hover_and_needs_two_clicks_to_close() {
    let mut rt = runtime(github::MINE);
    assert!(rt.find("close-card-acme-fleet-88").is_some());
    assert!(rt.find("close-card-acme-pipelines-1199").is_none(), "merged PRs have no X");
    let (x, y) = rt.find("close-card-acme-fleet-88").unwrap();
    rt.pointer_move(x, y);
    settle(&mut rt);
    click(&mut rt, "close-card-acme-fleet-88");
    assert_eq!(rt.state.armed.as_ref().map(|a| a.0.as_str()), Some(FLEET88));
    assert!(rt.state.detail.is_none(), "the X doesn't open the card");
    assert_eq!(rt.state.prs[0].iter().find(|p| p.url == FLEET88).unwrap().state, "OPEN");
    click(&mut rt, "close-card-acme-fleet-88");
    assert_eq!(rt.state.prs[0].iter().find(|p| p.url == FLEET88).unwrap().state, "CLOSED");
    assert!(rt.state.armed.is_none());
}

#[test]
fn an_armed_x_disarms_after_the_window_or_a_click_elsewhere() {
    let mut s = App::new(Saved::default(), None, false);
    fixture(&mut s, github::MINE);
    s.arm_close(FLEET88);
    s.armed.as_mut().unwrap().1 -= CONFIRM + Duration::from_secs(1);
    s.arm_close(FLEET88);
    assert_eq!(s.prs[0].iter().find(|p| p.url == FLEET88).unwrap().state, "OPEN", "a stale arm only re-arms");
    assert!(s.armed.is_some());
    s.arm_close("https://github.com/acme/fleet/pull/91");
    assert_eq!(s.armed.as_ref().map(|a| a.0.as_str()), Some("https://github.com/acme/fleet/pull/91"), "another PR re-arms instead of closing");
}

#[test]
fn a_review_cards_x_removes_me_instead_of_closing() {
    let mut rt = runtime(github::REVIEWS);
    assert!(rt.find("close-card-acme-pipelines-5490").is_none(), "already reviewed, nothing to drop");
    let (x, y) = rt.find("close-card-acme-fleet-5501").unwrap();
    rt.pointer_move(x, y);
    settle(&mut rt);
    click(&mut rt, "close-card-acme-fleet-5501");
    assert_eq!(rt.state.armed.as_ref().map(|a| a.0.as_str()), Some(R5501));
    click(&mut rt, "close-card-acme-fleet-5501");
    assert!(find_pr(&rt, 1, R5501).is_none(), "never reviewed, so it leaves the board");
    assert!(rt.state.prs[1].iter().all(|p| p.state != "CLOSED"), "nobody's PR got closed");

    let mut s = App::new(Saved::default(), None, false);
    fixture(&mut s, github::REVIEWS);
    s.prs[1][0].my_review = Some("COMMENTED".into());
    s.arm_close(R5501);
    s.arm_close(R5501);
    let pr = s.prs[1].iter().find(|p| p.url == R5501).unwrap();
    assert_eq!((pr.requested, pr.state.as_str()), (false, "OPEN"), "a commented one stays, only unrequested");
}
