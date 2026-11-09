use super::*;
use crate::cli;
use std::path::{Path, PathBuf};

/// A fresh directory with a database holding the fixture boards.
fn seeded(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("prsito-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = App::new(Saved::default(), None, false);
    fixture(&mut s, github::MINE);
    let path = dir.join("prsito.db");
    store::save(&path, &s.saved).unwrap();
    store::save_cache(&path, &s.prs).unwrap();
    path
}

fn run(path: &Path, args: &str) -> Result<String, String> {
    cli::run(path, &args.split_whitespace().collect::<Vec<_>>(), decline)
}

/// Says no to every action, so no test reaches GitHub.
fn decline(question: &str) -> Result<(), String> {
    Err(format!("declined: {question}"))
}

fn json(path: &Path, args: &str) -> serde_json::Value {
    serde_json::from_str(&run(path, &format!("{args} --json")).unwrap()).unwrap()
}

#[test]
fn lanes_count_cached_prs_per_lane() {
    let path = seeded("lanes");
    let lanes = json(&path, "lanes --board mine");
    let got: Vec<(String, u64)> = lanes.as_array().unwrap().iter().map(|l| (l["name"].as_str().unwrap().to_string(), l["prs"].as_u64().unwrap())).collect();
    assert_eq!(got, [("Fleet scheduling".into(), 3), ("Pipelines".into(), 5), ("Inbox".into(), 2)]);
    assert_eq!(json(&path, "lanes").as_array().unwrap().len(), 4, "both boards by default");
}

#[test]
fn ls_filters_by_lane_name_or_id_and_rejects_unknown_lanes() {
    let path = seeded("ls");
    let urls = |v: serde_json::Value| v.as_array().unwrap().iter().map(|p| p["url"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert!(run(&path, "ls --lane fleet").unwrap_err().contains("no lane"), "names match whole, not in part");
    let by_id = urls(json(&path, "ls --lane 1"));
    assert_eq!(by_id.len(), 3);
    assert!(by_id.contains(&FLEET88.to_string()));
    assert_eq!(urls(json(&path, "ls --lane inbox --board reviews")).len(), 5);
    assert_eq!(urls(json(&path, "ls")).len(), 15);
    assert!(run(&path, "ls --lane nope").unwrap_err().contains("no lane nope"));
    let text = run(&path, "ls --lane 1").unwrap();
    assert!(text.lines().all(|l| l.starts_with("mine\tFleet scheduling\t")), "{text}");
}

#[test]
fn spaced_lane_names_pass_as_one_argument() {
    let path = seeded("spaced");
    let out = cli::run(&path, &["ls", "--lane", "FLEET Scheduling", "--json"], decline).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 3);
    assert!(v.as_array().unwrap().iter().all(|p| p["lane"] == "Fleet scheduling" && p["board"] == "mine"));
}

#[test]
fn mv_moves_into_lanes_makes_new_ones_and_empties_old_ones() {
    let path = seeded("mv");
    assert_eq!(run(&path, "mv casassg/psy#77 pipelines").unwrap(), "moved casassg/psy#77 to Pipelines");
    assert_eq!(store::load(&path).boards[github::MINE].lane_of(PSY), 2);
    run(&path, &format!("mv {PSY} Later")).unwrap();
    let mine = &store::load(&path).boards[github::MINE];
    let later = mine.lane_of(PSY);
    assert_eq!(mine.lanes.iter().find(|l| l.id == later).unwrap().name, "Later");
    assert_eq!(mine.lanes.last().unwrap().id, INBOX, "a new lane goes above Inbox");
    run(&path, &format!("mv {PSY} inbox")).unwrap();
    let mine = &store::load(&path).boards[github::MINE];
    assert_eq!(mine.lane_of(PSY), INBOX);
    assert!(!has(mine, later), "the emptied lane goes");
    let v = json(&path, &format!("mv {R5501} Soon"));
    assert_eq!(v["board"], "reviews", "the board comes from the cache");
}

#[test]
fn bad_input_errors_before_touching_github() {
    let path = seeded("errors");
    assert!(run(&path, "mv nope x").unwrap_err().contains("isn't a PR"));
    assert!(run(&path, "mv o/r#x x").unwrap_err().contains("isn't a PR"));
    assert!(run(&path, "mv o/r#9 x").unwrap_err().contains("isn't in the cache"));
    assert!(run(&path, "approve https://github.com/o/r/pull/9").unwrap_err().contains("isn't in the cache"));
    assert!(run(&path, &format!("request-changes {R5501}")).unwrap_err().contains("needs a comment"));
    assert!(run(&path, &format!("request-changes {R5501} -m")).unwrap_err().contains("-m takes"));
    assert!(run(&path, &format!("frobnicate {R5501}")).unwrap_err().contains("unknown command frobnicate"));
    assert!(run(&path, "ls extra").unwrap_err().contains("usage"));
    assert!(run(&path, "ls --board both").unwrap_err().contains("mine or reviews"));
    assert!(run(&path, "ls --nope").unwrap_err().contains("unknown flag"));
    assert!(run(&path, "").unwrap().starts_with("usage"));
    assert!(run(&path, "help").unwrap().starts_with("usage"));
}

#[test]
fn actions_ask_first_and_stop_on_no() {
    let path = seeded("confirm");
    let err = run(&path, &format!("approve {R5501}")).unwrap_err();
    assert_eq!(err, "declined: approve acme/fleet#5501 \"feat: per-team gpu quotas\"?");
    let err = run(&path, &format!("request-changes {R5501} -m why")).unwrap_err();
    assert!(err.starts_with("declined: request-changes"), "{err}");
    let p = store::load_cache(&path).unwrap()[github::REVIEWS].iter().find(|p| p.url == R5501).cloned().unwrap();
    assert!(p.requested && p.my_review.is_none(), "a declined action changes nothing");
}

#[test]
fn pr_refs_parse_from_urls_and_short_form() {
    let u = FLEET88;
    assert_eq!(cli::parse("acme/fleet#88").unwrap(), u);
    assert_eq!(cli::parse(&format!("{u}/")).unwrap(), u);
    for bad in ["kitt#88", "/kitt#88", "squareup/kitt#", "https://github.com/squareup/kitt/issues/88", "https://github.com/squareup/kitt/pull/x"] {
        assert!(cli::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn install_links_the_binary_and_replaces_only_links() {
    let home = std::env::temp_dir().join(format!("prsito-test-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let (a, b) = (Path::new("/Applications/prsito.app/Contents/MacOS/prsito"), Path::new("/tmp/other-prsito"));
    assert!(!cli::installed(&home, a));
    cli::install(&home, a).unwrap();
    assert!(cli::installed(&home, a));
    assert!(!cli::installed(&home, b), "a link to another build isn't this one");
    cli::install(&home, b).unwrap();
    assert!(cli::installed(&home, b), "reinstalling repoints the link");
    let link = home.join(".local/bin/prsito");
    std::fs::remove_file(&link).unwrap();
    std::fs::write(&link, "#!/bin/sh").unwrap();
    assert!(cli::install(&home, a).unwrap_err().contains("isn't a link"));
    assert_eq!(std::fs::read_to_string(&link).unwrap(), "#!/bin/sh", "a real file is left alone");
}

#[test]
fn the_app_reloads_what_the_cli_writes_but_not_its_own_saves() {
    let path = seeded("watch");
    let mut s = App::new(store::load(&path), Some(path.clone()), false);
    s.prs = store::load_cache(&path).unwrap();
    s.watch = store::Watch::open(&path);
    assert!(!s.reload(), "nothing changed yet");
    s.persist();
    assert!(!s.reload(), "its own save isn't news");
    run(&path, &format!("mv {PSY} 1")).unwrap();
    s.drag = Some(DragState { what: Dragging::Card(FLEET88.into()), origin: [0.0; 4], dx: 0.0, dy: 0.0, over: None, act: None });
    assert!(!s.reload(), "a held drag waits");
    s.drag = None;
    assert!(s.reload());
    assert_eq!(s.saved.boards[github::MINE].lane_of(PSY), 1);
    assert!(!s.reload());
}

#[test]
fn settings_install_button_marks_the_tool_installed() {
    let mut rt = runtime(github::MINE);
    rt.state.settings_open = true;
    settle(&mut rt);
    assert!(!rt.state.cli);
    click(&mut rt, "install-cli");
    assert!(rt.state.cli, "headless installs nothing but lands at once");
}
