//! Shared server state, available to server functions via `expect_context::<AppState>()`.

use axum::extract::FromRef;
use leptos::config::LeptosOptions;
use sqlx::SqlitePool;

#[derive(Clone, Debug)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: SqlitePool,
    pub allow_signups: bool,
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}
