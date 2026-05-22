# lazypost

A keyboard-driven terminal HTTP client. Three panes — sidebar of saved
requests, request editor, response — laid out like `lazygit`. No mouse, no
config file to hand-write.

## Install

```sh
curl -sSf https://raw.githubusercontent.com/aaron-amelink/lazypost/main/install.sh | sh
```

Or clone and run the script directly:

```sh
git clone https://github.com/aaron-amelink/lazypost
cd lazypost
./install.sh
```

Requires Rust 1.85+ (edition 2024). The script checks your version and prints
upgrade instructions if needed. The binary lands in `~/.cargo/bin/lazypost`.

After install, run `lazypost` from any project directory.

### Local development

```sh
cargo run --release
```

## Quick start

On first launch in a new directory, lazypost prompts you to initialize a
workspace. Press `y` to confirm — this creates `lazypost-workspace.json` with
a set of example requests to get you started.

```
n           open the "new request" modal
e           edit the focused field; Esc to stop editing
[ / ]       cycle sub-tabs (Info / Auth / Body / Params / Headers / Capture)
Enter       (in modal) confirm
s           send the selected request
w           save the editor's changes
?           show full keybind reference
q           quit
```

Pick a request with `j`/`k` in the sidebar, jump to the editor with `2`, fill
in a URL, then send with `s`.

## Files on disk

| File                      | Location                  | Committed? | Contents                                          |
|---------------------------|---------------------------|------------|---------------------------------------------------|
| `lazypost-workspace.json` | current directory         | yes        | Request tree (folders, methods, URLs, bodies)     |
| `env.json`                | `~/lazypost/env/{cwd}/`   | —          | Environment variables (API keys, tokens, etc.)    |
| `history.json`            | current directory         | no         | Last 500 sent requests + responses (capped 256 KB)|

`lazypost-workspace.json` is safe to commit — it contains no secrets. Environment
variables are stored outside the project tree entirely (`~/lazypost/env/` mirroring
your working directory path), so they can never be accidentally committed.

## Environment variables (`E`)

Open the editor with `E`. A flat `key=value` table; the status bar shows the
current variable count.

Reference variables anywhere in URL, headers, params, body, and auth fields:

```
URL:     {{base_url}}/users/{{user_id}}
Header:  Authorization: Bearer {{token}}
Body:    {"id": "{{user_id}}"}
```

Missing or empty variables cause a request error.

```
j / k        navigate rows
e            edit focused field
a / d        add / delete a row
Enter        save and close
Esc          cancel
```

## URL variables

On the **URL Vars** tab, define key/value pairs and reference them in the URL
as `<key>`. Whitespace inside the brackets is allowed (`< key >`). Values are
URL-encoded when inserted. Missing or empty URL vars cause a request error.

## Capture templates

Each request has a **Capture** sub-tab. Write a JSON template using `%name%`
placeholders. After the request runs, the template is matched against the
response body and matched values are written to the env vars table.

Template:
```json
{ "item": { "id": "%item_id%", "slug": "%slug%" } }
```

Response:
```json
{ "item": { "id": 42, "slug": "hello" } }
```

After the response arrives, env vars gain `item_id=42` and `slug=hello`.
Subsequent requests can reference `{{item_id}}` and `{{slug}}`.

Placeholders can also be embedded in strings:
```json
{ "auth": "Bearer %key%" }
```
against `{"auth": "Bearer abc123"}` captures `key=abc123`. Multiple placeholders
per string are supported (`"%user%:%pass%"`).

Rules:
- Whole-string placeholders (`"%x%"`) capture any JSON value at that position.
- Mixed strings (`"Bearer %key%"`) only match string values with the same literal prefix/suffix.
- Capture is skipped silently if the response is not JSON; the status bar reports the outcome.

## History (`H`)

```
j / k     navigate entries (newest first)
Enter     restore request + response into the editor
d         delete the highlighted entry
D         clear all history
Esc / H   close
```

Capped at 500 entries; bodies over 256 KB are truncated for storage.

## Auth types

| Kind    | Sent as                                          |
|---------|--------------------------------------------------|
| None    | nothing                                          |
| Bearer  | `Authorization: Bearer <token>`                  |
| Basic   | `Authorization: Basic base64(user:pass)`         |
| API Key | header `<key>: <value>`, or `?<key>=<value>`     |

API Key location (header vs. query param) is toggled on the Auth sub-tab with `h` / `l`.

## Body types

| Kind      | Sent as                                                         |
|-----------|-----------------------------------------------------------------|
| None      | no body                                                         |
| Raw       | literal text, no Content-Type                                   |
| JSON      | parsed as JSON, `Content-Type: application/json`                |
| Form      | `application/x-www-form-urlencoded`                             |
| Multipart | `multipart/form-data`; rows toggle between text and file path   |

JSON bodies are validated on save. Invalid JSON blocks the save and shows the
parser error in the editor.

## Development

```sh
cargo fmt
cargo clippy --all-targets
cargo test
cargo run
```

Single binary crate. `src/main.rs` wires the ratatui event loop and pane
routing. Modules: `config/` (workspace, env, history), `ui/` (widgets and
modals), `net/` (HTTP client, OAuth), `logic/` (variable substitution,
capture), `model/` (data types). Async HTTP runs on a tokio multi-thread
runtime; responses arrive via `mpsc` so the render loop never blocks.
