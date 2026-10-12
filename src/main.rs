#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use axum::Router;
    use cissp_prep::{
        app::{App, shell},
        config::Config,
        db,
        state::AppState,
    };
    use leptos::{config::Env, prelude::*};
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use tower_sessions::{
        ExpiredDeletion, Expiry, SessionManagerLayer,
        cookie::{SameSite, time::Duration},
    };
    use tower_sessions_sqlx_store::SqliteStore;
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let leptos_options = get_configuration(None)?.leptos_options;
    let addr = leptos_options.site_addr;
    let config = Config::from_env();
    let pool = db::connect(&config.database_url).await?;

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await?;
    tokio::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_secs(60 * 60)),
    );
    let sessions = SessionManagerLayer::new(session_store)
        .with_name("cissp_session")
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(leptos_options.env == Env::PROD)
        .with_expiry(Expiry::OnInactivity(Duration::days(30)));

    if config.allow_signups {
        tracing::info!("public sign-ups are open");
    }
    let state = AppState {
        leptos_options: leptos_options.clone(),
        pool,
        allow_signups: config.allow_signups,
    };

    let routes = generate_route_list(App);
    let app = Router::new()
        .leptos_routes(&state, routes, {
            let options = leptos_options.clone();
            move || shell(options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(sessions)
        .with_state(state);

    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // The binary only exists for the server build; the browser bundle
    // starts from `hydrate()` in lib.rs.
}
