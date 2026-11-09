//! Self-update from the latest GitHub release of casassg/prsito.

use std::path::Path;
use std::process::Command;

/// The release version CI stamps in (`PRSITO_VERSION`); `None` for local builds, which never update.
pub const VERSION: Option<&str> = option_env!("PRSITO_VERSION");

#[cfg(target_os = "macos")]
const ASSET: &str = "prsito.dmg";
#[cfg(target_os = "windows")]
const ASSET: &str = "prsito-windows-x86_64.zip";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const ASSET: &str = "prsito-linux-x86_64.tar.gz";

#[cfg(target_os = "macos")]
const APP: &str = "/Applications/prsito.app";

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub url: String,
}

/// The latest release when it's newer than `current`. Reads the tag from the redirect of
/// github.com's releases/latest page rather than the API, whose unauthenticated limit (60 an
/// hour per IP) runs out behind shared office or VPN addresses.
pub fn check(current: &str) -> Result<Option<Release>, String> {
    if let Some(old) = old_exe() {
        let _ = std::fs::remove_file(old);
    }
    let res = ureq::get("https://github.com/casassg/prsito/releases/latest")
        .config().max_redirects(0).build()
        .header("User-Agent", "prsito")
        .call()
        .map_err(|e| format!("can't check for updates: {e}"))?;
    let tag = res.headers().get("location").and_then(|l| l.to_str().ok())
        .and_then(|l| l.rsplit_once("/tag/")).map(|(_, t)| t.to_string())
        .ok_or("can't check for updates: no release")?;
    let version = tag.trim_start_matches('v').to_string();
    Ok(newer(&version, current).then(|| Release {
        url: format!("https://github.com/casassg/prsito/releases/download/{tag}/{ASSET}"),
        version,
    }))
}

/// `a > b` as dotted numbers; anything that doesn't parse is never newer.
pub fn newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| v.trim_start_matches('v').split('.').map(|n| n.parse::<u64>().ok()).collect::<Option<Vec<_>>>();
    match (parse(a), parse(b)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Downloads the release, swaps it in for the running app and relaunches. Only returns on failure.
pub fn install(r: &Release) -> String {
    let dir = std::env::temp_dir().join(format!("prsito-update-{}", r.version));
    let err = match fetch(r, &dir).and_then(|file| swap(&file, &dir)) {
        Ok(()) => match relaunch() {
            Ok(()) => std::process::exit(0),
            Err(_) => "updated, but couldn't relaunch prsito".to_string(),
        },
        Err(e) => e,
    };
    let _ = std::fs::remove_dir_all(&dir);
    format!("update failed: {err}")
}

fn fetch(r: &Release, dir: &Path) -> Result<std::path::PathBuf, String> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let file = dir.join(ASSET);
    let mut body = ureq::get(&r.url).header("User-Agent", "prsito").call().map_err(|e| e.to_string())?.into_body();
    let mut out = std::fs::File::create(&file).map_err(|e| e.to_string())?;
    std::io::copy(&mut body.as_reader(), &mut out).map_err(|e| e.to_string())?;
    Ok(file)
}

#[cfg(target_os = "macos")]
fn swap(dmg: &Path, dir: &Path) -> Result<(), String> {
    let mount = dir.join("mnt");
    run(Command::new("hdiutil").arg("attach").arg(dmg).args(["-nobrowse", "-readonly", "-mountpoint"]).arg(&mount))?;
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

/// Extracts the archive and replaces the running binary. The running one is renamed aside first,
/// since Windows can't overwrite it; the next `check` deletes it.
#[cfg(not(target_os = "macos"))]
fn swap(archive: &Path, dir: &Path) -> Result<(), String> {
    run(Command::new("tar").arg("-xf").arg(archive).arg("-C").arg(dir))?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let old = old_exe().ok_or("no exe")?;
    let staged = exe.with_extension("new");
    std::fs::copy(dir.join(if cfg!(windows) { "prsito.exe" } else { "prsito" }), &staged).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(&staged, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(e.to_string());
    }
    let _ = std::fs::remove_dir_all(dir);
    Ok(())
}

fn old_exe() -> Option<std::path::PathBuf> {
    Some(std::env::current_exe().ok()?.with_extension("old"))
}

#[cfg(target_os = "macos")]
fn relaunch() -> Result<(), String> {
    run(Command::new("open").args(["-n", APP]))
}

#[cfg(not(target_os = "macos"))]
fn relaunch() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Command::new(exe).spawn().map(|_| ()).map_err(|e| e.to_string())
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    match out.status.success() {
        true => Ok(()),
        false => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
    }
}
