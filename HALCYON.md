# Halcyon patches for Horizon

This fork carries the changes [Halcyon](https://github.com/MrRed-dev6/Halcyon-Launcher) needs to run Horizon on Android.
Horizon itself is MIT-licensed; see `LICENSE`.

- **Base:** `horizon-rust/horizon` at `275566a6e99f8fb50e7ecdd5b028e53900d848a1` (2026-11-05). Upstream moves fast, so we stay on a
  pinned base and move it on purpose instead of following `main`.
- **Branch:** `halcyon/android`. Patches are small and grouped by area (platform and window, input, launch, packaging, server),
  so they rebase cleanly.
- **Workflow:** `.github/workflows/halcyon-android.yml` checks that the client library builds for `aarch64-linux-android`.
