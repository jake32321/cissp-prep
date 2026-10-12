# cissp-prep
Web application for helping users prepare to take the CISSP exam.

Built with [Leptos](https://leptos.dev) on [Axum](https://github.com/tokio-rs/axum), with SQLite for storage.

## Run it locally

You need Rust (stable), the WebAssembly target and [cargo-leptos](https://github.com/leptos-rs/cargo-leptos):

```sh
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --locked
cargo leptos watch
```

Then open http://127.0.0.1:3000. The first account you create becomes the only one until public sign-ups are turned on.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | `sqlite:data/cissp.db` | SQLite database file (created if missing). |
| `ALLOW_SIGNUPS` | `false` | Set to `true` to let anyone create an account. |
| `LEPTOS_SITE_ADDR` | `127.0.0.1:3000` | Address the server listens on. |
| `LEPTOS_ENV` | `DEV` | Set to `PROD` in production so session cookies are HTTPS-only. |

## Checks

```sh
cargo fmt --check
cargo clippy --features ssr --all-targets -- -D warnings
cargo clippy --lib --features hydrate --target wasm32-unknown-unknown -- -D warnings
cargo test --features ssr
```

## Study content

Questions are written from scratch using openly licensed or public-domain sources (NIST, CISA, IETF RFCs, OWASP, MITRE). No ISC2 material is used.
