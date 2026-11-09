use super::*;

#[test]
fn my_prs_land_in_the_right_column() {
    let col = |p: Pr| github::column(github::MINE, &p);
    assert_eq!(col(pr("OPEN", true, Ci::Fail, Review::None)), 0, "draft wins over CI");
    assert_eq!(col(pr("OPEN", false, Ci::Pending, Review::None)), 1);
    assert_eq!(col(pr("OPEN", false, Ci::Fail, Review::None)), 2);
    assert_eq!(col(pr("OPEN", false, Ci::Pending, Review::Changes)), 2, "changes requested beats running CI");
    assert_eq!(col(pr("OPEN", false, Ci::Pass, Review::None)), 3);
    assert_eq!(col(pr("OPEN", false, Ci::None, Review::None)), 3, "no CI counts as ready");
    assert_eq!(col(pr("MERGED", false, Ci::Fail, Review::None)), 4);
    assert_eq!(col(pr("CLOSED", true, Ci::None, Review::None)), 4);
}

#[test]
fn reviews_land_in_the_right_column() {
    let col = |requested: bool, mine: Option<&str>, state: &str| {
        let mut p = pr(state, false, Ci::Pass, Review::None);
        p.requested = requested;
        p.my_review = mine.map(String::from);
        github::column(github::REVIEWS, &p)
    };
    assert_eq!(col(true, None, "OPEN"), 0);
    assert_eq!(col(true, Some("COMMENTED"), "OPEN"), 0, "re-requested goes back to To review");
    assert_eq!(col(false, Some("COMMENTED"), "OPEN"), 1);
    assert_eq!(col(false, Some("CHANGES_REQUESTED"), "OPEN"), 1);
    assert_eq!(col(false, Some("APPROVED"), "OPEN"), 2);
    assert_eq!(col(false, None, "OPEN"), 0);
    assert_eq!(col(false, Some("APPROVED"), "MERGED"), 3);
}

