use super::*;

#[test]
fn right_click_opens_the_menu_with_the_same_actions() {
    let mut rt = runtime(github::MINE);
    let (x, y) = rt.find("card-acme-fleet-88").unwrap();
    rt.secondary_down(x, y);
    rt.secondary_up(x, y);
    settle(&mut rt);
    assert_eq!(rt.state.menu.as_ref().map(|m| m.url.as_str()), Some(FLEET88));
    assert!(rt.state.detail.is_none());
    rt.state.menu.as_mut().unwrap().merge_state = Some("BEHIND".into());
    settle(&mut rt);
    assert!(rt.find("m-rebase").is_some() && rt.find("m-open").is_some());
    click(&mut rt, "m-close");
    assert!(rt.state.menu.is_some() && rt.state.armed.is_some(), "Close in the menu confirms too");
    click(&mut rt, "m-close");
    assert!(rt.state.menu.is_none());
    assert_eq!(rt.state.prs[0].iter().find(|p| p.url == FLEET88).unwrap().state, "CLOSED");
}

#[test]
fn the_menu_copies_the_pr_url() {
    let mut rt = runtime(github::MINE);
    let board = rune::clipboard::memory();
    rt.use_clipboard(&board);
    let (x, y) = rt.find("card-acme-fleet-88").unwrap();
    rt.secondary_down(x, y);
    rt.secondary_up(x, y);
    settle(&mut rt);
    click(&mut rt, "m-copy");
    assert_eq!(board.text().as_deref().map(|t| t.as_str()), Some(FLEET88));
    assert!(rt.state.menu.is_none());
}

#[test]
fn the_menu_moves_a_card_to_a_lane_or_a_new_one() {
    let mut rt = runtime(github::MINE);
    let open_menu = |rt: &mut Runtime<App>| {
        let (x, y) = rt.find("card-acme-fleet-88").unwrap();
        rt.secondary_down(x, y);
        rt.secondary_up(x, y);
        settle(rt);
    };
    open_menu(&mut rt);
    let here = rt.state.lanes().lane_of(FLEET88);
    assert!(rt.find(&format!("m-lane-{here}")).is_some() && rt.find("m-lane-0").is_some());
    click(&mut rt, "m-lane-0");
    assert_eq!(rt.state.lanes().lane_of(FLEET88), INBOX);
    assert!(rt.state.menu.is_none());
    open_menu(&mut rt);
    let before = rt.state.lanes().lanes.len();
    let (_, y) = rt.find("m-newlane").unwrap();
    assert!(y < 760.0, "the menu stays inside the window");
    click(&mut rt, "m-newlane");
    let id = rt.state.lanes().lane_of(FLEET88);
    assert_ne!(id, INBOX);
    assert_eq!(rt.state.lanes().lanes.len(), before + 1);
    assert_eq!(rt.state.lanes().lanes.last().unwrap().id, INBOX, "the new lane sits above Inbox");
    assert_eq!(rt.state.renaming, Some(id), "and opens for naming");
}
