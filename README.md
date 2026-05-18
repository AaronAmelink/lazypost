# lazypost

A keyboard-driven terminal HTTP client. Three panes — sidebar of saved
requests, request editor, response — laid out like `lazygit`. No mouse, no
config file to hand-write.

## Install

Requires Rust 1.85+ (uses edition 2024).

```sh
cargo install --path .
```

This drops the `lazypost` binary into `~/.cargo/bin/`. Make sure that directory
is on your `PATH` (rustup adds it by default; if not, add it to your shell rc).
After install, run `lazypost` from any directory.

Because lazypost reads and writes its state files **in the current working
directory** (not next to the binary), each project gets its own isolated
workspace.

### For local Development

```sh
cargo run --release
```

Runs against `workspace.json` / `env.json` / `history.json` in the repo root.

## Quick start

```
n           open the "new request" modal
e           edit the focused field; Esc to stop editing
[ / ]       cycle sub-tabs (Info / Auth / Body / Params / Headers / Capture)
Enter       (in modal) save the new request
s           send the selected request
w           save the editor's changes
?           show full keybind reference
q           quit
```

A workspace ships with example requests on first launch. Pick one with `j`/`k`
in the sidebar, jump to the editor with `2`, then send with `s` (or from the
sidebar — `s` works there too).

## Files on disk

Lazypost reads and writes three files in the **current working directory** —
wherever you launched it from, not next to the binary. This is what makes
per-directory workspaces possible.

| File             | Committed? | Contents                                            |
|------------------|------------|-----------------------------------------------------|
| `workspace.json` | yes        | The request tree (folders, methods, URLs, bodies)   |
| `env.json`       | **no**     | Environment variables — frequently hold secrets     |
| `history.json`   | **no**     | Last 500 sent requests + responses (capped 256KB)   |


## Environment variables (`E`)

Open with `E`. A single flat `key=value` table, stored in `env.json`. The
status bar shows the variable count.

Reference variables anywhere in URL / headers / params / body / auth fields:

```
URL:     {{base_url}}/users/{{user_id}}
Header:  Authorization: Bearer {{token}}
Body:    {"id": "{{user_id}}"}
```

Missing variables expand to the empty string — they don't error.

```
j / k            navigate fields
e                edit focused field
a / d            add / delete a variable row
Enter            save and close
Esc              cancel
```

## Capture templates ("predicted response")

Each request has a **Capture** sub-tab. Put a JSON template using `%name%`
placeholders. After the request runs, the template is walked in parallel with
the actual response body; matched values are written to the env vars list
(visible when you press `E`).

Template:

```json
{ "item": { "id": "%item_id%", "slug": "%slug%" } }
```

Response:

```json
{ "item": { "id": 42, "slug": "hello" } }
```

After the response arrives, the env vars gain `item_id=42` and `slug=hello`.
Subsequent requests can reference `{{item_id}}` and `{{slug}}`.

Placeholders can also be embedded in literal strings:

```json
{ "auth": "Bearer %key%" }
```

against an actual response of `{"auth": "Bearer abc123"}` captures `key=abc123`.
Multiple placeholders per string are supported (`"%user%:%pass%"`).

Rules:
- Whole-string placeholders (`"%x%"`) capture any JSON value at that position.
- Mixed strings (`"Bearer %key%"`) only match when the actual value is a
  string with the same literal prefix/suffix.
- Capture is skipped silently if the response isn't JSON. The status bar
  reports what happened (`captured: item_id, slug` or `capture: nothing matched`).

## History (`H`)

```
j / k     navigate entries (newest first)
Enter     restore the request + response into the editor pane
d         delete the highlighted entry
D         clear all history
Esc / H   close
```

Capped at 500 entries; bodies over 256 KB are truncated for storage.

## Auth types

| Kind     | Sent as                                              |
|----------|------------------------------------------------------|
| None     | nothing                                              |
| Bearer   | `Authorization: Bearer <token>`                      |
| Basic    | `Authorization: Basic base64(user:pass)`             |
| API Key  | header `<key>: <value>`, or `?<key>=<value>`         |

API Key location is toggled on the editor's Auth sub-tab with `e` / `h` / `l`.

## Body types

| Kind       | Sent as                                                            |
|------------|--------------------------------------------------------------------|
| None       | no body                                                            |
| Raw        | the literal text, no Content-Type set                              |
| JSON       | the text parsed as JSON, `Content-Type: application/json`          |
| Form       | `application/x-www-form-urlencoded`                                |
| Multipart  | `multipart/form-data`; rows toggle between text and a file path    |

JSON bodies are validated on save. Invalid JSON blocks the save and shows the
parser error at the bottom of the editor.

## Development

```sh
cargo fmt
cargo clippy --all-targets   # currently lint-clean
cargo test                   # capture template + env substitution unit tests
cargo run                    # dev build, attached to your terminal
```

The codebase is a single binary crate: `main.rs` wires the ratatui loop and
event routing; everything else lives under `src/helpers/`. Async HTTP runs on
a multi-thread tokio runtime; responses come back to the UI thread through
`tokio::sync::mpsc`, so the render loop never blocks.

## What's intentionally missing

- OAuth2 (model exists, no UI yet)
- Request import/export (Postman/Insomnia collections)
- Cookies / sessions
- Mouse support — keyboard-only by design
