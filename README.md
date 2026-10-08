# Android Deck

A lightweight desktop app to manage wireless Android device connections: pair
(`adb pair`), connect (`adb connect`) and launch **scrcpy** with a single click —
no more typing commands or opening the scrcpy folder manually in a terminal.

> Platform: **Windows**. Built with **Tauri v2** (Rust + WebView2), so the
> binary is small and starts fast.

## Features

- Save **device presets** (name, IP, connect port, pair port, scrcpy arguments).
- **One-click pair & connect**: enter the 6-digit code once and the app runs
  `adb pair` then `adb connect` for you.
- **Launch scrcpy** per device with dedicated arguments (or the defaults).
- **Monitor connected devices** with a live view (auto-refreshes every 3s).
- Configure the **adb / scrcpy paths** and check the tools from within the app.

## Requirements

- Windows 10/11 (WebView2 Runtime is included by default on Windows 10/11).
- **Rust** (stable, MSVC toolchain): https://rustup.rs
- **Node.js** 18+ and npm.
- **Android Platform Tools** (`adb`) and **scrcpy**.
  - Download scrcpy: https://github.com/Genymobile/scrcpy/releases

## Setup & run (dev)

```powershell
npm install
npm run tauri dev
```

## Build

```powershell
npm run tauri build
```

Installers (`.msi` / `.exe`) are written to
`src-tauri\target\release\bundle\`.

On a fresh machine, Rust requires **Visual Studio Build Tools** with the
"Desktop development with C++" workload:

```powershell
rustup default stable-msvc
```

## Usage

1. On the phone, enable **Developer options → Wireless debugging**.
2. Open the app → **Settings** tab, set the `adb` path (or leave it as `adb`)
   and the `scrcpy.exe` path, then click **Check tools**.
3. **Devices → Add device**: enter a name, the connect IP + port and (when
   needed) the pair IP + port with the **6-digit code** shown on the phone.
4. Click **Save** — the app pairs and connects automatically.
5. Click **Open scrcpy** to mirror and control the phone screen.

## Project structure

```
android-deck/
├─ src/                     # Frontend (plain HTML/CSS/JS)
│  ├─ index.html
│  ├─ styles.css
│  └─ main.js
├─ src-tauri/               # Rust backend
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ lib.rs             # registers the Tauri commands
│  │  ├─ commands.rs        # commands exposed to the frontend
│  │  ├─ adb.rs             # adb command wrapper
│  │  ├─ scrcpy.rs          # scrcpy launcher
│  │  ├─ config.rs          # read/write the JSON config
│  │  └─ models.rs          # data structs
│  ├─ capabilities/
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ package.json
└─ README.md
```

The configuration is stored at `%APPDATA%\com.androideck.desktop\config.json`.

## Download

Windows installers (`.msi` / `.exe`) are built automatically and attached to
[**Releases**](../../releases) whenever a version tag is pushed.
Download the files from the **Assets** section of the latest release.

### Creating a release

```powershell
# 1. Bump the version in package.json, src-tauri/Cargo.toml and src-tauri/tauri.conf.json
# 2. Create and push a tag (the workflow triggers on tags starting with "v")
git tag v<X.Y.Z>
git push origin v<X.Y.Z>
```

GitHub Actions ([`.github/workflows/release.yml`](.github/workflows/release.yml))
builds on Windows and creates a **draft release** with the installers attached.
Go to Releases → edit the notes (if needed) → **Publish**.

## License

[MIT](LICENSE)
