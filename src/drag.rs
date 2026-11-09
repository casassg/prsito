use rune::{Drag, Phase, Tx};

use crate::github::Act;
use crate::store::{INBOX, Target};
use crate::view::menu::{act_name, actions};
use crate::App;

#[derive(Clone, Debug, PartialEq)]
pub enum Dragging {
    Card(String),
    Lane(u64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DragState {
    pub what: Dragging,
    /// Where the dragged element sat when the drag started, in window points.
    pub origin: [f32; 4],
    pub dx: f32,
    pub dy: f32,
    pub over: Option<Target>,
    /// An action pill the card is over, instead of a lane.
    pub act: Option<Act>,
}

const SLOP: f32 = 4.0;
/// How close to a lane's top or bottom edge a carried card offers a new lane there.
const EDGE: f32 = 16.0;

/// Which of the carried card's pills the pointer at (`x`, `y`) is over.
fn act_at(s: &App, url: &str, x: f32, y: f32) -> Option<Act> {
    let pr = s.prs[s.board()].iter().find(|pr| pr.url == url)?;
    actions(s, pr).into_iter().map(|(a, _)| a).find(|a| {
        let name = act_name(*a);
        s.rects.iter().any(|(k, r)| {
            (k == &format!("act:{name}") || k.starts_with(&format!("act:{name}:")))
                && x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3]
        })
    })
}

/// What the pointer at `y` is over. A carried card over the new-lane zone keeps it; near a
/// lane's top edge, in the gap above it or anywhere above the first lane it offers a new lane
/// before it, and near a lane's bottom edge one after it (Inbox stays last). A dragged lane
/// header only looks for the lane it's over.
fn target_at(s: &App, y: f32, card: bool) -> Option<Target> {
    let lanes = &s.lanes().lanes;
    let rect = |id: u64| s.rects.get(&format!("lane:{id}")).copied();
    if !card {
        return lanes.iter().find(|l| rect(l.id).is_some_and(|r| y >= r[1] && y <= r[1] + r[3])).map(|l| Target::Lane(l.id));
    }
    let zone = s.rects.iter().find_map(|(k, r)| {
        let at = k.strip_prefix("new:")?.parse().ok()?;
        (y >= r[1] && y <= r[1] + r[3]).then_some(Target::New(at))
    });
    if zone.is_some() {
        return zone;
    }
    for (i, l) in lanes.iter().enumerate() {
        let Some([_, top, _, h]) = rect(l.id) else { continue };
        if y < top + EDGE.min(h / 3.0) {
            return Some(Target::New(i));
        }
        if y <= top + h {
            if l.id != INBOX && y > top + h - EDGE.min(h / 3.0) {
                return Some(Target::New(i + 1));
            }
            return Some(Target::Lane(l.id));
        }
    }
    None
}

/// One drag handler for cards and lane headers: past a few points of travel the element lifts
/// into a ghost that follows the pointer, and the lane under the pointer is the drop target.
/// A press that never travels is a click (cards open, lane titles rename on double click).
pub(crate) fn drag(s: &mut App, what: Dragging, d: &mut Drag, tx: &mut Tx) {
    let rect_key = match &what {
        Dragging::Card(url) => format!("card:{url}"),
        Dragging::Lane(id) => format!("lane:{id}"),
    };
    match d.phase {
        Phase::Down => {}
        Phase::Move => {
            if s.drag.is_none() {
                if d.dx.hypot(d.dy) < SLOP || what == Dragging::Lane(INBOX) {
                    return;
                }
                let Some(origin) = s.rects.get(&rect_key).copied() else { return };
                let origin = match what {
                    Dragging::Card(_) => origin,
                    Dragging::Lane(_) => [origin[0], origin[1], origin[2], 46.0],
                };
                s.rects.retain(|k, _| !k.starts_with("act:"));
                s.drag = Some(DragState { what: what.clone(), origin, dx: 0.0, dy: 0.0, over: None, act: None });
                s.renaming = None;
            }
            let card = matches!(what, Dragging::Card(_));
            let Some(ds) = &s.drag else { return };
            let (x, y) = (ds.origin[0] + d.start_x + d.dx, ds.origin[1] + d.start_y + d.dy);
            let [ox, oy, ow, oh] = ds.origin;
            let home = x >= ox && x <= ox + ow && y >= oy && y <= oy + oh;
            let act = match &what {
                Dragging::Card(url) if !home => act_at(s, url, x, y),
                _ => None,
            };
            let over = if act.is_some() { None } else { target_at(s, y, card) };
            if s.drag.as_ref().is_some_and(|ds| ds.over != over) {
                s.rects.retain(|k, _| !k.starts_with("new:"));
                tx.animated();
            }
            if s.drag.as_ref().is_some_and(|ds| ds.act != act) {
                tx.instant();
            }
            if let Some(ds) = &mut s.drag {
                ds.dx = d.dx;
                ds.dy = d.dy;
                ds.over = over;
                ds.act = act;
            }
        }
        Phase::Up => {
            let Some(ds) = s.drag.take() else {
                if d.cancelled() {
                    return;
                }
                match &what {
                    Dragging::Card(url) => s.open_detail(url.clone()),
                    Dragging::Lane(id) if d.clicks >= 2 => s.start_rename(*id),
                    Dragging::Lane(_) => {}
                }
                return;
            };
            if d.cancelled() {
                return;
            }
            s.rects.retain(|k, _| !k.starts_with("act:"));
            match (&ds.what, ds.over) {
                (Dragging::Card(url), _) if ds.act == Some(Act::RequestChanges) => s.ask_changes(url),
                (Dragging::Card(url), _) if let Some(act) = ds.act => s.run(act, url),
                (Dragging::Card(url), Some(to)) => {
                    s.lanes_mut().move_card(url, to);
                    s.persist();
                }
                (Dragging::Lane(id), Some(Target::Lane(onto))) if onto != *id => {
                    let lanes = s.lanes_mut();
                    if let Some(to) = lanes.lanes.iter().position(|l| l.id == onto) {
                        lanes.reorder(*id, to);
                    }
                    s.persist();
                }
                _ => {}
            }
        }
    }
}
