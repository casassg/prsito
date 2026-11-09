//! prsito: my GitHub pull requests as a small kanban board, with swimlanes I arrange by drag.
//! `cargo run --release` opens it; `-- --snap board.png --scene fixture` renders sample data headless;
//! `-- help` lists the commands scripts and agents use (`cli.rs`).

mod cli;
mod drag;
mod github;
mod statusbar;
mod store;
mod theme;
mod update;
mod view;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use github::{Act, Pr, Query};
use std::sync::mpsc::{Receiver, Sender, channel};

use rune::{AppSpec, Key, KeyEvent, TextEdit, Tx, Waker};
use drag::{DragState, Dragging};
use store::{Saved, Target};

/// A page of a fetch (`done: false`, PRs so far) or its end.
pub struct Fetched {
    pub board: usize,
    pub generation: u64,
    pub done: bool,
    pub result: Result<Vec<Pr>, String>,
}

/// What a background action or detail lookup found.
pub enum Note {
    Detail { url: String, result: Result<github::Info, String> },
    Acted { url: String, act: Act, result: Result<(), String> },
    Update(Option<update::Release>),
    UpdateFailed(String),
}

const UPDATE_EVERY: f64 = 6.0 * 3600.0;

/// The open detail card; `info` is `None` while it loads.
#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub url: String,
    pub info: Option<github::Info>,
}

/// The right-click menu of a card, at window point (`x`, `y`).
#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub url: String,
    pub x: f32,
    pub y: f32,
    pub merge_state: Option<String>,
}

const CONFIRM: Duration = Duration::from_secs(3);

pub struct App {
    pub tab: usize,
    pub prs: [Vec<Pr>; 2],
    pub loaded: [bool; 2],
    pub saved: Saved,
    pub path: Option<PathBuf>,
    pub live: bool,
    /// Fetches in flight per board; their pages and results arrive on `inbox`.
    pub pending: [bool; 2],
    pub generation: u64,
    pub inbox: Receiver<Fetched>,
    pub outbox: Sender<Fetched>,
    pub waker: Option<Waker>,
    pub error: [Option<String>; 2],
    pub next_fetch: f64,
    /// A settings edit changed what to fetch: refetch once typing pauses.
    pub refetch_soon: bool,
    /// The next fetch refetches every PR (⌘R); interval fetches only take what changed.
    pub full: bool,
    pub now: i64,
    pub drag: Option<DragState>,
    /// Window rects of cards (`card:URL`), lanes (`lane:ID`) and the new-lane zone (`new`).
    pub rects: HashMap<String, [f32; 4]>,
    pub renaming: Option<u64>,
    pub rename: TextEdit,
    pub settings_open: bool,
    pub days: TextEdit,
    pub interval: TextEdit,
    pub filter: TextEdit,
    /// The toolbar search: every word must appear in a card's repo, number, title or author.
    pub search: TextEdit,
    pub detail: Option<Detail>,
    pub menu: Option<Menu>,
    /// The one mutation in flight: its buttons wait.
    pub acting: Option<(String, Act)>,
    /// A PR whose red X was clicked once; a second click within `CONFIRM` closes it.
    pub armed: Option<(String, Instant)>,
    /// The PR of a changes request waiting for its comment (`note_text`).
    pub note: Option<String>,
    pub note_text: TextEdit,
    /// Lanes (per board) whose done column is listed in full rather than stacked.
    pub open_stacks: HashSet<(usize, u64)>,
    /// The stacked done column the pointer rests on, fanned open a little.
    pub fanned: Option<(usize, u64)>,
    pub notes: Receiver<Note>,
    pub notes_out: Sender<Note>,
    /// Notices the CLI's writes, so the board follows them.
    pub watch: Option<store::Watch>,
    /// `~/.local/bin/prsito` links to this app.
    pub cli: bool,
    /// The window is coming back from the menu bar and may still carry a stale full screen.
    pub unfull: bool,
    /// A newer release to offer in the banner; dismissing it waits for the next check.
    pub update: Option<update::Release>,
    pub updating: bool,
    pub next_check: f64,
}

