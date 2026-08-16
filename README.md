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
before saving it locally - `~/.config/wufctl/config.toml` on Linux, `~/Library/Application
Support/wufctl/config.toml` on macOS, `%APPDATA%\wufctl\config.toml` on Windows, resolved via
the OS's own config directory convention rather than hardcoded. The file is written `0600` on
Unix so nothing else on the machine can read it.

## Building and running

```sh
cargo build --release -p wufctl
cargo run -p wufctl
```

Requires a stable Rust toolchain. Nothing else needs to be installed to browse images or manage
reservations. Connecting needs whatever client the method requires - `ssh` on your `PATH` for the
SSH handoff (present by default on Linux/macOS, an optional Windows feature that's on by default
on most modern installs), or an RDP client of your choice to open the saved `.rdp` file.

## Using it

Reservations is the home screen - everything else is a popup over it.

| Key | Where | Does |
|---|---|---|
| `n` | Reservations | Open the image picker to start a new reservation |
| `r` | Reservations | Refresh (debounced to once per 20s) |
| `t` | Reservations | Change the saved API token |
| `q` | Reservations | Quit |
| `c` | Reservations, on a `ready` row | Open the Connect popup |
| `e` | Reservations, on a `ready` row | Extend the reservation |
| `x` | Reservations, any row but `loading` | End the reservation (asks for confirmation) |
| `/` | Image picker | Search images by name |
| `Enter` | Connect popup | Start the SSH session |
| `r` | Connect popup, if xRDP is offered | Save a `.rdp` file to Downloads |
| `Up`/`Down`, `Left`/`Right` | Forms | Move between fields, cycle a field's value |
| `Esc` | Any popup | Back out |

## Layout

The workspace has three crates:

- `vcl_lib` - a thin XML-RPC client for the VCL API. It returns the raw response type and does no
  interpretation of its own; deserialization into typed structs lives in the consumer.
- `wufctl` - this TUI, built on `ratatui`.