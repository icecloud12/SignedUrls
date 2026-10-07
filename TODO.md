# Refactor TODO

Ordered by dependency: each step builds on the ones before it.
v1 routes keep their current behavior (legacy); new behavior targets v2.

## 1. Foundation

- [x] **Config struct**: load every env var once at startup (`PREFIX`, `REPLACED_URL`, `ADDRESS`, `PORT`, `DATABASE_URI`, `DATABASE_NAME`, `DEFAULT_DURATION_AS_SECONDS`, cert paths). Fail fast with a clear message if something is missing. Remove all `env::var(...)` calls inside handlers.
- [x] **TLS certs out of the repo**: untracked; a self-signed cert is generated at startup with `rcgen`. Set `TLS_CERT_PATH` + `TLS_KEY_PATH` to serve real cert files instead.
  - [ ] The old key is still in git history, so treat it as leaked: if it's used on any server, replace it there.
- [x] **AppState**: `AppState { db, config }` passed with axum `State`. Remove the global `DATABASE: OnceLock`.
- [x] **Crate layout**: move all modules into `lib.rs`, expose `pub fn app(state: AppState) -> Router`. `main.rs` only loads config, connects, and serves. (Needed so tests can build the app.)

## 2. Safety net

- [x] **Integration test harness**: build the app against a test Mongo database + temp data dir, send requests with `tower::ServiceExt::oneshot`.
- [x] **Characterization tests**: record the current behavior of every route (status codes + response shapes) for v1, legacy aliases, and v2: create bucket, create view/upload request, upload, view, public preview, delete. These keep the next steps from breaking v1 clients.

## 3. Error handling

- [ ] **AppError**: one error enum implementing `IntoResponse` (BadRequest, NotFound, Expired, PayloadTooLarge, Db, Io, ...). Handlers return `Result<impl IntoResponse, AppError>`.
- [ ] **Flatten handlers**: replace nested `match` blocks with `?`. Start with `validate_signed_url_v2` and `create_view_request`.
- [ ] **Remove unwraps on input/IO**: `ObjectId::from_str(...).unwrap()`, `next_field().unwrap()`, `chunk().unwrap()`, `File::create().unwrap()`, etc.
- [ ] **Stop ignoring write errors**: `let _ = file.write_all(...)` in `save_file_to_directory`.
- [ ] **Action enums**: replace hand-written `ToString` / `TryInto` on `ActionTypes` & co. with `Display` / serde `rename_all`.
- [ ] **Enable lints**: add to `Cargo.toml` and fix what's left (`clippy.toml` already allows unwrap/expect in tests):
  ```toml
  [lints.clippy]
  pedantic = { level = "warn", priority = -1 }
  unwrap_used = "warn"
  ```

## 4. Structure

- [ ] **Router**: `Router::nest(prefix, ...)` with `v1_routes()`, `v2_routes()`, `legacy_routes()`. No more `format!` per route.
- [ ] **Feature modules**: reorganize `project/`, `request/`, `signed_url/`, `models/`, `collections/` into features (`bucket/`, `upload/`, `view/`, `file/`, `auth/` or similar), each holding its handlers, logic, and models. Split `signed_url/actions.rs` (685 lines).

## 5. Upload pipeline (v2)

- [ ] **Stream to disk**: write chunks straight to a file instead of buffering the whole file in memory (currently ~2× file size in RAM). Sniff MIME type from the first chunk(s) only.
- [ ] **Size cap**: `size_limit` already exists per file. Add:
  - [ ] a default value when the request doesn't set one
  - [ ] a server maximum in `Config` (`MAX_UPLOAD_BYTES`) that clamps whatever the client asks for
  - [ ] a total-per-request limit (`max_total_size`)
  - [ ] return 413 `PayloadTooLarge` as soon as the running total goes over, while streaming
- [ ] **All-or-nothing uploads**:
  - [ ] stream files into staging (`data/.tmp/<request_id>/`), insert file records with `status: "pending"`
  - [ ] on any failure: delete the staging dir and pending records, return the error
  - [ ] on success: move files to their final location, set `status: "complete"`
  - [ ] mark a consumable request as consumed **only after** a successful upload (currently done during validation, `validate_signed_url_v2`)
- [ ] **Reads ignore incomplete files**: view/preview/delete only see `status: "complete"` (records without `status` count as complete, for existing data).
- [ ] **Sweeper**: background task that deletes `pending` records + staged files older than N minutes (covers crashes where cleanup never ran).

## Open questions

- [ ] v1 upload uses the same `save_files_to_directory`. Should v1 also get all-or-nothing + size caps, or keep a frozen copy of the old path?
- [ ] Sweeper interval / pending TTL values.
