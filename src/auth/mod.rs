//! Accounts and sessions: shared types plus the server functions the UI calls.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
pub mod service;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
}

/// Turns a server function error into text for the page.
pub fn error_message(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError(message) => message.clone(),
        _ => "Something went wrong. Please try again.".to_string(),
    }
}

#[cfg(feature = "ssr")]
mod session {
    use leptos::prelude::*;
    use tower_sessions::Session;

    use super::{User, service::AuthError};
    use crate::state::AppState;

    const USER_ID_KEY: &str = "user_id";

    pub fn state() -> AppState {
        expect_context::<AppState>()
    }

    pub async fn current() -> Result<Session, ServerFnError> {
        leptos_axum::extract::<Session>().await.map_err(internal)
    }

    pub async fn user_id(session: &Session) -> Result<Option<i64>, ServerFnError> {
        session.get(USER_ID_KEY).await.map_err(internal)
    }

    /// Starts a signed-in session under a fresh id, so a session id set
    /// before sign-in can't be reused afterwards.
    pub async fn sign_in(session: &Session, user: &User) -> Result<(), ServerFnError> {
        session.cycle_id().await.map_err(internal)?;
        session.insert(USER_ID_KEY, user.id).await.map_err(internal)
    }

    pub fn to_server_error(error: AuthError) -> ServerFnError {
        if matches!(error, AuthError::Database(_) | AuthError::Internal(_)) {
            tracing::error!(?error, "auth failure");
        }
        ServerFnError::new(error)
    }

    pub fn internal(error: impl std::fmt::Debug) -> ServerFnError {
        tracing::error!(?error, "session failure");
        ServerFnError::new("Something went wrong. Please try again.")
    }
}

#[server]
pub async fn current_user() -> Result<Option<User>, ServerFnError> {
    let session = session::current().await?;
    let Some(id) = session::user_id(&session).await? else {
        return Ok(None);
    };
    service::find_user(&session::state().pool, id)
        .await
        .map_err(session::to_server_error)
}

#[server]
pub async fn signups_open() -> Result<bool, ServerFnError> {
    let state = session::state();
    service::signups_open(&state.pool, state.allow_signups)
        .await
        .map_err(session::to_server_error)
}

#[server]
pub async fn sign_up(email: String, password: String) -> Result<(), ServerFnError> {
    let state = session::state();
    let user = service::create_user(&state.pool, state.allow_signups, &email, &password)
        .await
        .map_err(session::to_server_error)?;
    session::sign_in(&session::current().await?, &user).await?;
    leptos_axum::redirect("/");
    Ok(())
}

#[server]
pub async fn sign_in(email: String, password: String) -> Result<(), ServerFnError> {
    let user = service::verify_credentials(&session::state().pool, &email, &password)
        .await
        .map_err(session::to_server_error)?;
    session::sign_in(&session::current().await?, &user).await?;
    leptos_axum::redirect("/");
    Ok(())
}

#[server]
pub async fn sign_out() -> Result<(), ServerFnError> {
    session::current()
        .await?
        .flush()
        .await
        .map_err(session::internal)?;
    leptos_axum::redirect("/");
    Ok(())
}
