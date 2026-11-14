Published `sqlx-sqlite` 0.9.0 with one change: `libsqlite3-sys` upper bound
`<0.38.0` → `<0.40.0`.

That is the unpublished bound from launchbadge/sqlx@10462fa (PR #4502).
sqlx 0.9.0 is still the latest crates.io release and refuses 0.39, so the
lockfile cannot reach the RUSTSEC-2026-0459 fix (bundled SQLite 3.54.x in
`libsqlite3-sys` 0.39.1) without this copy.

Delete this directory and the `[patch.crates-io]` entry when a sqlx release
allows `libsqlite3-sys` 0.39.
