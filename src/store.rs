use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::github::Pr;

pub const INBOX: u64 = 0;
const KEEP: i64 = 30 * 86_400;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub days: u32,
    pub interval: u32,
    pub include_closed: bool,
    pub filter: String,
    /// `black` (pure black for OLED screens), `white`, or `accent` (dark, tinted with the system accent). Empty until the first frame picks black or white from macOS.
    pub theme: String,
    /// A menu bar item lists my PRs that need attention or are ready to merge; closing the window then keeps prsito running.
    pub menu_bar: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { days: 7, interval: 120, include_closed: true, filter: String::new(), theme: String::new(), menu_bar: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub collapsed: bool,
}

/// One board's lanes, top to bottom, with Inbox always last, and which lane each PR (by URL)
/// was dropped in. A PR with no lane sits in Inbox.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lanes {
    pub lanes: Vec<Lane>,
    pub assign: HashMap<String, u64>,
    /// When each assigned PR last showed up in a fetch (seconds since the epoch).
    pub seen: HashMap<String, i64>,
    pub next: u64,
}

impl Default for Lanes {
    fn default() -> Self {
        Lanes { lanes: vec![Lane { id: INBOX, name: "Inbox".into(), collapsed: false }], assign: HashMap::new(), seen: HashMap::new(), next: 1 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Lane(u64),
    /// A new lane, inserted at this index (never below Inbox).
    New(usize),
}

impl Lanes {
    /// Puts Inbox back at the bottom and drops assignments to lanes that no longer exist.
    pub fn normalize(&mut self) {
        let inbox = match self.lanes.iter().position(|l| l.id == INBOX) {
            Some(i) => self.lanes.remove(i),
            None => Lane { id: INBOX, name: "Inbox".into(), collapsed: false },
        };
        self.lanes.push(inbox);
        let ids: Vec<u64> = self.lanes.iter().map(|l| l.id).collect();
        self.assign.retain(|_, id| ids.contains(id) && *id != INBOX);
        self.next = self.next.max(ids.iter().max().copied().unwrap_or(0) + 1);
    }

    pub fn lane_of(&self, url: &str) -> u64 {
        self.assign.get(url).copied().unwrap_or(INBOX)
    }

    /// Moves a card to a lane, or to a new lane made for it. Returns the lane it landed in.
    pub fn move_card(&mut self, url: &str, to: Target) -> u64 {
        let id = match to {
            Target::Lane(id) if self.lanes.iter().any(|l| l.id == id) => id,
            Target::Lane(_) => INBOX,
            Target::New(at) => {
                let id = self.next;
                self.next += 1;
                let at = at.min(self.lanes.len() - 1);
                self.lanes.insert(at, Lane { id, name: format!("Lane {id}"), collapsed: false });
                id
            }
        };
        if id == INBOX {
            self.assign.remove(url);
        } else {
            self.assign.insert(url.to_string(), id);
        }
        self.drop_empty();
        id
    }

    /// Notes which PRs a fetch showed, then forgets ones unseen for `KEEP` seconds and removes
    /// lanes left empty. PRs hidden by a filter or a shorter window keep their lane meanwhile.
    pub fn prune(&mut self, urls: &[&str], now: i64) {
        for url in urls {
            if self.assign.contains_key(*url) {
                self.seen.insert(url.to_string(), now);
            }
        }
        let seen = &mut self.seen;
        self.assign.retain(|url, _| *seen.entry(url.clone()).or_insert(now) > now - KEEP);
        seen.retain(|url, _| self.assign.contains_key(url));
        self.drop_empty();
    }

    fn drop_empty(&mut self) {
        let used: Vec<u64> = self.assign.values().copied().collect();
        self.lanes.retain(|l| l.id == INBOX || used.contains(&l.id));
    }

    /// Moves a lane to `to` (an index), never below Inbox.
    pub fn reorder(&mut self, id: u64, to: usize) {
        let Some(from) = self.lanes.iter().position(|l| l.id == id) else { return };
        if id == INBOX {
            return;
        }
        let lane = self.lanes.remove(from);
        self.lanes.insert(to.min(self.lanes.len() - 1), lane);
    }

    pub fn lane_mut(&mut self, id: u64) -> Option<&mut Lane> {
        self.lanes.iter_mut().find(|l| l.id == id)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub settings: Settings,
    pub boards: [Lanes; 2],
}

pub fn path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/prsito/prsito.db"))
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS lanes (
        board INTEGER NOT NULL, id INTEGER NOT NULL, name TEXT NOT NULL,
        collapsed INTEGER NOT NULL DEFAULT 0, position INTEGER NOT NULL,
        PRIMARY KEY (board, id));
    CREATE TABLE IF NOT EXISTS assignments (
        board INTEGER NOT NULL, url TEXT NOT NULL, lane INTEGER NOT NULL, seen INTEGER,
        PRIMARY KEY (board, url));
    CREATE TABLE IF NOT EXISTS prs (
        board INTEGER NOT NULL, position INTEGER NOT NULL, data TEXT NOT NULL,
        PRIMARY KEY (board, position));";

fn open(path: &Path) -> Result<Connection, String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    db.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
    Ok(db)
}

/// Settings and lanes. A new database takes over a `state.json` beside it, from before SQLite.
pub fn load(path: &Path) -> Saved {
    let fresh = !path.exists();
    let mut saved = match open(path).and_then(|db| read(&db)) {
        Ok(saved) => saved,
        Err(e) => {
            eprintln!("prsito: can't read {}: {e}", path.display());
            Saved::default()
        }
    };
    let old = path.with_file_name("state.json");
    if fresh && let Some(imported) = std::fs::read_to_string(&old).ok().and_then(|s| serde_json::from_str::<Saved>(&s).ok()) {
        saved = imported;
        if let Err(e) = save(path, &saved) {
            eprintln!("prsito: can't import {}: {e}", old.display());
        }
    }
    for b in &mut saved.boards {
        b.normalize();
    }
    saved
}

fn read(db: &Connection) -> Result<Saved, String> {
    let e = |e: rusqlite::Error| e.to_string();
    let mut saved = Saved::default();
    let mut q = db.prepare("SELECT key, value FROM settings").map_err(e)?;
    let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(e)?;
    let st = &mut saved.settings;
    for row in rows {
        let (key, value) = row.map_err(e)?;
        match key.as_str() {
            "days" => st.days = value.parse().unwrap_or(st.days),
            "interval" => st.interval = value.parse().unwrap_or(st.interval),
            "include_closed" => st.include_closed = value == "1",
            "filter" => st.filter = value,
            "theme" => st.theme = value,
            "menu_bar" => st.menu_bar = value == "1",
            k if k.starts_with("next:") => {
                if let Some(b) = k[5..].parse::<usize>().ok().and_then(|b| saved.boards.get_mut(b)) {
                    b.next = value.parse().unwrap_or(b.next);
                }
            }
            _ => {}
        }
    }
    for (board, lanes) in saved.boards.iter_mut().enumerate() {
        let mut q = db.prepare("SELECT id, name, collapsed FROM lanes WHERE board = ?1 ORDER BY position").map_err(e)?;
        let rows = q.query_map([board as i64], |r| Ok(Lane { id: r.get::<_, i64>(0)? as u64, name: r.get(1)?, collapsed: r.get(2)? })).map_err(e)?;
        let found: Vec<Lane> = rows.collect::<Result<_, _>>().map_err(e)?;
        if !found.is_empty() {
            lanes.lanes = found;
        }
        let mut q = db.prepare("SELECT url, lane, seen FROM assignments WHERE board = ?1").map_err(e)?;
        let rows = q.query_map([board as i64], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64, r.get::<_, Option<i64>>(2)?))).map_err(e)?;
        for row in rows {
            let (url, lane, seen) = row.map_err(e)?;
            if let Some(t) = seen {
                lanes.seen.insert(url.clone(), t);
            }
            lanes.assign.insert(url, lane);
        }
    }
    Ok(saved)
}