impl App {
    pub fn new(saved: Saved, path: Option<PathBuf>, live: bool) -> App {
        let st = saved.settings.clone();
        let (outbox, inbox) = channel();
        let (notes_out, notes) = channel();
        App {
            tab: github::MINE,
            prs: [Vec::new(), Vec::new()],
            loaded: [false, false],
            saved,
            path,
            live,
            pending: [false, false],
            generation: 0,
            inbox,
            outbox,
            waker: None,
            error: [None, None],
            next_fetch: 0.0,
            refetch_soon: false,
            full: false,
            now: github::now(),
            drag: None,
            rects: HashMap::new(),
            renaming: None,
            rename: TextEdit::new(""),
            settings_open: false,
            days: TextEdit::new(st.days.to_string()),
            interval: TextEdit::new(st.interval.to_string()),
            filter: TextEdit::new(st.filter),
            search: TextEdit::new(""),
            detail: None,
            menu: None,
            acting: None,
            armed: None,
            note: None,
            note_text: TextEdit::new(""),
            open_stacks: HashSet::new(),
            fanned: None,
            notes,
            notes_out,
            watch: None,
            cli: false,
            unfull: false,
            update: None,
            updating: false,
            next_check: 0.0,
        }
    }

    /// Downloads and installs the offered release, then relaunches. Headless it only marks the
    /// install as running.
    pub fn install_update(&mut self) {
        let Some(r) = self.update.clone().filter(|_| !self.updating) else { return };
        self.updating = true;
        if !self.live {
            return;
        }
        let (out, wake) = (self.notes_out.clone(), self.waker.clone());
        std::thread::spawn(move || {
            if out.send(Note::UpdateFailed(update::install(&r))).is_ok() && let Some(w) = &wake {
                w();
            }
        });
    }

