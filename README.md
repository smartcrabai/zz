# zzsleep

A CLI sleep/timer utility that waits until a specified duration or time. Installs as the `zz` binary.

## Installation

### Homebrew (macOS / Linux)

```sh
brew install smartcrabai/tap/zzsleep
```

### Shell script (macOS / Linux)

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/smartcrabai/zz/releases/latest/download/zzsleep-installer.sh | sh
```

### PowerShell (Windows)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/smartcrabai/zz/releases/latest/download/zzsleep-installer.ps1 | iex"
```

### Build from source

```sh
cargo install --git https://github.com/smartcrabai/zz
```

This installs the binary as `zz`.

## Usage

```
zz [options] <duration|time>
```

### Options

| Flag | Description |
|------|-------------|
| `-q`, `--quiet` | Suppress the progress bar (useful in scripts) |

### Duration

```sh
zz 10          # wait 10 seconds
zz 2h          # wait 2 hours
zz 5m          # wait 5 minutes
zz 30s         # wait 30 seconds
zz 2h 5m 30s   # wait 2 hours, 5 minutes, and 30 seconds
```

### Wait until a specific time

```sh
zz 12:30             # wait until 12:30 (next occurrence today or tomorrow)
zz 12:30:45          # wait until 12:30:45
zz 20260220T123000+0900   # wait until 2026-02-20 12:30:00 JST (ISO 8601)
zz 20260220T123000Z       # wait until 2026-02-20 12:30:00 UTC (ISO 8601)
```

### Quiet mode

```sh
zz -q 5m          # wait 5 minutes, no progress bar
zz 12:30 --quiet  # wait until 12:30, no progress bar
zz -q 10 && echo "done"   # use in scripts without visual noise
```
