use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{A, Route, Router, Routes},
};

use crate::auth::{SignIn, SignOut, SignUp, User, current_user, error_message, signups_open};

type UserResource = Resource<Result<Option<User>, ServerFnError>>;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let sign_in = ServerAction::<SignIn>::new();
    let sign_up = ServerAction::<SignUp>::new();
    let sign_out = ServerAction::<SignOut>::new();

    // Refetch the signed-in user whenever one of the auth actions completes.
    let user: UserResource = Resource::new(
        move || {
            (
                sign_in.version().get(),
                sign_up.version().get(),
                sign_out.version().get(),
            )
        },
        |_| current_user(),
    );

    view! {
        <Stylesheet id="leptos" href="/pkg/cissp-prep.css" />
        <Title text="CISSP Prep" />
        <Router>
            <header class="site-header">
                <A href="/" attr:class="brand">
                    "CISSP Prep"
                </A>
                <nav>
                    <Transition fallback=|| ()>
                        {move || Suspend::new(async move {
                            match user.await {
                                Ok(Some(user)) => {
                                    view! {
                                        <span class="muted">{user.email}</span>
                                        <ActionForm action=sign_out attr:class="inline">
                                            <button type="submit" class="link">
                                                "Sign out"
                                            </button>
                                        </ActionForm>
                                    }
                                        .into_any()
                                }
                                _ => view! { <A href="/login">"Sign in"</A> }.into_any(),
                            }
                        })}
                    </Transition>
                </nav>
            </header>
            <main>
                <Routes fallback=|| view! { <p>"Page not found."</p> }>
                    <Route path=StaticSegment("") view=move || view! { <HomePage user /> } />
                    <Route
                        path=StaticSegment("login")
                        view=move || view! { <SignInPage action=sign_in /> }
                    />
                    <Route
                        path=StaticSegment("signup")
                        view=move || view! { <SignUpPage action=sign_up /> }
                    />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage(user: UserResource) -> impl IntoView {
    view! {
        <h1>"Prepare for the CISSP"</h1>
        <Transition fallback=|| view! { <p class="muted">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match user.await {
                    Ok(Some(user)) => {
                        view! {
                            <p>"Signed in as " <strong>{user.email}</strong> "."</p>
                            <p class="muted">
                                "Practice quizzes and spaced review arrive in the next milestones."
                            </p>
                        }
                            .into_any()
                    }
                    _ => {
                        view! {
                            <p>
                                "Study with practice questions, spaced review and timed mock exams across all eight CISSP domains."
                            </p>
                            <p>
                                <A href="/login">"Sign in"</A>
                                " or "
                                <A href="/signup">"create an account"</A>
                                "."
                            </p>
                        }
                            .into_any()
                    }
                }
            })}
        </Transition>
    }
}

#[component]
fn SignInPage(action: ServerAction<SignIn>) -> impl IntoView {
    view! {
        <Title text="Sign in · CISSP Prep" />
        <h1>"Sign in"</h1>
        <ActionForm action attr:class="stack">
            <label>
                "Email" <input type="email" name="email" autocomplete="email" required />
            </label>
            <label>
                "Password"
                <input type="password" name="password" autocomplete="current-password" required />
            </label>
            <button type="submit" disabled=move || action.pending().get()>
                "Sign in"
            </button>
        </ActionForm>
        <FormError result=action.value() />
        <p class="muted">"No account? " <A href="/signup">"Create one"</A></p>
    }
}

#[component]
fn SignUpPage(action: ServerAction<SignUp>) -> impl IntoView {
    let open = Resource::new(|| (), |_| signups_open());

    view! {
        <Title text="Create account · CISSP Prep" />
        <h1>"Create an account"</h1>
        <Suspense fallback=|| view! { <p class="muted">"Loading…"</p> }>
            {move || Suspend::new(async move {
                if open.await.unwrap_or(false) {
                    view! {
                        <ActionForm action attr:class="stack">
                            <label>
                                "Email"
                                <input type="email" name="email" autocomplete="email" required />
                            </label>
                            <label>
                                "Password (at least 12 characters)"
                                <input
                                    type="password"
                                    name="password"
                                    autocomplete="new-password"
                                    minlength="12"
                                    maxlength="128"
                                    required
                                />
                            </label>
                            <button type="submit" disabled=move || action.pending().get()>
                                "Create account"
                            </button>
                        </ActionForm>
                        <FormError result=action.value() />
                    }
                        .into_any()
                } else {
                    view! { <p>"Sign-ups are closed for now."</p> }.into_any()
                }
            })}
        </Suspense>
        <p class="muted">"Already have an account? " <A href="/login">"Sign in"</A></p>
    }
}

#[component]
fn FormError(#[prop(into)] result: Signal<Option<Result<(), ServerFnError>>>) -> impl IntoView {
    move || {
        result.get().and_then(Result::err).map(|error| {
            view! {
                <p class="error" role="alert">
                    {error_message(&error)}
                </p>
            }
        })
    }
}