    fn check_update(&self) {
        let Some(current) = update::VERSION else { return };
        let (out, wake) = (self.notes_out.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let Ok(found) = update::check(current) else { return };
            if out.send(Note::Update(found)).is_ok() && let Some(w) = &wake {
                w();
            }
        });
    }

    fn pr(&self, url: &str) -> Option<&Pr> {
        self.prs.iter().flatten().find(|p| p.url == url)
    }

    fn lookup(&self, url: &str) {
        let Some(id) = self.pr(url).map(|p| p.id.clone()).filter(|id| self.live && !id.is_empty()) else { return };
        let (out, wake, url) = (self.notes_out.clone(), self.waker.clone(), url.to_string());
        std::thread::spawn(move || {
            let result = github::detail(&id);
            if out.send(Note::Detail { url, result }).is_ok() && let Some(w) = &wake {
                w();
            }
        });
    }

    pub fn open_detail(&mut self, url: String) {
        self.armed = None;
        self.menu = None;
        if self.pr(&url).is_none() {
            return;
        }
        self.lookup(&url);
        self.detail = Some(Detail { url, info: None });
    }

    pub fn open_menu(&mut self, url: String, x: f32, y: f32) {
        self.armed = None;
        if self.pr(&url).is_none() {
            return;
        }
        self.lookup(&url);
        self.menu = Some(Menu { url, x, y, merge_state: None });
    }

    /// Runs a mutation on a PR in the background; one at a time. Headless there's no GitHub,
    /// so it lands at once.
    pub fn run(&mut self, act: Act, url: &str) {
        self.run_with(act, url, String::new());
    }

    fn run_with(&mut self, act: Act, url: &str, body: String) {
        if self.acting.is_some() {
            return;
        }
        let Some(pr) = self.pr(url).cloned() else { return };
        self.armed = None;
        self.menu = None;
        if !self.live {
            return self.finish(url, act, Ok(()));
        }
        if pr.id.is_empty() {
            self.error[self.board()] = Some("this PR hasn't loaded from GitHub yet, try again after the refresh".into());
            return;
        }
        self.acting = Some((url.to_string(), act));
        let (out, wake, url) = (self.notes_out.clone(), self.waker.clone(), url.to_string());
        std::thread::spawn(move || {
            let result = github::act(act, &pr, &body);
            if out.send(Note::Acted { url, act, result }).is_ok() && let Some(w) = &wake {
                w();
            }
        });
    }

    /// What a card's X does: closes my PRs, and on Reviews drops my review request (nothing
    /// once there's none left to drop).
    pub fn x_act(&self, pr: &Pr) -> Option<Act> {
        match self.board() {
            github::REVIEWS => pr.requested.then_some(Act::Uninvolve),
            _ => Some(Act::Close),
        }
    }

    /// The first click on a red X arms it; a second on the same PR within `CONFIRM` runs it.
    pub fn arm_close(&mut self, url: &str) {
        let Some(act) = self.pr(url).and_then(|p| self.x_act(p)) else { return };
        match &self.armed {
            Some((u, at)) if u == url && at.elapsed() < CONFIRM => self.run(act, url),
            _ => self.armed = Some((url.to_string(), Instant::now())),
        }
    }

    /// Asks for the comment a changes request needs.
    pub fn ask_changes(&mut self, url: &str) {
        self.note_text = TextEdit::new("");
        self.note_text.focused = true;
        self.note = Some(url.to_string());
    }

    pub fn send_changes(&mut self) {
        let body = self.note_text.text.trim().to_string();
        if body.is_empty() {
            return;
        }
        let Some(url) = self.note.take() else { return };
        self.run_with(Act::RequestChanges, &url, body);
    }

    fn finish(&mut self, url: &str, act: Act, result: Result<(), String>) {
        self.acting = None;
        if let Err(e) = result {
            self.error[self.board()] = Some(e);
            return;
        }
        let now = github::now();
        github::apply(act, &mut self.prs, url, now);
        match act {
            Act::Rebase | Act::Publish | Act::Draft | Act::Approve | Act::RequestChanges | Act::Dismiss if self.detail.as_ref().is_some_and(|d| d.url == url) => self.open_detail(url.to_string()),
            Act::Rebase | Act::Publish | Act::Draft | Act::Approve | Act::RequestChanges | Act::Dismiss => {}
            Act::Merge | Act::Close | Act::Uninvolve => self.detail = None,
        }
        self.now = now;
        self.next_fetch = 0.0;
    }

    pub fn lanes(&self) -> &store::Lanes {
        &self.saved.boards[self.board()]
    }

    pub fn lanes_mut(&mut self) -> &mut store::Lanes {
        &mut self.saved.boards[self.board()]
    }

    pub fn persist(&mut self) {
        let Some(path) = &self.path else { return };
        if let Err(e) = store::save(path, &self.saved) {
            eprintln!("prsito: can't save {}: {e}", path.display());
        }
        if let Some(w) = &mut self.watch {
            w.changed();
        }
    }

    /// Takes in lanes and PRs another process (the CLI) wrote. Waits while a drag is held.
    pub fn reload(&mut self) -> bool {
        if self.drag.is_some() || !self.watch.as_mut().is_some_and(|w| w.changed()) {
            return false;
        }
        let Some(path) = &self.path else { return false };
        self.saved.boards = store::load(path).boards;
        if let Some(prs) = store::load_cache(path) {
            self.prs = prs;
        }
        true
    }

    pub fn install_cli(&mut self) {
        let result = if self.live { cli::here().and_then(|(home, exe)| cli::install(&home, &exe)) } else { Ok(()) };
        match result {
            Ok(()) => self.cli = true,
            Err(e) => self.error[self.board()] = Some(e),
        }
    }

    pub fn refresh(&mut self) {
        self.full = true;
        self.next_fetch = 0.0;
    }

    fn request(&mut self) {
        let st = &self.saved.settings;
        let q = Query { days: st.days, include_closed: st.include_closed, filter: st.filter.clone() };
        self.generation += 1;
        let full = std::mem::take(&mut self.full);
        for board in [github::MINE, github::REVIEWS] {
            let (q, out, wake, generation) = (q.clone(), self.outbox.clone(), self.waker.clone(), self.generation);
            let prev = self.prs[board].clone();
            let send = move |done: bool, result: Result<Vec<Pr>, String>| {
                if out.send(Fetched { board, generation, done, result }).is_ok() && let Some(w) = &wake {
                    w();
                }
            };
            self.pending[board] = true;
            std::thread::spawn(move || {
                let result = github::fetch(board, &q, &prev, full, |prs| send(false, Ok(prs)));
                send(true, result);
            });
        }
    }

    /// A fetch that started before a merge or close, or a search GitHub hasn't reindexed yet, still
    /// says OPEN with the old `updatedAt`; the local copy wins until GitHub reports something newer.
    pub fn land(&mut self, board: usize, result: Result<Vec<Pr>, String>) {
        match result {
            Ok(prs) => {
                let prs: Vec<Pr> = prs.into_iter().map(|p| match self.prs[board].iter().find(|o| o.url == p.url) {
                    Some(o) if o.state != "OPEN" && p.state == "OPEN" && p.updated <= o.updated => o.clone(),
                    _ => p,
                }).collect();
                let urls: Vec<&str> = prs.iter().map(|p| p.url.as_str()).collect();
                let lanes = &mut self.saved.boards[board];
                let before = lanes.clone();
                lanes.prune(&urls, github::now());
                let changed = *lanes != before;
                self.prs[board] = prs;
                self.loaded[board] = true;
                self.error[board] = None;
                self.now = github::now();
                if changed {
                    self.persist();
                }
                if let Some(path) = &self.path
                    && let Err(e) = store::save_cache(path, &self.prs)
                {
                    eprintln!("prsito: can't save the PR cache: {e}");
                }
                if let Some(w) = &mut self.watch {
                    w.changed();
                }
            }
            Err(e) => self.error[board] = Some(e),
        }
    }

    pub fn start_rename(&mut self, id: u64) {
        let Some(lane) = self.lanes().lanes.iter().find(|l| l.id == id) else { return };
        let mut edit = TextEdit::new(lane.name.clone());
        edit.anchor = 0;
        edit.focus = edit.text.len();
        edit.focused = true;
        self.rename = edit;
        self.renaming = Some(id);
    }

    pub fn finish_rename(&mut self) {
        let Some(id) = self.renaming.take() else { return };
        let name = self.rename.text.trim().to_string();
        if name.is_empty() {
            return;
        }
        if let Some(lane) = self.lanes_mut().lane_mut(id) {
            lane.name = name;
        }
        self.persist();
    }

    /// Reads the settings fields back into the saved settings.
    /// Saves the settings fields as they're edited. The board follows at once for what it can
    /// show from PRs it has (the window, closed PRs); a change GitHub has to answer refetches
    /// once typing pauses, dropping any fetch still running for the old settings.
    pub fn apply_settings(&mut self) {
        let st = &mut self.saved.settings;
        let before = st.clone();
        st.days = self.days.text.trim().parse().unwrap_or(st.days).clamp(1, 365);
        st.interval = self.interval.text.trim().parse().unwrap_or(st.interval).clamp(10, 3600);
        st.filter = self.filter.text.trim().to_string();
        if *st == before {
            return;
        }
        let refetch = (st.days, st.include_closed, &st.filter) != (before.days, before.include_closed, &before.filter);
        self.persist();
        if refetch {
            self.generation += 1;
            self.pending = [false, false];
            self.refetch_soon = true;
        }
    }

    pub fn toggle_closed(&mut self) {
        self.saved.settings.include_closed = !self.saved.settings.include_closed;
        self.persist();
        self.generation += 1;
        self.pending = [false, false];
        self.refresh();
    }

    pub fn toggle_menu_bar(&mut self) {
        self.saved.settings.menu_bar = !self.saved.settings.menu_bar;
        self.persist();
        self.sync_menu_bar();
    }

    pub fn sync_menu_bar(&self) {
        if self.live {
            statusbar::sync(self.saved.settings.menu_bar, &self.prs[github::MINE]);
        }
    }

    pub fn close_settings(&mut self) {
        self.settings_open = false;
        self.apply_settings();
        let st = &self.saved.settings;
        self.days = TextEdit::new(st.days.to_string());
        self.interval = TextEdit::new(st.interval.to_string());
    }

    /// The board (data source) behind the current view.
    pub fn board(&self) -> usize {
        view::VIEWS[self.tab].board
    }

    /// A view's PRs under the current settings: its board narrowed by the view's filter, closed
    /// ones only when asked for, merged or closed ones only inside the window, and only those
    /// matching the search.
    pub fn visible(&self, view: usize) -> Vec<&Pr> {
        let st = &self.saved.settings;
        let since = self.now - st.days as i64 * 86_400;
        let query = self.search.text.to_lowercase();
        let spec = &view::VIEWS[view];
        self.prs[spec.board].iter()
            .filter(|p| spec.filter.is_none_or(|f| f(p)))
            .filter(|p| p.state == "OPEN" || (p.closed == 0 || p.closed >= since) && (p.state != "CLOSED" || st.include_closed))
            .filter(|p| {
                let hay = format!("{} #{} {} {}", p.repo, p.number, p.title, p.author).to_lowercase();
                query.split_whitespace().all(|w| hay.contains(w))
            })
            .collect()
    }
}

