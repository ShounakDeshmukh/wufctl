# wufctl

A terminal UI for [NCSU's VCL](https://vcl.ncsu.edu) (Virtual Computing Lab), for people who'd
rather manage a reservation from a terminal than click through the web dashboard every time.

This is an unofficial, personal project. It's not built or endorsed by NCSU, and it talks to the
same XML-RPC API the web UI uses, over your own account token.

## What it does

- Browse available images and start a reservation without leaving the terminal.
- Extend or end a reservation
- Connect once it's ready: either an SSH handoff that suspends the TUI and hands your terminal
  straight to a real `ssh` process, or an xRDP `.rdp` file saved to your Downloads folder for
  images that support it.

It intentionally doesn't do everything the web UI does. Windows/AVD images aren't reservable
through this tool

## Setup

You'll need a VCL API token:

1. Log in to <https://vcl.ncsu.edu>
2. Manage -> User Preferences -> Manage Tokens
3. Generate a new token

The first time you run `wufctl` it'll ask for that token and validate it against the server
before saving it. It's saved to the OS's own secure credential store - Keychain on macOS,
Credential Manager on Windows, the Secret Service (GNOME Keyring/KWallet) on Linux - so it's
encrypted at rest rather than sitting in a plaintext file. If no secret store is available
(e.g. a headless Linux box with no Secret Service running), it falls back to a plain file at
`~/.config/wufctl/config.toml` on Linux, `~/Library/Application Support/wufctl/config.toml` on
macOS, or `%APPDATA%\wufctl\config.toml` on Windows, written `0600` on Unix so nothing else on
the machine can read it.

## Installing

Prebuilt binaries for Linux, macOS, and Windows are on the
[Releases page](https://github.com/ShounakDeshmukh/wufctl/releases).

macOS/Linux:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ShounakDeshmukh/wufctl/releases/latest/download/wufctl-installer.sh | sh
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ShounakDeshmukh/wufctl/releases/latest/download/wufctl-installer.ps1 | iex"
```

## Building from source

```sh
cargo build --release -p wufctl
cargo run -p wufctl
```

Requires a stable Rust toolchain. Nothing else needs to be installed to browse images or manage
reservations. Connecting needs whatever client the method requires - `ssh` on your `PATH` for the
SSH handoff (present by default on Linux/macOS, an optional Windows feature that's on by default
on most modern installs), or an RDP client of your choice to open the saved `.rdp` file.

## Layout

The workspace has two crates:

- `vcl_lib` - a thin XML-RPC client for the VCL API. It returns the raw response type and does no
  interpretation of its own; deserialization into typed structs lives in the consumer.
- `wufctl` - this TUI, built on `ratatui`.