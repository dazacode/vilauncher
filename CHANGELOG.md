# Changelog

The release notes for each tag are taken from the section for that version.

## 1.0.5

### New
- **Controller keys (keyboard as controller):** the 24 controls VIctory reads from the keyboard can each get another key, set in Settings. Off by default. A low-level keyboard hook sends the layer's own key only while `victory.exe` is in front. Windows only.
- **Add-ons:** optional features for forks, described by a JSON manifest. The memory stack ships as the first one. Turning one on puts its patches on a `vilauncher/<id>` branch of the project, and turning it off restores the branch and the player's own settings.
- **Linux downloads:** AppImage and .deb are attached to each release, and the launcher runs natively on Linux.
- **Clearer messages:** a repo with no Rust source says so; a game file that is not a Windows program is caught before Play; an unhandled game exception points at the extra switches; shader model 6.6 refusals point at the Agility SDK.

### Fixed
- Install and update fetch a project's git submodules (AnyXboxX's `third_party/tombox`).
- Winget's "already installed" result no longer stops the tool install, and a failed winget step is retried once after a source reset. A manual install page is named when it still fails.
- Listed projects whose programs are not called `victory` (AnyXboxX) install by workspace instead of `cargo build -p victory`.
- The shader cache is no longer deleted after every run (a benign log line was read as a failure).
- Agility SDK files are accepted when the project names both the package hash and the file hash.
- The default output resolution is kept when saved.
- On Linux, the AppImage's own libraries no longer leak into git and cargo, and the CPU and GPU are detected.
- The updater and the Source link point at `dazacode/vilauncher` again.

### Thanks
- **@karrot0**: running the launcher on Linux (#9).
- **@Louisreed**: the Agility hash fix (#11) and the shader cache fix (#15).
- **@arno721**: Traditional and Simplified Chinese (#18), and the updater link fix (#27).
- **@TheInfamous404**: the saved default output resolution (#18).
- **Estevan (PalmVI)**: the shout-out, and for sending players our way.
- **Reports and debugging:** @Enrico-Signore (Linux support, #1), @VertofChest (the Agility export check, #3), @berat-c (AnyXboxX submodules, #5), @106399353 and @pthrdc0re (winget, #6 and #8), @Anxwiety (the Agility hash, #12), @miaohaixiang1-netizen (listing request, #7), @WolfIsTough (AnyXboxX install, #22), and @MujoraCS (the malformed-image report).

### Known limits
- The games are still Windows-only; the Linux build runs the launcher.
- Controller keys need a real game session to confirm; the translation logic is tested, the live hook is not yet.
- Add-ons only apply where the patches fit the project's code; otherwise the launcher says so and changes nothing.