fn init() -> App {
    let live = !rune::is_headless();
    let path = live.then(store::path).flatten();
    let saved = path.as_deref().map(store::load).unwrap_or_default();
    let cached = path.as_deref().and_then(store::load_cache);
    let mut app = App::new(saved, path, live);
    if let Some(prs) = cached {
        app.prs = prs;
        app.loaded = [true, true];
    }
    app.watch = app.path.as_deref().and_then(store::Watch::open);
    app.cli = live && cli::here().is_ok_and(|(home, exe)| cli::installed(&home, &exe));
    app
}

fn setup(s: &mut App, waker: Waker) {
    s.waker = Some(waker);
}

fn poll(s: &mut App, t: f64, tx: &mut Tx) -> bool {
    let mut changed = s.reload();
    if s.watch.is_some() {
        tx.wake_after(Duration::from_secs(1));
    }
    while let Ok(m) = s.inbox.try_recv() {
        if m.generation != s.generation {
            continue;
        }
        if !s.loaded[m.board] {
            tx.instant();
        }
        match (m.done, m.result) {
            (true, r) => {
                s.pending[m.board] = false;
                s.land(m.board, r);
            }
            // Pages fill the board on the first load only; a refresh swaps in whole results.
            (false, Ok(prs)) if !s.loaded[m.board] => s.prs[m.board] = prs,
            (false, _) => continue,
        }
        changed = true;
    }
    while let Ok(n) = s.notes.try_recv() {
        match n {
            Note::Detail { url, result } => {
                let info = result.unwrap_or_else(|e| {
                    s.error[s.board()] = Some(e);
                    github::Info { merge_state: "UNKNOWN".into(), ..Default::default() }
                });
                if let Some(m) = s.menu.as_mut().filter(|m| m.url == url) {
                    m.merge_state = Some(info.merge_state.clone());
                }
                if let Some(d) = s.detail.as_mut().filter(|d| d.url == url) {
                    d.info = Some(info);
                }
            }
            Note::Acted { url, act, result } => s.finish(&url, act, result),
            Note::Update(found) => s.update = found,
            Note::UpdateFailed(e) => {
                s.updating = false;
                s.error[s.board()] = Some(e);
            }
        }
        changed = true;
    }
    if s.live && update::VERSION.is_some() {
        if t >= s.next_check {
            s.check_update();
            s.next_check = t + UPDATE_EVERY;
        }
        tx.wake_after(Duration::from_secs_f64((s.next_check - t).max(1.0)));
    }
    if let Some((_, at)) = &s.armed {
        match CONFIRM.checked_sub(at.elapsed()) {
            Some(left) => tx.wake_after(left),
            None => {
                s.armed = None;
                changed = true;
            }
        }
    }
    // A fetch can take longer than the interval (search is slow for a hundred PRs), and a new
    // request would drop the running one's answer, so the next one waits for this one to land.
    if s.refetch_soon {
        s.refetch_soon = false;
        s.next_fetch = t + 0.8;
    }
    let pending = s.pending.iter().any(|p| *p);
    if s.live && !pending && t >= s.next_fetch {
        s.request();
        s.next_fetch = f64::INFINITY;
        changed = true;
    } else if s.live && !pending && s.next_fetch == f64::INFINITY {
        s.next_fetch = t + s.saved.settings.interval as f64;
    }
    if s.live && s.next_fetch.is_finite() {
        tx.wake_after(Duration::from_secs_f64((s.next_fetch - t).max(0.5)));
    }
    if changed {
        s.sync_menu_bar();
    }
    if s.unfull {
        s.unfull = !statusbar::unfull();
        if s.unfull {
            tx.wake_after(Duration::from_millis(100));
        }
    }
    changed
}

