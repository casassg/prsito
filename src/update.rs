//! Self-update from the latest GitHub release of casassg/prsito.

use std::path::Path;
use std::process::Command;

/// The release version CI stamps in (`PRSITO_VERSION`); `None` for local builds, which never update.
pub const VERSION: Option<&str> = option_env!("PRSITO_VERSION");

const APP: &str = "/Applications/prsito.app";

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub dmg: String,
}

/// The latest release when it's newer than `current`.
pub fn check(current: &str) -> Result<Option<Release>, String> {
    let text = ureq::get("https://api.github.com/repos/casassg/prsito/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "prsito")
        .call()
        .and_then(|mut r| r.body_mut().read_to_string())
        .map_err(|e| format!("can't check for updates: {e}"))?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("can't check for updates: {e}"))?;
    let version = body["tag_name"].as_str().unwrap_or_default().trim_start_matches('v').to_string();
    let dmg = body["assets"].as_array().into_iter().flatten()
        .find(|a| a["name"] == "prsito.dmg")
        .and_then(|a| a["browser_download_url"].as_str());
    Ok(match dmg {
        Some(dmg) if newer(&version, current) => Some(Release { version, dmg: dmg.to_string() }),
        _ => None,
    })
}

/// `a > b` as dotted numbers; anything that doesn't parse is never newer.
pub fn newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| v.trim_start_matches('v').split('.').map(|n| n.parse::<u64>().ok()).collect::<Option<Vec<_>>>();
    match (parse(a), parse(b)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Downloads the release's dmg, swaps it into /Applications and relaunches. Only returns on failure.
pub fn install(r: &Release) -> String {
    let dir = std::env::temp_dir().join(format!("prsito-update-{}", r.version));
    let err = match fetch_and_swap(r, &dir) {
        Ok(()) => match Command::new("open").args(["-n", APP]).status() {
            Ok(s) if s.success() => std::process::exit(0),
            _ => "updated, but couldn't relaunch prsito".to_string(),
        },
        Err(e) => e,
    };
    let _ = std::fs::remove_dir_all(&dir);
    format!("update failed: {err}")
}

fn fetch_and_swap(r: &Release, dir: &Path) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let dmg = dir.join("prsito.dmg");
    let mut body = ureq::get(&r.dmg).header("User-Agent", "prsito").call().map_err(|e| e.to_string())?.into_body();
    let mut file = std::fs::File::create(&dmg).map_err(|e| e.to_string())?;
    std::io::copy(&mut body.as_reader(), &mut file).map_err(|e| e.to_string())?;
    let mount = dir.join("mnt");
    run(Command::new("hdiutil").arg("attach").arg(&dmg).args(["-nobrowse", "-readonly", "-mountpoint"]).arg(&mount))?;
    let staged = Path::new("/Applications/.prsito.new.app");
    let _ = std::fs::remove_dir_all(staged);
    let copied = run(Command::new("cp").arg("-R").arg(mount.join("prsito.app")).arg(staged));
    let _ = Command::new("hdiutil").arg("detach").arg(&mount).arg("-quiet").status();
    copied?;
    let _ = std::fs::remove_dir_all(APP);
    std::fs::rename(staged, APP).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(dir);
    Ok(())
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    match out.status.success() {
        true => Ok(()),
        false => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
    }
}
