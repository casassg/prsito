# prsito

My GitHub PRs as a small kanban board, built with [Rune](https://github.com/anomalyco/rune).

- **Mine**: Draft, Validating (CI running), Needs attention (CI failed or changes requested), Ready, Merged (merged or closed in the last N days).
- **Reviews**: To review, Waiting on author, Approved, Done.
- Drag cards between swimlanes. Carry a card to the gap between two lanes (or above the top one) and a zone opens there to start a new lane. Empty lanes go away. Drag a lane title to reorder lanes, double click it to rename, and click the chevron to collapse.
- Click a card to open the PR. ⌘R refreshes. Settings (merged window, refresh interval, closed toggle, extra search terms) save as you type and the board follows at once.
- Auth comes from `gh auth token`. Settings, lanes and the last fetched PRs live in SQLite at `~/.config/prsito/prsito.db`, so a launch shows the board at once and refreshes behind it.

## Install

Needs an Apple silicon Mac and [`gh`](https://cli.github.com) logged in (`gh auth login`).

```sh
curl -fsSL https://raw.githubusercontent.com/casassg/prsito/main/install.sh | sh
```

Run it again to update. Or grab `prsito.dmg` from the [latest release](https://github.com/casassg/prsito/releases/latest) and drag the app to Applications; the app isn't notarized, so a browser download needs System Settings → Privacy & Security → Open Anyway on first launch (the curl install skips that).

## Develop

```sh
. bin/activate-hermit
cargo run --release
scripts/release.sh                           # target/prsito.dmg, and installs /Applications/prsito.app
cargo test                                   # add -- --ignored to hit GitHub for real
./target/release/prsito --snap board.png --scene fixture   # also: reviews, settings
./target/release/prsito --scene fixture --script scripts/drag.json --snap after.png --at mark:end
```