fn on_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.is_command('q') {
        tx.quit();
        return true;
    }
    if ev.is_command('r') {
        s.refresh();
        return true;
    }
    if ev.is_command('f') || ev.mods.ctrl && tx.key_base() == Some('f') {
        s.search.focused = true;
        s.search.anchor = 0;
        s.search.focus = s.search.text.len();
        return true;
    }
    if ev.key == Key::Escape && s.note.is_some() {
        s.note = None;
        return true;
    }
    if ev.key == Key::Escape && s.menu.is_some() {
        s.menu = None;
        return true;
    }
    if ev.key == Key::Escape && s.detail.is_some() {
        s.detail = None;
        s.armed = None;
        return true;
    }
    if ev.key == Key::Escape && s.settings_open {
        s.close_settings();
        return true;
    }
    false
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| !a.starts_with('-')) {
        std::process::exit(cli::main(&args));
    }
    rune::prewarm_gpu();
    let app = rune::App::new(AppSpec::<App> {
        scenes: &[
            ("fixture", |s| fixture(s, github::MINE)),
            ("reviews", |s| fixture(s, github::REVIEWS)),
            ("settings", |s| {
                fixture(s, github::MINE);
                s.settings_open = true;
            }),
            ("detail", |s| {
                fixture(s, github::MINE);
                let url = "https://github.com/acme/pipelines/pull/1210".to_string();
                s.detail = Some(Detail {
                    url,
                    info: Some(github::Info {
                        body: "Checkpoints now resume after a spot preemption.\n\n- Write a marker file after each save.\n- On restart, load the newest complete checkpoint.\n- Falls back to the previous one if the marker is missing.\n\n```mermaid\nflowchart LR\n  save --> marker --> resume\n```\n\nRun `pytest -k resume` to check.".into(),
                        merge_state: "BEHIND".into(),
                        checks: vec![
                            github::Check { name: "unit-tests (py3.12)".into(), running: false, url: "https://github.com/acme/pipelines/actions/runs/1".into() },
                            github::Check { name: "integration".into(), running: true, url: "https://github.com/acme/pipelines/actions/runs/2".into() },
                        ],
                        approvers: vec!["alice".into(), "bob".into()],
                    }),
                });
            }),
            ("menu", |s| {
                fixture(s, github::MINE);
                s.menu = Some(Menu { url: "https://github.com/acme/pipelines/pull/1210".into(), x: 420.0, y: 380.0, merge_state: Some("BEHIND".into()) });
            }),
            ("armed", |s| {
                fixture(s, github::MINE);
                s.armed = Some(("https://github.com/acme/fleet/pull/88".into(), Instant::now()));
            }),
            ("drop", |s| {
                fixture(s, github::REVIEWS);
                let url = "https://github.com/acme/fleet/pull/5501".to_string();
                s.drag = Some(DragState { what: Dragging::Card(url), origin: [700.0, 140.0, 240.0, 90.0], dx: 0.0, dy: 120.0, over: None, act: Some(Act::Approve) });
            }),
            ("note", |s| {
                fixture(s, github::REVIEWS);
                s.ask_changes("https://github.com/acme/fleet/pull/5501");
            }),
            ("update", |s| {
                fixture(s, github::MINE);
                s.update = Some(update::Release { version: "0.4.0".into(), dmg: String::new() });
            }),
            ("black", |s| {
                fixture(s, github::MINE);
                s.saved.settings.theme = "black".into();
            }),
            ("white", |s| {
                fixture(s, github::MINE);
                s.saved.settings.theme = "white".into();
            }),
            ("accent", |s| {
                fixture(s, github::MINE);
                s.saved.settings.theme = "accent".into();
            }),
        ],
        min_size: Some((720.0, 420.0)),
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new("prsito", (1180.0, 760.0), init, view::view)
    })
    .keep_running()
    .on_close(|s, tx| match s.saved.settings.menu_bar {
        true if s.live => statusbar::dock(false),
        _ => tx.quit(),
    })
    .on_reopen(|s, shown, _| {
        if !shown && s.live && s.saved.settings.menu_bar {
            statusbar::dock(true);
            return true;
        }
        false
    });
    let handle = app.handle();
    statusbar::connect(handle.clone());
    app.on_ready(move |_| handle.update(|s, _| s.sync_menu_bar())).run();
}

