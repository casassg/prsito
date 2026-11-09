//! `prsito <command>`: the board from a terminal, for scripts and agents. It shares the app's
//! database and GitHub calls; a running app picks up its writes within a second.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::github::{self, Act, MINE, Pr, Query, REVIEWS};
use crate::store::{self, Lanes, Target};

const USAGE: &str = "usage: prsito <command> [--json]
  lanes [--board mine|reviews]                 lanes, with how many cached PRs each holds
  ls [--lane NAME|ID] [--board B] [--fetch]    cached PRs; --fetch asks GitHub first
  mv PR LANE [--board B]                       moves a PR to a lane, made if no lane has that name
  approve PR | request-changes PR -m TEXT | merge PR | rebase PR | close PR
  publish PR | draft PR | uninvolve PR | dismiss PR
PR is a URL or owner/repo#123. merge squashes. Actions ask first; -y or --yes skips that,
and without a terminal (scripts, agents) they need it.";

pub fn main(args: &[String]) -> i32 {
    let Some(path) = store::path() else {
        eprintln!("prsito: HOME isn't set");
        return 1;
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&path, &args, ask) {
        Ok(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
            0
        }
        Err(e) => {
            eprintln!("prsito: {e}");
            1
        }
    }
}

/// Asks on the terminal whether to go ahead, Enter meaning yes.
fn ask(question: &str) -> Result<(), String> {
    use std::io::{BufRead, IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        return Err(format!("{question} needs --yes when not run from a terminal"));
    }
    eprint!("{question} [Y/n] ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).map_err(|e| e.to_string())?;
    match line.trim().to_lowercase().as_str() {
        "" | "y" | "yes" => Ok(()),
        _ => Err("cancelled".into()),
    }
}

/// Runs a command; `confirm` is asked before an action reaches GitHub, unless `--yes`.
pub fn run(path: &Path, args: &[&str], confirm: impl Fn(&str) -> Result<(), String>) -> Result<String, String> {
    let (mut json, mut fetch, mut yes, mut board, mut lane, mut message) = (false, false, false, None, None, None);
    let mut pos = Vec::new();
    let mut it = args.iter().copied();
    while let Some(a) = it.next() {
        match a {
            "--json" => json = true,
            "--fetch" => fetch = true,
            "-y" | "--yes" => yes = true,
            "--board" => board = Some(match it.next() {
                Some("mine") => MINE,
                Some("reviews") => REVIEWS,
                _ => return Err("--board takes mine or reviews".into()),
            }),
            "--lane" => lane = Some(it.next().ok_or("--lane takes a lane name or id")?),
            "-m" | "--message" => message = Some(it.next().ok_or("-m takes a comment")?),
            "-h" | "--help" | "help" => return Ok(USAGE.into()),
            f if f.starts_with('-') => return Err(format!("unknown flag {f}\n{USAGE}")),
            _ => pos.push(a),
        }
    }
    let Some((&cmd, rest)) = pos.split_first() else { return Ok(USAGE.into()) };
    let boards: Vec<usize> = board.map_or(vec![MINE, REVIEWS], |b| vec![b]);
    let mut saved = store::load(path);
    let mut prs = store::load_cache(path).unwrap_or_default();
    let name = |b: usize| ["mine", "reviews"][b];
    match (cmd, rest) {
        ("lanes", []) => {
            let mut rows = Vec::new();
            for &b in &boards {
                let lanes = &saved.boards[b];
                for l in &lanes.lanes {
                    let n = prs[b].iter().filter(|p| lanes.lane_of(&p.url) == l.id).count();
                    rows.push(json!({ "board": name(b), "id": l.id, "name": l.name, "prs": n }));
                }
            }
            Ok(if json {
                Value::Array(rows).to_string()
            } else {
                rows.iter().map(|r| format!("{}\t{}\t{}\t{}", r["board"].as_str().unwrap_or_default(), r["id"], r["name"].as_str().unwrap_or_default(), r["prs"])).collect::<Vec<_>>().join("\n")
            })
        }
        ("ls", []) => {
            if fetch {
                let st = &saved.settings;
                let q = Query { days: st.days, include_closed: st.include_closed, filter: st.filter.clone() };
                for &b in &boards {
                    prs[b] = github::fetch(b, &q, &prs[b], false, |_| {})?;
                    let urls: Vec<&str> = prs[b].iter().map(|p| p.url.as_str()).collect();
                    saved.boards[b].prune(&urls, github::now());
                }
                store::save(path, &saved)?;
                store::save_cache(path, &prs)?;
            }
            if let Some(q) = lane && !boards.iter().any(|&b| find_lane(&saved.boards[b], q).is_some()) {
                return Err(format!("no lane {q}"));
            }
            let mut rows = Vec::new();
            for &b in &boards {
                let lanes = &saved.boards[b];
                let only = lane.map(|q| find_lane(lanes, q));
                if only == Some(None) {
                    continue;
                }
                let only = only.flatten();
                let mut list: Vec<&Pr> = prs[b].iter().filter(|p| only.is_none_or(|id| lanes.lane_of(&p.url) == id)).collect();
                let order = |p: &Pr| (lanes.lanes.iter().position(|l| l.id == lanes.lane_of(&p.url)), github::column(b, p));
                list.sort_by_key(|p| order(p));
                for p in list {
                    let id = lanes.lane_of(&p.url);
                    let mut v = serde_json::to_value(p).map_err(|e| e.to_string())?;
                    v["board"] = json!(name(b));
                    v["lane"] = json!(lanes.lanes.iter().find(|l| l.id == id).map_or("Inbox", |l| l.name.as_str()));
                    v["lane_id"] = json!(id);
                    v["column"] = json!(github::COLUMNS[b][github::column(b, p)]);
                    rows.push(v);
                }
            }
            Ok(if json {
                Value::Array(rows).to_string()
            } else {
                let s = |v: &Value, k: &str| v[k].as_str().unwrap_or_default().to_string();
                rows.iter().map(|r| format!("{}\t{}\t{}\t{}#{}\t{}\t{}", s(r, "board"), s(r, "lane"), s(r, "column"), s(r, "repo"), r["number"], s(r, "title"), s(r, "url"))).collect::<Vec<_>>().join("\n")
            })
        }
        ("mv", [pr, to]) => {
            let url = parse(pr)?;
            let b = board.or_else(|| prs.iter().position(|list| list.iter().any(|p| p.url == url)))
                .ok_or(format!("{pr} isn't in the cache: run `prsito ls --fetch`, or pass --board"))?;
            let lanes = &mut saved.boards[b];
            let id = match find_lane(lanes, to) {
                Some(id) => lanes.move_card(&url, Target::Lane(id)),
                None => {
                    let id = lanes.move_card(&url, Target::New(lanes.lanes.len()));
                    if let Some(l) = lanes.lane_mut(id) {
                        l.name = to.to_string();
                    }
                    id
                }
            };
            let lane = lanes.lanes.iter().find(|l| l.id == id).map_or("Inbox".into(), |l| l.name.clone());
            store::save(path, &saved)?;
            Ok(if json { json!({ "url": url, "board": name(b), "lane": lane, "lane_id": id }).to_string() } else { format!("moved {pr} to {lane}") })
        }
        (verb, [pr]) => {
            let act = match verb {
                "approve" => Act::Approve,
                "request-changes" => Act::RequestChanges,
                "merge" => Act::Merge,
                "rebase" => Act::Rebase,
                "close" => Act::Close,
                "publish" => Act::Publish,
                "draft" => Act::Draft,
                "uninvolve" => Act::Uninvolve,
                "dismiss" => Act::Dismiss,
                _ => return Err(format!("unknown command {verb}\n{USAGE}")),
            };
            let url = parse(pr)?;
            let p = prs.iter().flatten().find(|p| p.url == url).cloned()
                .ok_or(format!("{pr} isn't in the cache: run `prsito ls --fetch`"))?;
            if p.id.is_empty() {
                return Err(format!("{pr} hasn't loaded from GitHub yet: run `prsito ls --fetch`"));
            }
            let body = message.unwrap_or_default().trim();
            if act == Act::RequestChanges && body.is_empty() {
                return Err("request-changes needs a comment: -m TEXT".into());
            }
            if !yes {
                confirm(&format!("{verb} {}#{} \"{}\"?", p.repo, p.number, p.title))?;
            }
            github::act(act, &p, body)?;
            github::apply(act, &mut prs, &url, github::now());
            store::save_cache(path, &prs)?;
            Ok(if json { json!({ "url": url, "act": verb, "ok": true }).to_string() } else { format!("{verb}: {pr} done") })
        }
        _ => Err(format!("unknown command or arguments: {}\n{USAGE}", pos.join(" "))),
    }
}

/// A lane by name (any case), else by id.
fn find_lane(lanes: &Lanes, q: &str) -> Option<u64> {
    lanes.lanes.iter().find(|l| l.name.eq_ignore_ascii_case(q)).map(|l| l.id)
        .or_else(|| q.parse().ok().filter(|id| lanes.lanes.iter().any(|l| l.id == *id)))
}

/// A PR's URL from its URL or `owner/repo#123`.
pub fn parse(pr: &str) -> Result<String, String> {
    if let Some(rest) = pr.strip_prefix("https://github.com/") {
        let rest = rest.trim_end_matches('/');
        if let [_, _, "pull", n] = rest.split('/').collect::<Vec<_>>()[..] && n.parse::<u64>().is_ok() {
            return Ok(format!("https://github.com/{rest}"));
        }
    }
    match pr.split_once('#') {
        Some((repo, n)) if repo.split('/').count() == 2 && !repo.starts_with('/') && !repo.ends_with('/') && n.parse::<u64>().is_ok() => {
            Ok(format!("https://github.com/{repo}/pull/{n}"))
        }
        _ => Err(format!("{pr} isn't a PR: use its URL or owner/repo#123")),
    }
}

fn link(home: &Path) -> PathBuf {
    home.join(".local/bin/prsito")
}

/// HOME and this binary, resolved through links.
pub fn here() -> Result<(PathBuf, PathBuf), String> {
    let home = std::env::var_os("HOME").ok_or("HOME isn't set")?;
    let exe = std::env::current_exe().and_then(|e| e.canonicalize()).map_err(|e| format!("can't find prsito's binary: {e}"))?;
    Ok((PathBuf::from(home), exe))
}

/// Links `~/.local/bin/prsito` to `exe`, so the command follows the app through updates.
pub fn install(home: &Path, exe: &Path) -> Result<(), String> {
    let link = link(home);
    if let Ok(m) = std::fs::symlink_metadata(&link) {
        if !m.file_type().is_symlink() {
            return Err(format!("{} exists and isn't a link, remove it first", link.display()));
        }
        std::fs::remove_file(&link).map_err(|e| e.to_string())?;
    }
    if let Some(dir) = link.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::os::unix::fs::symlink(exe, &link).map_err(|e| format!("can't link {}: {e}", link.display()))
}

pub fn installed(home: &Path, exe: &Path) -> bool {
    std::fs::read_link(link(home)).is_ok_and(|t| t == exe)
}
