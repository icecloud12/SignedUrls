# Findings from the characterization tests (step 2)

The tests in `tests/` record how the API behaves **today**. Wherever the current
behavior is a bug, the test still asserts it, with a `BUG:` comment. Fixing a bug
means changing that test on purpose.

Run everything with `cargo test` (needs MongoDB on `localhost:27017`, or set
`TEST_DATABASE_URI`).

## Status

- 70 tests (5 unit, 65 integration). The last full run passed; the target tests added afterwards haven't been run yet.
- One **flaky** test (below).
- **Fixed:** the `target` path escape (see "Fixed" at the bottom).

## Open: flaky test

One test in `tests/upload.rs` failed once during a full `cargo test` run and passed
on the next 4 runs of that file. Not identified yet.

- [ ] Run the suite in a loop until it fails, read the `---- name stdout ----` output, fix the cause.

A flaky test can't serve as a safety net: you can't tell a real break from noise.

## Security

### Signatures can be forged 🔴
`hash_parameters` (`src/signed_url/actions.rs:150`) is plain SHA-256 over values the
client sees or picks: bucket id, created, expiration, action, nonce, file id.
There is no server secret in it, so anyone who knows a bucket id can sign their own
URL with any expiry. (`SECRET_KEY` in `.env-template` is never used.)

- Tests: `tests/signature.rs` (`forged_upload_signature_...`, `forged_view_signature_...`)
- Fix idea: HMAC-SHA256 with a server-side secret. That invalidates every URL
  already issued, so plan the rollout (v2 only? accept both for a while?).

## Upload

### Consumable upload URLs can be reused 🟠
- **v2:** the "mark consumed" `update_one` is never `.await`ed, so it never runs
  (`src/signed_url/actions.rs:451`).
  Test: `upload.rs::v2_consumable_url_can_be_reused`.
- **v1:** `validate_signed_url_v1` sets `is_consumed`, but never checks it
  (`src/signed_url/actions.rs:230`).
  Test: `upload.rs::v1_consumable_url_can_be_reused`.
- Also: both consume during *validation*, before the upload succeeds (see TODO step 5).

### A failed size limit leaves earlier files saved 🟠
With several files, if file N is over `size_limit`, files 1..N-1 stay in the
database and on disk, but the client only gets a 400.
Test: `upload.rs::v2_size_limit_rejects_request_but_keeps_earlier_files`. Planned fix: TODO step 5 (all-or-nothing uploads).

### Smaller upload quirks 🟡
- `mime_types` is stored but never enforced (`upload.rs::v2_mime_types_option_is_not_enforced`).
- The file on disk keeps the *original* extension even when content detection says
  otherwise (a PNG named `.txt` is stored as `<id>.txt` with `mime_type: image/png`).
- Parts with the wrong multipart field name (`files` vs `file`) are silently ignored → `200 {"files": []}`.
- `size_limit` is per file, not per request.

## Files without an extension 🟠

Upload saves `README` as `<id>`, but delete and preview look for `<id>.README`:
- **Delete** removes the database record, then fails to remove the file → 400, the file stays on disk
  (`src/signed_url/handlers.rs:121`, `:236`).
  Test: `delete.rs::v2_delete_of_file_without_extension_fails_after_deleting_record`.
- **Preview** returns 404 for a public file (`src/request/view.rs:278`).
  Test: `view.rs::preview_of_public_file_without_extension_is_not_found`.

(`read_file`, used by the signed v1/v2 view URLs, handles it correctly.)

## Panics (connection dropped instead of an error response) 🟡

| Request | Cause |
|---|---|
| `/v2/upload` without `r`, or with an invalid `r` | `query.request.clone().unwrap()` (`src/signed_url/process_upload.rs:104`), `ObjectId::from_str(..).unwrap()` |
| v2 delete with `public_key` but no `secret_key` | `secret_key.unwrap()` (`src/project/actions.rs:163`) |
| v2 view request where no file id is valid | `signatures.get(0).unwrap()` (`src/request/view.rs:139`) |
| v1/v2 view request without a file list | `files.unwrap()` in `create_view_request` |

All are covered by `app.panics(...)` assertions; they become normal errors in step 3 (`AppError`).

## Wrong status codes 🟡

- v1 and v2 **view requests**: any key validation error (unknown key = 400, wrong secret = 401)
  becomes **500** (`src/request/view.rs:101`, `:226`).
- **Preview** with a malformed id returns **500** "Internal server error" (`src/request/view.rs:268`).
- Unknown v2 file ids give **401**, not 404.

## API shape inconsistencies (keep for v1, decide for v2)

- `request_id` is a plain string in upload responses, but `{"$oid": "..."}` in view
  responses (an `ObjectId` serialized straight into JSON).
- v1 responses are wrapped in `{"data": ...}`, v2 responses are not.
- v2 view requests silently drop unknown file ids instead of reporting them.
- A v2 view URL can be created for another bucket's file. The URL doesn't work (wrong
  signature), but creating it succeeds.
- File responses have no `Content-Type` header (the stored `mime_type` isn't used).
- Buckets created through `/v2/bucket` have no `api_key`, so they can't use v1 routes.
- v1 view requests are never consumed, even with `is_consumable: true`.

## Fixed

### `target` could write outside `DATA_DIR` ✅
`target.trim_matches('/').replace("..", "")` turned `"..<absolute path>"` into an
absolute path, and `PathBuf::join` with an absolute path replaces the base, so uploads
could be written anywhere the process can write.

Now handled differently per version (decided 2026-10):
- **v1 (legacy):** unchanged API. The target is still accepted as given and cleaned
  when the file is written: `sanitize_target` keeps only plain folder names.
- **v2:** a target with `..` (or a drive prefix) is **rejected**:
  - when creating the upload URL: `400 {"message": "Invalid target"}`, nothing stored;
  - when writing the file (requests stored before this change):
    `400 {"data": "Request could not be completed"}`, nothing written.
  - Leading/trailing slashes are still fine (`"/images/2024/"`).

Tests (written, **not run yet**): `actions.rs::is_safe_target_rejects_only_traversal`,
`actions.rs::sanitize_target_keeps_only_folder_names`,
`upload_request.rs::v2_rejects_unsafe_target`, `v2_accepts_slashes_around_target`,
`v1_accepts_unsafe_target_and_stores_it_as_given`,
`upload.rs::v1_unsafe_target_is_cleaned_and_stays_inside_data_dir`,
`v2_upload_with_stored_unsafe_target_is_rejected`.
