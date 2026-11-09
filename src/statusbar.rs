//! The optional menu bar item: my PRs that need attention or are ready to merge, one click from
//! the browser. AppKit work stays on the main thread, so the item lives in a thread local.

#[cfg(target_os = "macos")]
use std::cell::RefCell;

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::{AnyObject, NSObjectProtocol, Sel};
#[cfg(target_os = "macos")]
use objc2::{AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send, sel};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSWindowStyleMask};
#[cfg(target_os = "macos")]
use objc2_foundation::{MainThreadMarker, NSData, NSObject, NSSize, NSString};
use rune::AppHandle;

use crate::App;
use crate::github::{self, Ci, Pr, Review};

/// A PR row of the menu: `ready` to merge, or needing attention. `detail` says where and why,
/// `symbol` is the SF Symbol beside it.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub struct Entry {
    pub ready: bool,
    pub title: String,
    pub detail: String,
    pub symbol: &'static str,
    pub url: String,
}

/// My open PRs with failing CI or changes requested, then those approved with CI green, each
/// group newest first.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn entries(prs: &[Pr]) -> Vec<Entry> {
    let mut found: Vec<&Pr> = prs.iter().filter(|p| match github::column(github::MINE, p) {
        2 => true,
        3 => p.review == Review::Approved,
        _ => false,
    }).collect();
    found.sort_by_key(|p| (github::column(github::MINE, p), -p.updated));
    found.into_iter().map(|p| {
        let ready = github::column(github::MINE, p) == 3;
        let (why, symbol) = match (p.ci == Ci::Fail, p.review == Review::Changes) {
            _ if ready => ("approved, CI green", "checkmark.circle"),
            (true, true) => ("CI failed, changes requested", "xmark.circle"),
            (true, false) => ("CI failed", "xmark.circle"),
            _ => ("changes requested", "exclamationmark.bubble"),
        };
        let title = if p.title.chars().count() > 56 { format!("{}…", p.title.chars().take(55).collect::<String>().trim_end()) } else { p.title.clone() };
        let repo = p.repo.rsplit('/').next().unwrap_or(&p.repo);
        Entry { ready, title, detail: format!("{repo} #{} · {why}", p.number), symbol, url: p.url.clone() }
    }).collect()
}

#[cfg(target_os = "macos")]
struct Ivars {
    handle: AppHandle<App>,
}

#[cfg(target_os = "macos")]
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct PrsitoMenuTarget;

    unsafe impl NSObjectProtocol for PrsitoMenuTarget {}

    impl PrsitoMenuTarget {
        #[unsafe(method(openPr:))]
        fn open_pr(&self, sender: &NSMenuItem) {
            let url = BAR.with(|b| b.borrow().as_ref().and_then(|bar| bar.shown.get(sender.tag() as usize).map(|e| e.url.clone())));
            if let Some(url) = url {
                self.ivars().handle.update(move |s, tx| {
                    if let Err(e) = tx.open_url(&url) {
                        s.error[github::MINE] = Some(format!("can't open {url}: {e:?}"));
                    }
                });
            }
        }

        #[unsafe(method(openApp:))]
        fn open_app(&self, _sender: &AnyObject) {
            dock(true);
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: &AnyObject) {
            self.ivars().handle.quit();
        }
    }
);

#[cfg(target_os = "macos")]
struct Bar {
    item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    target: Retained<PrsitoMenuTarget>,
    shown: Vec<Entry>,
    built: bool,
}

#[cfg(target_os = "macos")]
thread_local! {
    static HANDLE: RefCell<Option<AppHandle<App>>> = const { RefCell::new(None) };
    static BAR: RefCell<Option<Bar>> = const { RefCell::new(None) };
}

/// Keeps the handle menu actions reach the app through. Call on the main thread before `run`.
#[cfg(target_os = "macos")]
pub fn connect(handle: AppHandle<App>) {
    HANDLE.with(|h| *h.borrow_mut() = Some(handle));
}

/// Shows the window with its Dock icon, or hides the Dock icon while only the menu bar item is left.
/// A window closed in full screen was only ordered out, so it keeps AppKit's full screen style
/// (screen sized, no title bar) and winit still thinks it's full screen: once it shows again,
/// a real exit brings it back windowed.
#[cfg(target_os = "macos")]
pub fn dock(open: bool) {
    let Some(handle) = HANDLE.with(|h| h.borrow().clone()) else { return };
    handle.set_dock_icon(open);
    if open {
        let hidden = !handle.is_visible();
        handle.open();
        if hidden {
            handle.update(|s, tx| {
                s.unfull = true;
                tx.wake_after(std::time::Duration::from_millis(100));
            });
        }
    }
}

