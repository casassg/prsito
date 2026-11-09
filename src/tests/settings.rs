use super::*;

#[test]
fn settings_save_as_typed_and_clamp_and_keep_old_values_on_garbage() {
    let mut s = App::new(Saved::default(), None, false);
    s.interval = TextEdit::new("300");
    s.apply_settings();
    assert_eq!(s.saved.settings.interval, 300);
    assert!(!s.refetch_soon, "the interval alone needs no new fetch");
    s.days = TextEdit::new("0");
    s.interval = TextEdit::new("abc");
    s.filter = TextEdit::new("  org:x ");
    let generation = s.generation;
    s.apply_settings();
    assert_eq!((s.saved.settings.days, s.saved.settings.interval), (1, 300));
    assert_eq!(s.saved.settings.filter, "org:x");
    assert!(s.refetch_soon && s.generation > generation, "a new search drops the running fetch");
    assert_eq!(s.days.text, "0", "typing isn't rewritten under the caret");
    s.close_settings();
    assert_eq!(s.days.text, "1", "closing shows what was saved");
}

#[test]
fn the_window_and_closed_toggle_filter_the_board_at_once() {
    let mut s = App::new(Saved::default(), None, false);
    fixture(&mut s, github::MINE);
    let n = |s: &App| s.visible(github::MINE).len();
    assert_eq!(n(&s), 10);
    s.toggle_closed();
    assert_eq!(n(&s), 9, "the closed PR hides");
    s.toggle_closed();
    s.days = TextEdit::new("1");
    s.apply_settings();
    assert_eq!(n(&s), 7, "the PR merged 20h ago stays, those done 44h and more ago go");
    assert!(s.visible(github::MINE).iter().all(|p| !p.url.ends_with("/85")));
}

#[test]
fn a_missing_or_unknown_theme_is_picked_from_macos_on_launch() {
    let launch = |theme: &str, dark: bool| {
        let mut s = App::new(Saved::default(), None, false);
        s.saved.settings.theme = theme.into();
        fixture(&mut s, github::MINE);
        let mut rt = Runtime::new(s, view::view, (1180.0, 760.0), Env { system_dark: dark, scale: 2.0 });
        for _ in 0..3 {
            rt.frame(1.0 / 60.0);
        }
        rt.state.saved.settings.theme.clone()
    };
    assert_eq!(launch("", true), "black");
    assert_eq!(launch("", false), "white");
    assert_eq!(launch("navy", false), "white", "an old theme id counts as unset");
    assert_eq!(launch("accent", false), "accent", "a saved pick is kept");
}

#[test]
fn settings_pick_a_theme() {
    let mut rt = runtime(github::MINE);
    assert_eq!(rt.state.saved.settings.theme, "black", "a new install saves the theme macOS is in");
    rt.state.settings_open = true;
    settle(&mut rt);
    click(&mut rt, "theme-black");
    assert_eq!(rt.state.saved.settings.theme, "black");
    click(&mut rt, "theme-white");
    assert_eq!(rt.state.saved.settings.theme, "white");
    click(&mut rt, "theme-accent");
    assert_eq!(rt.state.saved.settings.theme, "accent");
}

#[test]
fn the_menu_bar_lists_my_prs_needing_attention_then_ready_to_merge() {
    assert!(statusbar::entries(&[]).is_empty());
    let mut s = App::new(Saved::default(), None, false);
    assert!(!s.saved.settings.menu_bar, "the menu bar item is opt in");
    fixture(&mut s, github::MINE);
    let got: Vec<(bool, String)> = statusbar::entries(&s.prs[github::MINE]).into_iter().map(|e| (e.ready, e.detail)).collect();
    assert_eq!(got, [
        (false, "fleet #91 · CI failed".to_string()),
        (false, "pipelines #1203 · changes requested".into()),
        (true, "pipelines #1210 · approved, CI green".into()),
    ], "drafts, pending CI, unapproved and merged PRs stay out; newest first in each group");
    s.toggle_menu_bar();
    assert!(s.saved.settings.menu_bar);
}

#[test]
fn settings_check_now_reports_up_to_date_and_offers_a_found_release() {
    let mut rt = runtime(github::MINE);
    rt.state.settings_open = true;
    settle(&mut rt);
    assert!(rt.state.update_status.is_none());
    click(&mut rt, "check-update");
    assert_eq!(rt.state.update_status.as_deref(), Some("Up to date"), "headless finds nothing at once");
    rt.state.update = Some(crate::update::Release { version: "9.9.9".into(), url: String::new() });
    settle(&mut rt);
    click(&mut rt, "check-update");
    assert!(rt.state.updating, "with a release found the button installs it");
}