pub fn fixture(s: &mut App, tab: usize) {
    use github::{Ci, Review};
    let now = 1_793_000_000;
    let pr = |n: u64, repo: &str, title: &str, state: &str, draft: bool, ci: Ci, review: Review, h: i64| Pr {
        id: format!("PR_{n}"),
        url: format!("https://github.com/{repo}/pull/{n}"),
        repo: repo.into(),
        number: n,
        title: title.into(),
        author: "casassg".into(),
        draft,
        state: state.into(),
        updated: now - h * 3600,
        closed: if state == "OPEN" { 0 } else { now - h * 3600 },
        additions: n * 7 % 900,
        deletions: n * 3 % 300,
        ci,
        review,
        my_review: None,
        requested: false,
    };
    s.now = now;
    s.tab = tab;
    s.prs[github::MINE] = vec![
        pr(412, "casassg/wiki", "feat: weekly notes template", "OPEN", true, Ci::None, Review::None, 2),
        pr(88, "acme/fleet", "fix: karpenter nodepool taints for gpu jobs", "OPEN", false, Ci::Pending, Review::None, 1),
        pr(91, "acme/fleet", "feat: kueue cohort borrowing limits", "OPEN", false, Ci::Fail, Review::None, 5),
        pr(1203, "acme/pipelines", "chore: bump torch to 2.9", "OPEN", false, Ci::Pass, Review::Changes, 30),
        pr(1210, "acme/pipelines", "feat: resumable checkpoints on spot preemption", "OPEN", false, Ci::Pass, Review::Approved, 3),
        pr(77, "casassg/psy", "fix: lane order lost on restart", "OPEN", false, Ci::Pass, Review::None, 50),
        pr(1199, "acme/pipelines", "feat: yaml anchors in pipeline configs", "MERGED", false, Ci::Pass, Review::Approved, 20),
        pr(1188, "acme/pipelines", "fix: retry flaky s3 uploads", "MERGED", false, Ci::Pass, Review::Approved, 44),
        pr(1180, "acme/pipelines", "chore: pin ruff", "MERGED", false, Ci::Pass, Review::Approved, 60),
        pr(85, "acme/fleet", "chore: drop unused node selectors", "CLOSED", false, Ci::Fail, Review::None, 70),
    ];
    let mut reviews = vec![
        pr(5501, "acme/fleet", "feat: per-team gpu quotas", "OPEN", false, Ci::Pass, Review::None, 4),
        pr(5502, "acme/serving", "fix: model warmup race", "OPEN", false, Ci::Pending, Review::None, 9),
        pr(5490, "acme/pipelines", "feat: dataset lineage in run metadata", "OPEN", false, Ci::Pass, Review::Changes, 26),
        pr(5470, "acme/serving", "chore: retire v1 predict route", "OPEN", false, Ci::Pass, Review::Approved, 40),
        pr(5450, "acme/fleet", "fix: stale workload admission", "MERGED", false, Ci::Pass, Review::Approved, 60),
    ];
    for (r, author) in reviews.iter_mut().zip(["alice", "bob", "alice", "carol", "bob"]) {
        r.author = author.into();
    }
    reviews[0].requested = true;
    reviews[1].requested = true;
    reviews[2].my_review = Some("CHANGES_REQUESTED".into());
    reviews[3].my_review = Some("APPROVED".into());
    reviews[4].my_review = Some("APPROVED".into());
    s.prs[github::REVIEWS] = reviews;
    s.loaded = [true, true];
    let mine = &mut s.saved.boards[github::MINE];
    mine.move_card("https://github.com/acme/fleet/pull/88", Target::New(9));
    mine.move_card("https://github.com/acme/fleet/pull/91", Target::Lane(1));
    mine.move_card("https://github.com/acme/fleet/pull/85", Target::Lane(1));
    if let Some(l) = mine.lane_mut(1) {
        l.name = "Fleet scheduling".into();
    }
    mine.move_card("https://github.com/acme/pipelines/pull/1210", Target::New(9));
    mine.move_card("https://github.com/acme/pipelines/pull/1203", Target::Lane(2));
    mine.move_card("https://github.com/acme/pipelines/pull/1199", Target::Lane(2));
    mine.move_card("https://github.com/acme/pipelines/pull/1188", Target::Lane(2));
    mine.move_card("https://github.com/acme/pipelines/pull/1180", Target::Lane(2));
    if let Some(l) = mine.lane_mut(2) {
        l.name = "Pipelines".into();
    }
}

#[cfg(test)]
mod tests;