/// Exits a stale full screen once the window shows; false while it's still hidden.
#[cfg(target_os = "macos")]
pub fn unfull() -> bool {
    let Some(mtm) = MainThreadMarker::new() else { return true };
    if !HANDLE.with(|h| h.borrow().as_ref().is_some_and(|h| h.is_visible())) {
        return false;
    }
    let windows = NSApplication::sharedApplication(mtm).windows();
    if let Some(w) = windows.iter().find(|w| w.isVisible() && w.styleMask().contains(NSWindowStyleMask::Titled))
        && w.styleMask().contains(NSWindowStyleMask::FullScreen)
    {
        w.toggleFullScreen(None);
    }
    true
}

/// Shows, updates or removes the item. Off the main thread, or before `connect`, it does nothing.
#[cfg(target_os = "macos")]
pub fn sync(on: bool, prs: &[Pr]) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    BAR.with(|b| {
        let mut b = b.borrow_mut();
        if !on {
            if let Some(bar) = b.take() {
                NSStatusBar::systemStatusBar().removeStatusItem(&bar.item);
            }
            return;
        }
        if b.is_none() {
            let Some(handle) = HANDLE.with(|h| h.borrow().clone()) else { return };
            let this = PrsitoMenuTarget::alloc(mtm).set_ivars(Ivars { handle });
            let target: Retained<PrsitoMenuTarget> = unsafe { msg_send![super(this), init] };
            let menu = NSMenu::new(mtm);
            let item = NSStatusBar::systemStatusBar().statusItemWithLength(-2.0);
            if let Some(button) = item.button(mtm) {
                let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" color="black">{}</svg>"#, crate::view::icons::LOGO.body.replace("#5fc38f", "black"));
                let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(svg.as_bytes()));
                if let Some(image) = &image {
                    image.setSize(NSSize::new(18.0, 18.0));
                    image.setTemplate(true);
                    image.setAccessibilityDescription(Some(&NSString::from_str("prsito")));
                }
                button.setImage(image.as_deref());
            }
            item.setMenu(Some(&menu));
            *b = Some(Bar { item, menu, target, shown: Vec::new(), built: false });
        }
        let Some(bar) = b.as_mut() else { return };
        let shown = entries(prs);
        if bar.built && bar.shown == shown {
            return;
        }
        let add = |title: &str, action: Option<Sel>| {
            let it = unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(mtm), &NSString::from_str(title), action, &NSString::new()) };
            if action.is_some() {
                unsafe { it.setTarget(Some(&bar.target)) };
            }
            bar.menu.addItem(&it);
            it
        };
        bar.menu.removeAllItems();
        for (i, e) in shown.iter().enumerate() {
            if i == 0 || shown[i - 1].ready != e.ready {
                if i > 0 {
                    bar.menu.addItem(&NSMenuItem::separatorItem(mtm));
                }
                let n = shown.iter().filter(|o| o.ready == e.ready).count();
                let name = if e.ready { "Ready to merge" } else { "Needs attention" };
                bar.menu.addItem(&NSMenuItem::sectionHeaderWithTitle(&NSString::from_str(&format!("{name} · {n}")), mtm));
            }
            let it = add(&e.title, Some(sel!(openPr:)));
            it.setTag(i as isize);
            it.setSubtitle(Some(&NSString::from_str(&e.detail)));
            it.setImage(NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(e.symbol), None).as_deref());
            it.setToolTip(Some(&NSString::from_str(&e.url)));
        }
        if shown.is_empty() {
            add("All clear, nothing needs you", None);
        }
        bar.menu.addItem(&NSMenuItem::separatorItem(mtm));
        add("Open prsito", Some(sel!(openApp:)));
        let quit = add("Quit prsito", Some(sel!(quit:)));
        quit.setKeyEquivalent(&NSString::from_str("q"));
        quit.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        if let Some(button) = bar.item.button(mtm) {
            button.setAppearsDisabled(shown.is_empty());
        }
        bar.shown = shown;
        bar.built = true;
    });
}

#[cfg(not(target_os = "macos"))]
pub fn connect(_: AppHandle<App>) {}
#[cfg(not(target_os = "macos"))]
pub fn dock(_: bool) {}
#[cfg(not(target_os = "macos"))]
pub fn unfull() -> bool { true }
#[cfg(not(target_os = "macos"))]
pub fn sync(_: bool, _: &[Pr]) {}