#[test]
fn graphql_results_parse_and_merge() {
    let q = Query { days: 7, include_closed: false, filter: " org:x ".into() };
    let searches = github::searches(github::REVIEWS, &q, github::parse_time("2026-11-06T00:00:00Z").unwrap());
    assert_eq!(searches[0].1, "is:pr archived:false is:open user-review-requested:@me sort:updated-desc org:x");
    assert_eq!(searches[1].1, "is:pr archived:false reviewed-by:@me -author:@me updated:>=2026-10-30 sort:updated-desc org:x");
    for query in [github::search_query(&["requested", "reviewed"]), github::details_query()] {
        assert_eq!(query.matches('{').count(), query.matches('}').count(), "{query}");
    }
    let t = |s: &str| github::parse_time(s).unwrap();
    let node = |id: &str, state: &str, updated: &str| serde_json::json!({ "id": id, "state": state, "updatedAt": updated });
    let mut hits = github::hits("requested", &serde_json::json!([node("a", "OPEN", "2026-11-01T00:00:00Z")]));
    hits.extend(github::hits("reviewed", &serde_json::json!([
        node("a", "OPEN", "2026-11-01T00:00:00Z"), node("b", "CLOSED", "2026-11-05T00:00:00Z"),
        node("c", "MERGED", "2026-11-04T00:00:00Z"), node("d", "OPEN", "2026-11-03T00:00:00Z"), { "id": null },
    ])));
    assert_eq!(hits.len(), 5, "a node without an id is skipped");
    assert!(github::hits("x", &serde_json::json!({})).is_empty());

    let cached = |id: &str, updated: &str, ci: Ci| { let mut p = pr("OPEN", false, ci, Review::None); p.id = id.into(); p.url = id.into(); p.updated = t(updated); p };
    let prev = [cached("a", "2026-10-01T00:00:00Z", Ci::Pass), cached("c", "2026-11-04T00:00:00Z", Ci::Pass), cached("d", "2026-11-03T00:00:00Z", Ci::Pending)];
    let (stale, pending) = github::plan(&hits, &prev, false);
    assert_eq!(stale, ["a", "b"], "a changed, b is new; the duplicate hit on a counts once");
    assert_eq!(pending, ["d"], "d is unchanged but its CI was still running");
    let (stale, pending) = github::plan(&hits, &prev, true);
    assert_eq!((stale.len(), pending.len()), (4, 0), "a full refresh refetches everything");
    assert_eq!(github::plan(&hits, &[], false).0.len(), 4, "with no cache everything is new");
    let mut old = cached("a", "2026-11-01T00:00:00Z", Ci::Pass);
    old.id.clear();
    assert_eq!(github::plan(&hits, &[old], false).0.len(), 4, "a cache row without an id (from an older version) never matches");
    let mut old = cached("c", "2026-11-04T00:00:00Z", Ci::Pass);
    old.author.clear();
    assert_eq!(github::plan(&hits, &[old], false).0.len(), 4, "a cache row without an author (from an older version) refetches");

    let details = |id: &str, state: &str, updated: &str| serde_json::json!({
        "id": id, "url": id, "number": 7, "title": "T", "isDraft": false, "state": state, "updatedAt": updated,
        "additions": 3, "deletions": 1, "reviewDecision": "APPROVED",
        "repository": { "nameWithOwner": "o/r" },
        "latestReviews": { "nodes": [{ "state": "COMMENTED", "author": { "login": "me" } }] },
        "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "state": "FAILURE" } } }] },
    });
    let mut a = details("a", "OPEN", "2026-11-01T00:00:00Z");
    a["author"] = serde_json::json!({ "login": "alice" });
    let fresh = github::details(&serde_json::json!({ "viewer": { "login": "me" }, "nodes": [a, details("b", "CLOSED", "2026-11-05T00:00:00Z"), null] }));
    assert_eq!(fresh.len(), 2);
    assert_eq!((fresh["a"].author.as_str(), fresh["b"].author.as_str()), ("alice", "ghost"), "a deleted author reads as ghost, so it doesn't refetch forever");
    assert_eq!((fresh["a"].ci, fresh["a"].review, fresh["a"].my_review.as_deref(), fresh["a"].additions), (Ci::Fail, Review::Approved, Some("COMMENTED"), 3));
    assert!(github::details(&serde_json::json!({})).is_empty());

    let prs = github::assemble(&hits, false, &fresh, &prev);
    let ids: Vec<&str> = prs.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["c", "d", "a"], "deduped, closed dropped, newest first, cached c and d kept");
    assert!(prs[2].requested && !prs[1].requested, "requested comes from which search found it");
    assert_eq!(prs[2].updated, t("2026-11-01T00:00:00Z"), "a came from the fresh details");
    assert_eq!(github::assemble(&hits, true, &fresh, &prev).len(), 4, "closed PRs show when asked");
    assert_eq!(github::assemble(&hits, true, &std::collections::HashMap::new(), &prev).len(), 3, "before details land, new PRs are left out");

    let ci = github::ci_states(&serde_json::json!({ "nodes": [
        { "id": "a", "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "state": "FAILURE" } } }] } },
        { "id": "d", "commits": { "nodes": [{ "commit": { "statusCheckRollup": null } }] } },
        { "id": "e", "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "state": "PENDING" } } }] } },
        null,
    ] }));
    assert_eq!((ci["a"], ci["d"], ci["e"], ci.len()), (Ci::Fail, Ci::None, Ci::Pending, 3));
}

#[test]
fn dates_round_trip() {
    let t = github::parse_time("2026-03-01T10:20:30Z").unwrap();
    assert_eq!(github::date(t), "2026-03-01");
    assert_eq!(github::date(t - 86_400), "2026-02-28");
    assert_eq!(github::parse_time("garbage"), None);
    assert_eq!(github::age(t, t), "now");
    assert_eq!(github::age(t, t - 3 * 86_400), "3d");
}

#[test]
#[ignore = "talks to GitHub with `gh auth token`"]
fn live_fetch_returns_prs() {
    let q = Query { days: 7, include_closed: true, filter: String::new() };
    for board in [github::MINE, github::REVIEWS] {
        let mut pages = 0;
        let t = std::time::Instant::now();
        let prs = github::fetch(board, &q, &[], false, |_| pages += 1).unwrap();
        assert!(pages >= 1);
        let cold = t.elapsed();
        let mut warm_pages = 0;
        let t = std::time::Instant::now();
        let warm = github::fetch(board, &q, &prs, false, |_| warm_pages += 1).unwrap();
        assert_eq!(warm.len(), prs.len());
        println!("board {board}: cold {cold:.1?} over {pages} detail pages, warm {:.1?} over {warm_pages}", t.elapsed());
        let cols = prs.iter().map(|p| github::column(board, p)).fold(vec![0; github::COLUMNS[board].len()], |mut c, i| { c[i] += 1; c });
        println!("board {board}: {} PRs, per column {cols:?}", prs.len());
    }
}

#[test]
fn info_keeps_only_unfinished_checks_and_the_approvers() {
    let n = serde_json::json!({
        "body": "hi", "mergeStateStatus": "BLOCKED",
        "latestReviews": { "nodes": [
            { "state": "APPROVED", "author": { "login": "alice" } },
            { "state": "CHANGES_REQUESTED", "author": { "login": "bob" } },
            { "state": "APPROVED", "author": null },
        ] },
        "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "contexts": { "nodes": [
            { "__typename": "CheckRun", "name": "ok", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "u0" },
            { "__typename": "CheckRun", "name": "unit", "status": "COMPLETED", "conclusion": "FAILURE", "detailsUrl": "u1" },
            { "__typename": "CheckRun", "name": "build", "status": "IN_PROGRESS", "conclusion": null, "detailsUrl": "u2" },
            { "__typename": "StatusContext", "context": "legacy", "state": "ERROR", "targetUrl": "u3" },
            { "__typename": "StatusContext", "context": "fine", "state": "SUCCESS", "targetUrl": "u4" },
        ] } } } }] },
    });
    let i = github::info_from(&n);
    let got: Vec<(&str, bool)> = i.checks.iter().map(|c| (c.name.as_str(), c.running)).collect();
    assert_eq!(got, [("unit", false), ("build", true), ("legacy", false)]);
    assert_eq!((i.body.as_str(), i.merge_state.as_str(), i.approvers.as_slice()), ("hi", "BLOCKED", &["alice".to_string()][..]));
    assert_eq!(github::info_from(&serde_json::json!(null)), github::Info::default(), "a missing node is empty, not a panic");
}
