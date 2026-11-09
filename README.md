# prsito

My GitHub PRs as a small kanban board, built with [Rune](https://github.com/anomalyco/rune).

## Boards

| Board | Columns |
|---|---|
| **Mine** | Draft, Validating (CI running), Needs attention (CI failed or changes requested), Ready, Merged (last N days) |
| **Reviews** | To review, Waiting on author, Approved, Done |

## Using it

- **Open a PR**: click its card. ⌘R refreshes.
- **Organize**: drag cards between swimlanes. Drop a card in the gap between lanes (or above the top one) to start a new lane. Empty lanes go away.
- **Lanes**: drag a title to reorder, double click to rename, click the chevron to collapse.
- **Settings**: merged window, refresh interval, closed toggle and extra search terms. They save as you type.

## Install

You need [`gh`](https://cli.github.com) logged in (`gh auth login`).

| Platform | How |
|---|---|
| macOS (Apple silicon) | `curl -fsSL https://raw.githubusercontent.com/casassg/prsito/main/install.sh \| sh`, installs to `/Applications`. |
| Linux (x86_64) | Same script, installs to `~/.local/bin`. |
| Windows (x86_64) | Unzip `prsito-windows-x86_64.zip` from the [latest release](https://github.com/casassg/prsito/releases/latest) into a writable folder. |

prsito updates itself. It checks for a new release on launch and every 6 hours, then shows an **Update and relaunch** banner. Running the install script again also updates.

Notes:
- The menu bar item is macOS only.
- Downloading `prsito.dmg` by hand works too, but the app isn't notarized, so the first launch needs System Settings → Privacy & Security → Open Anyway. The script skips that.
- Auth comes from `gh auth token`. Settings, lanes and cached PRs live in SQLite at `~/.config/prsito/prsito.db` (`$XDG_CONFIG_HOME` on Linux, `%APPDATA%` on Windows), so the board shows at once on launch.

## Develop

```sh
. bin/activate-hermit
cargo run --release
cargo test                    # add -- --ignored to hit GitHub for real
scripts/release.sh            # builds target/prsito.dmg and installs it (no update checks)
scripts/release.sh 1.2.3      # stamps a version so the build checks for updates (CI does this)
```