pub fn save(path: &Path, saved: &Saved) -> Result<(), String> {
    let e = |e: rusqlite::Error| e.to_string();
    let mut db = open(path)?;
    let tx = db.transaction().map_err(e)?;
    tx.execute_batch("DELETE FROM settings; DELETE FROM lanes; DELETE FROM assignments;").map_err(e)?;
    let st = &saved.settings;
    let mut settings = vec![
        ("days".to_string(), st.days.to_string()),
        ("interval".to_string(), st.interval.to_string()),
        ("include_closed".to_string(), if st.include_closed { "1" } else { "0" }.to_string()),
        ("filter".to_string(), st.filter.clone()),
        ("theme".to_string(), st.theme.clone()),
        ("menu_bar".to_string(), if st.menu_bar { "1" } else { "0" }.to_string()),
    ];
    for (board, lanes) in saved.boards.iter().enumerate() {
        settings.push((format!("next:{board}"), lanes.next.to_string()));
        for (position, l) in lanes.lanes.iter().enumerate() {
            tx.execute("INSERT INTO lanes (board, id, name, collapsed, position) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![board as i64, l.id as i64, l.name, l.collapsed, position as i64]).map_err(e)?;
        }
        for (url, lane) in &lanes.assign {
            tx.execute("INSERT INTO assignments (board, url, lane, seen) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![board as i64, url, *lane as i64, lanes.seen.get(url)]).map_err(e)?;
        }
    }
    for (key, value) in settings {
        tx.execute("INSERT INTO settings (key, value) VALUES (?1, ?2)", [key, value]).map_err(e)?;
    }
    tx.commit().map_err(e)
}

/// A connection held open to notice writes to the database, like the CLI's.
pub struct Watch {
    db: Connection,
    version: i64,
}

impl Watch {
    pub fn open(path: &Path) -> Option<Watch> {
        let db = open(path).ok()?;
        let version = db.query_row("PRAGMA data_version", [], |r| r.get(0)).ok()?;
        Some(Watch { db, version })
    }

    /// Whether any other connection, this process's saves included, wrote since the last call.
    pub fn changed(&mut self) -> bool {
        let v = self.db.query_row("PRAGMA data_version", [], |r| r.get(0)).unwrap_or(self.version);
        std::mem::replace(&mut self.version, v) != v
    }
}

/// The last PRs each board fetched, so a launch shows the board before GitHub answers.
pub fn load_cache(path: &Path) -> Option<[Vec<Pr>; 2]> {
    let db = open(path).ok()?;
    let mut out = [Vec::new(), Vec::new()];
    for (board, prs) in out.iter_mut().enumerate() {
        let mut q = db.prepare("SELECT data FROM prs WHERE board = ?1 ORDER BY position").ok()?;
        let rows = q.query_map([board as i64], |r| r.get::<_, String>(0)).ok()?;
        for data in rows.flatten() {
            prs.push(serde_json::from_str(&data).ok()?);
        }
    }
    out.iter().any(|b| !b.is_empty()).then_some(out)
}

pub fn save_cache(path: &Path, prs: &[Vec<Pr>; 2]) -> Result<(), String> {
    let e = |e: rusqlite::Error| e.to_string();
    let mut db = open(path)?;
    let tx = db.transaction().map_err(e)?;
    tx.execute("DELETE FROM prs", []).map_err(e)?;
    for (board, list) in prs.iter().enumerate() {
        for (position, pr) in list.iter().enumerate() {
            let data = serde_json::to_string(pr).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO prs (board, position, data) VALUES (?1, ?2, ?3)", rusqlite::params![board as i64, position as i64, data]).map_err(e)?;
        }
    }
    tx.commit().map_err(e)
}
