use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ci {
    None,
    Pending,
    Fail,
    Pass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Review {
    None,
    Approved,
    Changes,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pr {
    /// GitHub's node id; empty in caches written before it was kept, so those refetch once.
    #[serde(default)]
    pub id: String,
    pub url: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    /// Login of who opened it; empty in caches written before it was kept, so those refetch once.
    #[serde(default)]
    pub author: String,
    pub draft: bool,
    /// OPEN, MERGED or CLOSED.
    pub state: String,
    /// Seconds since the epoch.
    pub updated: i64,
    /// When it merged or closed, in seconds since the epoch (0 while open).
    #[serde(default)]
    pub closed: i64,
    pub additions: u64,
    pub deletions: u64,
    pub ci: Ci,
    pub review: Review,
    /// The viewer's latest review state (APPROVED, CHANGES_REQUESTED, COMMENTED), if any.
    pub my_review: Option<String>,
    /// The viewer's review is requested.
    pub requested: bool,
}

pub const MINE: usize = 0;
pub const REVIEWS: usize = 1;
pub const COLUMNS: [&[&str]; 2] = [
    &["Draft", "Validating", "Needs attention", "Ready", "Done"],
    &["To review", "Waiting on author", "Approved", "Done"],
];

pub fn column(board: usize, pr: &Pr) -> usize {
    let done = pr.state != "OPEN";
    if board == MINE {
        if done {
            4
        } else if pr.draft {
            0
        } else if pr.ci == Ci::Fail || pr.review == Review::Changes {
            2
        } else if pr.ci == Ci::Pending {
            1
        } else {
            3
        }
    } else if done {
        3
    } else if pr.requested {
        0
    } else {
        match pr.my_review.as_deref() {
            Some("APPROVED") => 2,
            Some(_) => 1,
            None => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Query {
    pub days: u32,
    pub include_closed: bool,
    pub filter: String,
}

pub fn searches(board: usize, q: &Query, today: i64) -> Vec<(&'static str, String)> {
    let since = date(today - q.days as i64 * 86_400);
    let f = if q.filter.trim().is_empty() { String::new() } else { format!(" {}", q.filter.trim()) };
    let mut out = Vec::new();
    if board == MINE {
        out.push(("open", format!("is:pr archived:false author:@me is:open sort:updated-desc{f}")));
        out.push(("merged", format!("is:pr archived:false author:@me is:merged merged:>={since}{f}")));
        if q.include_closed {
            out.push(("closed", format!("is:pr archived:false author:@me is:closed is:unmerged closed:>={since}{f}")));
        }
    } else {
        out.push(("requested", format!("is:pr archived:false is:open user-review-requested:@me sort:updated-desc{f}")));
        out.push(("reviewed", format!("is:pr archived:false reviewed-by:@me -author:@me updated:>={since} sort:updated-desc{f}")));
    }
    out
}

/// The fields a card shows. `reviewDecision`, `additions`/`deletions` and the CI rollup are slow
/// for GitHub (seconds per 50 PRs), so they're asked by id only for PRs whose `updatedAt` moved.
const FIELDS: &str = "id url number title isDraft state updatedAt closedAt additions deletions reviewDecision
    author { login } repository { nameWithOwner }
    latestReviews(first: 20) { nodes { state author { login } } }
    commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }";

const CI_QUERY: &str = "query($ids: [ID!]!) { nodes(ids: $ids) { ... on PullRequest { id commits(last: 1) { nodes { commit { statusCheckRollup { state } } } } } } }";

/// PRs per search page.
const PAGE: usize = 100;
/// Most pages read per search: past that only the most recently updated show.
const MAX_PAGES: usize = 2;
/// PRs per details or CI lookup; GitHub times out (502) around a hundred.
const BATCH: usize = 25;

/// One page of each search, all in one request, asking only what tells whether a PR changed.
pub fn search_query(aliases: &[&str]) -> String {
    let vars: Vec<String> = aliases.iter().map(|a| format!("${a}: String!, ${a}_after: String")).collect();
    let fields: Vec<String> = aliases.iter().map(|a| format!(
        "{a}: search(query: ${a}, type: ISSUE, first: {PAGE}, after: ${a}_after) {{ pageInfo {{ hasNextPage endCursor }} nodes {{ ... on PullRequest {{ id updatedAt state }} }} }}"
    )).collect();
    format!("query({}) {{ {} }}", vars.join(", "), fields.join(" "))
}

pub fn details_query() -> String {
    format!("query($ids: [ID!]!) {{ viewer {{ login }} nodes(ids: $ids) {{ ... on PullRequest {{ {FIELDS} }} }} }}")
}

/// A PR a search found: enough to tell whether the cached copy is current.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub alias: &'static str,
    pub id: String,
    pub updated: i64,
    pub open: bool,
}

pub fn hits(alias: &'static str, nodes: &Value) -> Vec<Hit> {
    nodes.as_array().into_iter().flatten().filter_map(|n| Some(Hit {
        alias,
        id: n["id"].as_str()?.to_string(),
        updated: parse_time(n["updatedAt"].as_str().unwrap_or_default()).unwrap_or_default(),
        open: n["state"] == "OPEN",
    })).collect()
}

/// Which hits need their details fetched (new, changed, or all when `full`), and which cached
/// open PRs still need a CI look because their CI was pending. CI alone doesn't bump `updatedAt`.
pub fn plan(hits: &[Hit], prev: &[Pr], full: bool) -> (Vec<String>, Vec<String>) {
    let (mut stale, mut pending, mut seen) = (Vec::new(), Vec::new(), HashSet::new());
    for h in hits {
        if !seen.insert(h.id.as_str()) {
            continue;
        }
        match prev.iter().find(|p| p.id == h.id) {
            Some(p) if !full && p.updated == h.updated && !p.author.is_empty() => {
                if h.open && p.ci == Ci::Pending {
                    pending.push(h.id.clone());
                }
            }
            _ => stale.push(h.id.clone()),
        }
    }
    (stale, pending)
}

/// The board from what the searches found, taking each PR from the fresh details or else the
/// previous fetch, and skipping ones neither has yet.
pub fn assemble(hits: &[Hit], include_closed: bool, fresh: &HashMap<String, Pr>, prev: &[Pr]) -> Vec<Pr> {
    let mut out: Vec<Pr> = Vec::new();
    for h in hits {
        let Some(mut pr) = fresh.get(&h.id).or_else(|| prev.iter().find(|p| p.id == h.id)).cloned() else { continue };
        pr.requested = h.alias == "requested";
        if pr.state == "CLOSED" && !include_closed {
            continue;
        }
        match out.iter_mut().find(|p| p.id == pr.id) {
            Some(p) => p.requested |= pr.requested,
            None => out.push(pr),
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.updated));
    out
}

/// PRs by id from a `details_query` answer.
pub fn details(data: &Value) -> HashMap<String, Pr> {
    let viewer = data["viewer"]["login"].as_str().unwrap_or_default();
    data["nodes"].as_array().into_iter().flatten()
        .filter(|n| n["id"].is_string())
        .map(|n| { let pr = pr_from(n, viewer); (pr.id.clone(), pr) })
        .collect()
}

/// CI by id from a `CI_QUERY` answer.
pub fn ci_states(data: &Value) -> HashMap<String, Ci> {
    data["nodes"].as_array().into_iter().flatten().filter_map(|n| Some((n["id"].as_str()?.to_string(), ci_from(n)))).collect()
}

fn ci_from(n: &Value) -> Ci {
    match n["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["state"].as_str() {
        Some("SUCCESS") => Ci::Pass,
        Some("FAILURE" | "ERROR") => Ci::Fail,
        Some("PENDING" | "EXPECTED") => Ci::Pending,
        _ => Ci::None,
    }
}

fn pr_from(n: &Value, viewer: &str) -> Pr {
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let review = match n["reviewDecision"].as_str() {
        Some("APPROVED") => Review::Approved,
        Some("CHANGES_REQUESTED") => Review::Changes,
        _ => Review::None,
    };
    let my_review = n["latestReviews"]["nodes"].as_array().into_iter().flatten()
        .find(|r| r["author"]["login"].as_str() == Some(viewer))
        .map(|r| s(&r["state"]));
    Pr {
        id: s(&n["id"]),
        url: s(&n["url"]),
        repo: s(&n["repository"]["nameWithOwner"]),
        number: n["number"].as_u64().unwrap_or_default(),
        title: s(&n["title"]),
        author: n["author"]["login"].as_str().unwrap_or("ghost").to_string(),
        draft: n["isDraft"].as_bool().unwrap_or_default(),
        state: s(&n["state"]),
        updated: parse_time(n["updatedAt"].as_str().unwrap_or_default()).unwrap_or_default(),
        closed: n["closedAt"].as_str().and_then(parse_time).unwrap_or_default(),
        additions: n["additions"].as_u64().unwrap_or_default(),
        deletions: n["deletions"].as_u64().unwrap_or_default(),
        ci: ci_from(n),
        review,
        my_review,
        requested: false,
    }
}

static TOKEN: Mutex<Option<String>> = Mutex::new(None);

fn token(fresh: bool) -> Result<String, String> {
    let mut cached = TOKEN.lock().map_err(|e| e.to_string())?;
    if !fresh && let Some(t) = cached.as_ref() {
        return Ok(t.clone());
    }
    // An app opened from Finder gets a bare PATH, without Homebrew's.
    let out = ["gh", "/opt/homebrew/bin/gh", "/usr/local/bin/gh"].iter()
        .find_map(|gh| std::process::Command::new(gh).args(["auth", "token"]).output().ok())
        .ok_or("can't find gh, install it (brew install gh) and run `gh auth login`")?;
    let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || t.is_empty() {
        return Err("no GitHub token, run `gh auth login`".into());
    }
    *cached = Some(t.clone());
    Ok(t)
}

/// Runs the board's searches (cheap, one request a page), then fetches details and CI only for
/// PRs that are new or changed since `prev`, or all of them when `full`. Cached open PRs whose
/// CI was pending get a CI look too. Hands partial boards to `progress` as details land.
/// One request at a time, as GitHub asks: parallel heavy queries trip its secondary rate limit.
pub fn fetch(board: usize, q: &Query, prev: &[Pr], full: bool, mut progress: impl FnMut(Vec<Pr>)) -> Result<Vec<Pr>, String> {
    let mut live: Vec<(&'static str, String, Value)> = searches(board, q, now()).into_iter().map(|(a, s)| (a, s, Value::Null)).collect();
    let mut found = Vec::new();
    for _ in 0..MAX_PAGES {
        let aliases: Vec<&str> = live.iter().map(|(a, _, _)| *a).collect();
        let mut vars = serde_json::Map::new();
        for (alias, search, after) in &live {
            vars.insert(alias.to_string(), json!(search));
            vars.insert(format!("{alias}_after"), after.clone());
        }
        let page = post(&search_query(&aliases), Value::Object(vars))?;
        live.retain_mut(|(alias, _, after)| {
            found.extend(hits(alias, &page[*alias]["nodes"]));
            let info = &page[*alias]["pageInfo"];
            *after = info["endCursor"].clone();
            info["hasNextPage"] == true
        });
        if live.is_empty() {
            break;
        }
    }
    let (stale, pending) = plan(&found, prev, full);
    let mut fresh = HashMap::new();
    for ids in stale.chunks(BATCH) {
        fresh.extend(details(&post(&details_query(), json!({ "ids": ids }))?));
        progress(assemble(&found, q.include_closed, &fresh, prev));
    }
    let mut out = assemble(&found, q.include_closed, &fresh, prev);
    for ids in pending.chunks(BATCH) {
        let ci = ci_states(&post(CI_QUERY, json!({ "ids": ids }))?);
        for p in &mut out {
            if let Some(c) = ci.get(&p.id) {
                p.ci = *c;
            }
        }
    }
    Ok(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Merge,
    Rebase,
    Close,
    Publish,
    /// Back to draft: convertPullRequestToDraft.
    Draft,
    Approve,
    /// Needs a comment: GitHub refuses a changes request without one.
    RequestChanges,
    /// Drops the viewer's review request.
    Uninvolve,
    /// Dismisses the viewer's own review (approval or changes request).
    Dismiss,
}

/// Runs `act` on `pr`; `body` is the comment of a changes request.
pub fn act(act: Act, pr: &Pr, body: &str) -> Result<(), String> {
    let id = pr.id.as_str();
    let (query, input) = match act {
        Act::Uninvolve => {
            let me = post("query { viewer { login } }", json!({}))?["viewer"]["login"].as_str().unwrap_or_default().to_string();
            let url = format!("https://api.github.com/repos/{}/pulls/{}/requested_reviewers", pr.repo, pr.number);
            AGENT.delete(&url)
                .header("Authorization", &format!("Bearer {}", token(false)?))
                .header("User-Agent", "prsito")
                .header("Accept", "application/vnd.github+json")
                .force_send_body()
                .send(&json!({ "reviewers": [me] }).to_string())
                .map_err(|e| format!("GitHub: {e}"))?;
            return Ok(());
        }
        Act::Dismiss => {
            let v = post(
                "query($id: ID!) { viewer { login } node(id: $id) { ... on PullRequest { latestReviews(first: 50) { nodes { id state author { login } } } } } }",
                json!({ "id": id }),
            )?;
            let me = v["viewer"]["login"].as_str().unwrap_or_default().to_string();
            let rid = v["node"]["latestReviews"]["nodes"].as_array().into_iter().flatten()
                .find(|n| n["author"]["login"] == me.as_str() && matches!(n["state"].as_str(), Some("APPROVED" | "CHANGES_REQUESTED")))
                .and_then(|n| n["id"].as_str())
                .ok_or("no review of mine to dismiss")?
                .to_string();
            (
                "mutation($i: DismissPullRequestReviewInput!) { dismissPullRequestReview(input: $i) { clientMutationId } }",
                json!({ "pullRequestReviewId": rid, "message": "Dismissed from prsito" }),
            )
        }
        Act::Approve => (
            "mutation($i: AddPullRequestReviewInput!) { addPullRequestReview(input: $i) { clientMutationId } }",
            json!({ "pullRequestId": id, "event": "APPROVE" }),
        ),
        Act::RequestChanges => (
            "mutation($i: AddPullRequestReviewInput!) { addPullRequestReview(input: $i) { clientMutationId } }",
            json!({ "pullRequestId": id, "event": "REQUEST_CHANGES", "body": body }),
        ),
        Act::Merge => (
            "mutation($i: MergePullRequestInput!) { mergePullRequest(input: $i) { clientMutationId } }",
            json!({ "pullRequestId": id, "mergeMethod": "SQUASH" }),
        ),
        Act::Rebase => (
            "mutation($i: UpdatePullRequestBranchInput!) { updatePullRequestBranch(input: $i) { clientMutationId } }",
            json!({ "pullRequestId": id, "updateMethod": "REBASE" }),
        ),
        Act::Close => (
            "mutation($i: ClosePullRequestInput!) { closePullRequest(input: $i) { clientMutationId } }",
            json!({ "pullRequestId": id }),
        ),
        Act::Publish => (
            "mutation($i: MarkPullRequestReadyForReviewInput!) { markPullRequestReadyForReview(input: $i) { clientMutationId } }",
            json!({ "pullRequestId": id }),
        ),
        Act::Draft => (
            "mutation($i: ConvertPullRequestToDraftInput!) { convertPullRequestToDraft(input: $i) { clientMutationId } }",
            json!({ "pullRequestId": id }),
        ),
    };
    post(query, json!({ "i": input })).map(|_| ())
}

/// What a landed `act` changes in the boards' copies of a PR, ahead of the next fetch.
pub fn apply(act: Act, prs: &mut [Vec<Pr>; 2], url: &str, now: i64) {
    for p in prs.iter_mut().flatten().filter(|p| p.url == url) {
        match act {
            Act::Merge => (p.state, p.closed) = ("MERGED".into(), now),
            Act::Close => (p.state, p.closed) = ("CLOSED".into(), now),
            Act::Publish => p.draft = false,
            Act::Draft => p.draft = true,
            Act::Approve => (p.my_review, p.requested) = (Some("APPROVED".into()), false),
            Act::RequestChanges => (p.my_review, p.requested) = (Some("CHANGES_REQUESTED".into()), false),
            Act::Uninvolve => p.requested = false,
            Act::Dismiss => p.my_review = None,
            Act::Rebase => {}
        }
    }
    if act == Act::Uninvolve {
        prs[REVIEWS].retain(|p| p.url != url || p.my_review.is_some());
    }
}

/// A CI check that hasn't passed: failed, or still running.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub name: String,
    pub running: bool,
    pub url: String,
}

/// What the detail card shows beyond the board's cache.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Info {
    pub body: String,
    pub merge_state: String,
    /// Failed and running checks only.
    pub checks: Vec<Check>,
    pub approvers: Vec<String>,
}

const INFO_QUERY: &str = "query($id: ID!) { node(id: $id) { ... on PullRequest { body mergeStateStatus
    latestReviews(first: 20) { nodes { state author { login } } }
    commits(last: 1) { nodes { commit { statusCheckRollup { contexts(first: 100) { nodes { __typename
        ... on CheckRun { name status conclusion detailsUrl }
        ... on StatusContext { context state targetUrl } } } } } } } } } }";

