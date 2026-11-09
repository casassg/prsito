# prsito

A macOS kanban of the user's GitHub PRs, built on Rune (`github.com/anomalyco/rune`, git dep pinned to a `dev` rev in `Cargo.toml`). Rune has no docs site in this repo: read its source from the cargo git checkout, which Hermit puts in `.hermit/rust/git/checkouts/rune-*` (not `~/.cargo`). Start with `crates/rune/README.md`.

## Commands

- Run `. bin/activate-hermit` first. Hermit's rustup honours `rust-toolchain.toml` (1.99.1, the same as Rune's); the system cargo builds the wrong toolchain.
- `cargo test`: unit and headless UI tests, no network. `cargo test live_fetch -- --ignored --nocapture` hits real GitHub with `gh auth token` and takes minutes.
- `cargo clippy --all-targets`: keep it at zero warnings.
- `scripts/release.sh [version]`: release build, `.app` bundle (icon from `assets/icon.png`, ad hoc signed), `target/prsito.dmg`, then quits prsito and replaces `/Applications/prsito.app`.
- When a change is finished (tests and clippy green), run `scripts/release.sh` without asking so the installed app picks it up. Skip it for doc-only or test-only changes.

## Headless checks (Rune tooling, no window)

- `./target/release/prsito --snap out.png --scene NAME --scale 1`. Scenes (`main.rs`): `fixture`, `reviews`, `settings`, `detail`, `menu`, `armed`, `black`, `white`, `accent`. They load `fixture()` sample PRs; headless runs never fetch or touch the database.
- `--script scripts/drag.json --snap after.png --at mark:end` replays input; `--tree tree.json --at T` dumps the element tree.
- UI tests drive `rune::Runtime` (`pointer_down/move/up`, `frame`, `find(key)`, `presented(key)`). Element keys are paths split on `/`, so card keys swap the repo's `/` for `-` (`card-casassg-psy-77`); tests rely on keys like `lane-{id}/head/grip` and `newlane`.

## Layout

- `src/github.rs`: GraphQL over `ureq` with `gh auth token` (cached; falls back to `/opt/homebrew/bin/gh` and `/usr/local/bin/gh`, since Finder launches get a bare PATH). Holds column classification, searches, incremental fetch and the PR mutations.
- `src/store.rs`: lane logic (`Lanes`, `Target`) and SQLite at `~/.config/prsito/prsito.db` (settings, lanes, assignments, PR cache). A new db imports an old `state.json` beside it.
- `src/main.rs`: `App` state, `poll` (fetch scheduling), scenes, `fixture`. `src/view.rs`: the whole UI and drag handling. Tests live in `src/tests.rs`.

## GitHub gotchas (learned the hard way)

- Search is slow (seconds per page). Asking for CI rollups or diff stats inside a search, or batching more than about 25 node ids, gets 502s. Parallel heavy queries tripped the secondary rate limit, which blocked the account's API for minutes. Keep requests sequential and avoid repeated live runs while debugging.
- The fetch is incremental: cheap searches (`id updatedAt state`), then full details only for new or changed PRs. CI changes don't bump `updatedAt`, so cached PRs with pending CI get a separate CI look.
- `act()` runs real mutations (squash merge, rebase, close). Never call it from tests or against real PRs while experimenting.
- Reviews use `user-review-requested:@me`; `review-requested:@me` includes team requests and returns 300+ PRs for this user.

## Behaviour that's easy to break

- A new fetch waits until the running one lands. Stale results are dropped by `generation`, so bump it whenever settings change what to fetch.
- Lane assignments survive PRs hidden by filters. They're pruned only after 30 days unseen (`store::KEEP`); empty lanes (except Inbox, id 0, always last) are removed.
- Drops are hit-tested against window rects that `on_layout_tx` handlers store in `App::rects` (with `tx.live_only()`), not against Rune's drop targets. Clear stale `new:*` rects when the hover target changes.

## Conventions

- Branches `gerardc/<name>`. PR titles take bare `feat:`, `fix:` or `chore:` prefixes.
- No code comments except doc comments on non-obvious behaviour; match the existing terse style. Few files, no new abstractions.
