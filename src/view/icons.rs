use rune::Icon;

use crate::github::Act;

pub(crate) static LOGO: Icon = Icon {
    name: "prsito-logo",
    view: 24.0,
    body: r##"<path d="M6.5 7.5V16.5" stroke="currentColor" stroke-width="2" fill="none"/><circle cx="6.5" cy="5.5" r="2.6" fill="currentColor"/><circle cx="6.5" cy="18.5" r="2.6" fill="currentColor"/><path d="M17.5 16V9.5A3 3 0 0 0 14.5 6.5H10.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" fill="none"/><path d="M12.8 4L10.3 6.5L12.8 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="17.5" cy="18.5" r="2.6" fill="#5fc38f"/>"##,
};

pub(crate) static BRANCH: Icon = Icon {
    name: "prsito-branch",
    view: 24.0,
    body: r#"<path d="M6 3V15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="18" cy="6" r="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="6" cy="18" r="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M18 9A9 9 0 0 1 9 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static EYE: Icon = Icon {
    name: "prsito-eye",
    view: 24.0,
    body: r#"<path d="M1 12S5 4 12 4 23 12 23 12 19 20 12 20 1 12 1 12Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static REFRESH: Icon = Icon {
    name: "prsito-refresh",
    view: 24.0,
    body: r#"<path d="M21 12A9 9 0 1 1 12 3C14.52 3 16.93 4 18.74 5.74L21 8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M21 3L21 8L16 8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static GEAR: Icon = Icon {
    name: "prsito-gear",
    view: 24.0,
    body: r#"<circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static EXTERNAL: Icon = Icon {
    name: "prsito-external",
    view: 24.0,
    body: r#"<path d="M18 13V19A2 2 0 0 1 16 21H5A2 2 0 0 1 3 19V8A2 2 0 0 1 5 6H11" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M15 3H21V9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M10 14L21 3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static COPY: Icon = Icon {
    name: "prsito-copy",
    view: 24.0,
    body: r#"<rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M5 15H4A2 2 0 0 1 2 13V4A2 2 0 0 1 4 2H13A2 2 0 0 1 15 4V5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static MERGE: Icon = Icon {
    name: "prsito-merge",
    view: 24.0,
    body: r#"<circle cx="18" cy="18" r="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="6" cy="6" r="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M6 21V9A9 9 0 0 0 15 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static REBASE: Icon = Icon {
    name: "prsito-rebase",
    view: 24.0,
    body: r#"<path d="M12 3V15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M7 10L12 15L17 10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M5 21H19" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static SEND: Icon = Icon {
    name: "prsito-send",
    view: 24.0,
    body: r#"<path d="M22 2L11 13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M22 2L15 22L11 13L2 9Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static PENCIL: Icon = Icon {
    name: "prsito-pencil",
    view: 24.0,
    body: r#"<path d="M12 20H21" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M16.5 3.5A2.12 2.12 0 0 1 19.5 6.5L7 19L3 20L4 16Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static X: Icon = Icon {
    name: "prsito-x",
    view: 24.0,
    body: r#"<path d="M18 6L6 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M6 6L18 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static CHECK: Icon = Icon {
    name: "prsito-check",
    view: 24.0,
    body: r#"<path d="M20 6L9 17L4 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static PLUS: Icon = Icon {
    name: "prsito-plus",
    view: 24.0,
    body: r#"<path d="M12 5V19" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M5 12H19" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static COMMENT: Icon = Icon {
    name: "prsito-comment",
    view: 24.0,
    body: r#"<path d="M21 15A2 2 0 0 1 19 17H7L3 21V5A2 2 0 0 1 5 3H19A2 2 0 0 1 21 5Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static USER_MINUS: Icon = Icon {
    name: "prsito-user-minus",
    view: 24.0,
    body: r#"<path d="M16 21V19A4 4 0 0 0 12 15H5A4 4 0 0 0 1 19V21" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="8.5" cy="7" r="4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M23 11H17" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) static UNDO: Icon = Icon {
    name: "prsito-undo",
    view: 24.0,
    body: r#"<path d="M3 7V13H9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M21 17A9 9 0 0 0 6 10.3L3 13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub(crate) fn act_icon(act: Act) -> &'static Icon {
    match act {
        Act::Merge => &MERGE,
        Act::Rebase => &REBASE,
        Act::Close => &X,
        Act::Publish => &SEND,
        Act::Draft => &PENCIL,
        Act::Approve => &CHECK,
        Act::RequestChanges => &COMMENT,
        Act::Uninvolve => &USER_MINUS,
        Act::Dismiss => &UNDO,
    }
}