/// Fetched when a detail or menu opens: mergeability moves with the base branch, not with the
/// PR's `updatedAt`, so the board's cache can't hold it.
pub fn detail(id: &str) -> Result<Info, String> {
    Ok(info_from(&post(INFO_QUERY, json!({ "id": id }))?["node"]))
}

pub fn info_from(n: &Value) -> Info {
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let checks = n["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["contexts"]["nodes"].as_array().into_iter().flatten()
        .filter_map(|c| match c["__typename"].as_str()? {
            "CheckRun" => {
                let running = c["status"] != "COMPLETED";
                let failed = matches!(c["conclusion"].as_str(), Some("FAILURE" | "TIMED_OUT" | "CANCELLED" | "STARTUP_FAILURE" | "ACTION_REQUIRED"));
                (running || failed).then(|| Check { name: s(&c["name"]), running, url: s(&c["detailsUrl"]) })
            }
            _ => {
                let running = matches!(c["state"].as_str(), Some("PENDING" | "EXPECTED"));
                let failed = matches!(c["state"].as_str(), Some("FAILURE" | "ERROR"));
                (running || failed).then(|| Check { name: s(&c["context"]), running, url: s(&c["targetUrl"]) })
            }
        })
        .collect();
    let approvers = n["latestReviews"]["nodes"].as_array().into_iter().flatten()
        .filter(|r| r["state"] == "APPROVED")
        .filter_map(|r| r["author"]["login"].as_str().map(String::from))
        .collect();
    Info { body: s(&n["body"]), merge_state: s(&n["mergeStateStatus"]), checks, approvers }
}

/// Why Merge is off for a `mergeStateStatus` (`None` while it's still loading), or `None` when it's on.
pub fn merge_block(state: Option<&str>) -> Option<&'static str> {
    match state {
        None => Some("Checking mergeability"),
        Some("CLEAN" | "HAS_HOOKS" | "UNSTABLE") => None,
        Some("BEHIND") => Some("Behind the base branch, rebase first"),
        Some("BLOCKED") => Some("Blocked by checks or reviews"),
        Some("DIRTY") => Some("Has merge conflicts"),
        Some("DRAFT") => Some("Still a draft"),
        Some(_) => Some("GitHub hasn't computed mergeability yet"),
    }
}

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(ureq::Agent::new_with_defaults);

