Thank you for contributing to timeman — your help makes this project better.

This document explains how to set up a development environment, how to make
changes, and what we expect from contributions (PRs). Follow these steps to
move fast and keep the repo healthy.

Getting started (local dev)
- Install Rust (recommended toolchain: 1.88.0). Pin locally if desired:
  `rustup override set 1.88.0`.
- On Windows, ensure C++ build tools / Windows SDK are installed
  ("Desktop development with C++").
- Install frontend deps: `bun install` (or `npm install` / `pnpm install`).
- Start the app for development: `bun run tauri dev` (starts Vite and the native
  Tauri binary).

Building and testing
- Rust backend: `cd src-tauri && cargo +1.88.0 build` or `cargo +1.88.0 test`.
- Frontend: use your package manager scripts (`bun run build`, `bun run dev`,
  `bun run test`) depending on the task.
- Formatting & linting (recommended):
  - `cargo fmt` (Rust)
  - `cargo clippy` (optional static checks)
  - `bun run lint` / `npm run lint` if a linter is configured for the frontend.

Branching & commits
- Work on topic branches named like `feat/<short-desc>`, `fix/<short-desc>`,
  or `chore/<short-desc>`.
- Keep commits small and focused. Use conventional short messages, examples:
  - `feat(tray): left-click toggles hide/show`
  - `fix(autostart): synchronize menu checkbox with backend`
  - `docs(readme): clarify quick start`

Pull request checklist
- Create a descriptive PR title and include a short summary in the body.
- Link the issue that the PR closes (if applicable).
- Include verification steps for the reviewer (how to run, expected behavior).
- Ensure code is formatted (`cargo fmt`) and no obvious lint errors remain.
- Add tests for new behavior when practical.

Code review & merge
- PRs are reviewed for correctness, style, and UX. Address reviewer comments
  in commits or by pushing updates to the same branch.
- After approval, a maintainer will merge the PR (usually via GitHub). If your
  PR requires release notes, add them to the PR body.

Communication
- If you plan a larger change, open an issue first to discuss the approach.
- Ask questions on the PR if something is unclear — we prefer short iterative
  PRs to giant changes.

Thank you — and welcome aboard!