fn post(query: &str, variables: Value) -> Result<Value, String> {
    let body = json!({ "query": query, "variables": variables }).to_string();
    let send = |token: &str| {
        AGENT.post("https://api.github.com/graphql")
            .header("Authorization", &format!("Bearer {token}"))
            .header("User-Agent", "prsito")
            .send(&body)
    };
    let resp = match send(&token(false)?) {
        Err(ureq::Error::StatusCode(401)) => send(&token(true)?),
        Err(ureq::Error::StatusCode(502 | 504)) => send(&token(false)?),
        r => r,
    };
    let text = resp.map_err(|e| match e {
        ureq::Error::StatusCode(403 | 429) => "GitHub is rate limiting, will retry on the next refresh".to_string(),
        e => format!("GitHub: {e}"),
    })?.body_mut().read_to_string().map_err(|e| format!("GitHub: {e}"))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("GitHub: {e}"))?;
    if let Some(err) = v["errors"][0]["message"].as_str() {
        return Err(format!("GitHub: {err}"));
    }
    Ok(v["data"].clone())
}

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or_default()
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-11-06T12:34:56Z` to seconds since the epoch.
pub fn parse_time(s: &str) -> Option<i64> {
    let n = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let days = days_from_civil(n(0..4)?, n(5..7)?, n(8..10)?);
    Some(days * 86_400 + n(11..13)? * 3600 + n(14..16)? * 60 + n(17..19)?)
}

/// Seconds since the epoch to `YYYY-MM-DD` (UTC).
pub fn date(secs: i64) -> String {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn age(now: i64, then: i64) -> String {
    let s = (now - then).max(0);
    match s {
        0..60 => "now".into(),
        60..3600 => format!("{}m", s / 60),
        3600..86_400 => format!("{}h", s / 3600),
        86_400..2_592_000 => format!("{}d", s / 86_400),
        _ => format!("{}mo", s / 2_592_000),
    }
}
