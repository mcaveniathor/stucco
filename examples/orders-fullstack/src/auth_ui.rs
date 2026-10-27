//! Authentication state belongs to the application root, above routed editors.
use crate::{
    Drafts, PendingDeletes, api,
    auth::{AccessError, LoginError, LoginState, PrincipalKey, SessionView},
};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate, use_query_map};
use stucco_leptos::{ErrorSummary, StatusMessage, SubmitButton};

// The server emits this codec value only for a response tagged by Stucco's
// CSRF layer. An arbitrary HTTP 403 or unreadable proxy response is not proof
// that a mutation was rejected before execution.
pub(crate) fn csrf_rejected(error: &ServerFnError) -> bool {
    matches!(error, ServerFnError::MiddlewareError(code) if code == "stucco.csrf.rejected")
}

#[derive(Clone, Copy)]
pub(crate) struct Auth {
    pub session: RwSignal<SessionView>,
    pub epoch: RwSignal<u64>,
    pub busy: RwSignal<bool>,
    quarantined: RwSignal<Option<PrincipalKey>>,
    drafts: Drafts,
    deletes: PendingDeletes,
}

impl Auth {
    pub fn new(session: SessionView, drafts: Drafts) -> Self {
        Self {
            session: RwSignal::new(session),
            epoch: RwSignal::new(0),
            busy: RwSignal::new(false),
            quarantined: RwSignal::new(None),
            drafts,
            deletes: expect_context::<PendingDeletes>(),
        }
    }

    pub fn key(self) -> Option<PrincipalKey> {
        self.session
            .get_untracked()
            .principal
            .map(|principal| principal.key)
    }

    pub fn apply(self, session: SessionView) {
        let before = self.key();
        let after = session.principal.as_ref().map(|principal| principal.key);
        if before != after {
            if after.is_none() {
                self.quarantined.set(before);
            } else {
                // Same-identity reauthentication may recover its quarantined
                // draft. A different account must never receive that state.
                self.drafts
                    .0
                    .update(|drafts| drafts.retain(|(owner, _), _| Some(owner) == after.as_ref()));
                self.deletes
                    .0
                    .update(|drafts| drafts.retain(|(owner, _), _| Some(owner) == after.as_ref()));
                self.quarantined.set(None);
            }
            self.epoch.update(|epoch| *epoch = epoch.wrapping_add(1));
        }
        self.session.set(session);
    }

    pub fn clear(self) {
        self.drafts.0.update(|drafts| drafts.clear());
        self.deletes.0.update(|drafts| drafts.clear());
        self.quarantined.set(None);
        self.epoch.update(|epoch| *epoch = epoch.wrapping_add(1));
        self.session.update(|session| session.principal = None);
    }

    pub async fn refresh(self, epoch: u64) {
        if self.busy.get_untracked() {
            return;
        }
        if let Ok(session) = api::session().await
            && self.epoch.try_get_untracked() == Some(epoch)
            && !self.busy.get_untracked()
        {
            self.apply(session);
        }
    }
}

/// Captured by each protected route. Late completions cannot cross identity
/// changes, including logout followed by a fresh login to the same account.
#[derive(Clone, Copy)]
pub(crate) struct AuthScope {
    pub auth: Auth,
    pub key: PrincipalKey,
    epoch: u64,
}

impl AuthScope {
    pub fn new(auth: Auth, key: PrincipalKey) -> Self {
        Self {
            auth,
            key,
            epoch: auth.epoch.get_untracked(),
        }
    }

    pub fn current(self) -> bool {
        self.auth.epoch.try_get_untracked() == Some(self.epoch)
            && self
                .auth
                .session
                .try_get_untracked()
                .and_then(|session| session.principal)
                .is_some_and(|principal| principal.key == self.key)
    }

    pub fn reject(self) {
        if !self.current() {
            return;
        }
        self.auth.apply(SessionView {
            principal: None,
            csrf: self.auth.session.get_untracked().csrf,
        });
        let auth = self.auth;
        let epoch = auth.epoch.get_untracked();
        leptos::task::spawn_local(async move {
            auth.refresh(epoch).await;
        });
    }

    pub async fn check_session(self) {
        if self.current() {
            self.auth.refresh(self.epoch).await;
        }
    }
}

#[component]
pub(crate) fn AccessRequired(reason: AccessError) -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    Effect::new(move |_| scope.reject());
    let message = match reason {
        AccessError::Unauthenticated => {
            "Your session ended. Sign in again to continue. Your draft is retained for this account."
        }
        AccessError::IdentityChanged => {
            "The signed-in account changed. Sign in to continue with the correct account."
        }
    };
    view! { <p role="alert">{message}</p><a href="/login">"Sign in"</a> }
}

#[component]
pub(crate) fn LoginPage() -> impl IntoView {
    let auth = expect_context::<Auth>();
    let location = use_location();
    let query = use_query_map();
    let initial = if auth.epoch.get_untracked() == 0 {
        use_context::<LoginState>().unwrap_or_default()
    } else {
        LoginState::default()
    };
    let native_destination = StoredValue::new(initial.return_to.clone());
    let return_to = move || {
        let explicit = query.get_untracked().get("return_to").or_else(|| {
            let destination = native_destination.get_value();
            (!destination.is_empty()).then_some(destination)
        });
        explicit
            .filter(|path| protected_destination(path))
            .unwrap_or_else(|| {
                let path = location.pathname.get_untracked();
                if protected_destination(&path) {
                    {
                        let query = location.search.get_untracked();
                        if query.is_empty() {
                            path
                        } else {
                            format!("{path}?{query}")
                        }
                    }
                } else {
                    "/orders".into()
                }
            })
    };
    let username = RwSignal::new(initial.username);
    let password = RwSignal::new(String::new());
    let failure = RwSignal::new(initial.error);
    let notice = RwSignal::new(String::new());
    let focus_trigger = RwSignal::new(u32::from(failure.get_untracked().is_some()));
    let password_node = NodeRef::<leptos::html::Input>::new();
    let username_node = NodeRef::<leptos::html::Input>::new();
    let heading = NodeRef::<leptos::html::H1>::new();
    let hydrated = RwSignal::new(false);
    let navigate = use_navigate();
    let login = Action::new(move |credentials: &(String, String)| {
        let (username, password) = credentials.clone();
        let csrf = auth.session.get_untracked().csrf;
        auth.busy.set(true);
        let epoch = auth.epoch.get_untracked();
        async move { (epoch, api::login(username, password, csrf).await) }
    });
    let pending = login.pending();
    Effect::new(move |_| {
        hydrated.set(true);
        #[cfg(feature = "hydrate")]
        {
            if focus_trigger.get_untracked() == 0
                && let Some(heading) = heading.get()
            {
                crate::focus_if_unclaimed(&heading);
            }
        }
    });
    #[cfg(feature = "hydrate")]
    {
        let initialized = std::cell::Cell::new(false);
        Effect::new(move |_| {
            let values = (username.get(), password.get());
            if let (Some(user), Some(pass)) = (username_node.get(), password_node.get()) {
                if !initialized.replace(true) {
                    username.set(user.value());
                    password.set(pass.value());
                } else {
                    if user.value() != values.0 {
                        user.set_value(&values.0);
                    }
                    if pass.value() != values.1 {
                        pass.set_value(&values.1);
                    }
                }
            }
        });
    }
    Effect::new(move |_| {
        if let Some((epoch, result)) = login.value().get() {
            login.input().set(None);
            auth.busy.set(false);
            if auth.epoch.get_untracked() != epoch {
                return;
            }
            password.set(String::new());
            match result {
                Ok(Ok(session)) => {
                    let destination = return_to();
                    auth.apply(session);
                    navigate(&destination, Default::default());
                }
                Ok(Err(error)) => {
                    failure.set(Some(error));
                    focus_trigger.update(|n| *n = n.saturating_add(1));
                }
                Err(_) => {
                    notice.set(
                        "Sign-in could not be confirmed. Check your connection and try again."
                            .into(),
                    );
                    let epoch = auth.epoch.get_untracked();
                    leptos::task::spawn_local(async move {
                        auth.refresh(epoch).await;
                    });
                }
            }
        }
    });
    let message = Signal::derive(move || match failure.get() {
        Some(LoginError::InvalidCredentials) => "The username or password is incorrect.".to_owned(),
        Some(LoginError::Unavailable) => {
            "Sign-in is temporarily unavailable. Please try again.".to_owned()
        }
        None => String::new(),
    });
    view! {
        <leptos_meta::Title text="Sign in · Stucco"/>
        <section class="order-card" data-hydrated=move || hydrated.get().to_string()>
            <h1 node_ref=heading tabindex="-1">"Sign in"</h1>
            <p>{move || if auth.quarantined.get().is_some() { "Your session ended. Sign in to the same account to recover your unsaved draft." } else { "Sign in to manage your team’s orders." }}</p>
            <form action="/login" method="post" on:submit=move |event| {
                event.prevent_default();
                if !pending.get_untracked() && !auth.busy.get_untracked() {
                    failure.set(None); notice.set(String::new());
                    login.dispatch((username.get_untracked(), password.get_untracked()));
                }
            }>
                <input type="hidden" name="_csrf" value=move || auth.session.get().csrf/>
                <input type="hidden" name="return_to" value=return_to()/>
                <ErrorSummary id="login-errors" message focus_trigger/>
                <div class="st-field">
                    <label class="st-label" for="username">"Username"</label>
                    <input node_ref=username_node class="st-input" id="username" name="username" type="text" autocomplete="username" required
                        value=username.get_untracked() readonly=move || pending.get() || auth.busy.get()
                        on:input=move |event| username.set(event_target_value(&event))/>
                </div>
                <div class="st-field">
                    <label class="st-label" for="password">"Password"</label>
                    <input node_ref=password_node class="st-input" id="password" name="password" type="password" autocomplete="current-password" required
                        readonly=move || pending.get() || auth.busy.get()
                        on:input=move |event| password.set(event_target_value(&event))/>
                </div>
                <SubmitButton label="Sign in" pending blocked=auth.busy pending_label="Signing in…"/>
                <StatusMessage message=notice/>
            </form>
        </section>
    }
}

#[component]
pub(crate) fn AccountMenu() -> impl IntoView {
    let auth = expect_context::<Auth>();
    let notice = RwSignal::new(String::new());
    let logout = Action::new(move |_: &()| {
        let csrf = auth.session.get_untracked().csrf;
        // Remove protected DOM and drafts before waiting for the response.
        auth.busy.set(true);
        auth.clear();
        let epoch = auth.epoch.get_untracked();
        async move { (epoch, api::logout(csrf).await) }
    });
    let pending = logout.pending();
    Effect::new(move |_| {
        if let Some((epoch, result)) = logout.value().get() {
            auth.busy.set(false);
            if auth.epoch.get_untracked() != epoch {
                return;
            }
            match result {
                Ok(session) => auth.apply(session),
                Err(_) => {
                    notice.set("Sign-out could not be confirmed. Try signing out again before leaving this browser.".into());
                    leptos::task::spawn_local(async move {
                        if let Ok(session) = api::session().await
                            && auth.epoch.try_get_untracked() == Some(epoch)
                        {
                            auth.session.update(|current| current.csrf = session.csrf);
                        }
                    });
                }
            }
        }
    });
    view! {
        <div class="account-menu">
            <span>{move || auth.session.get().principal.map(|principal| principal.display_name).unwrap_or_default()}</span>
            <Show when=move || auth.session.get().principal.is_some() || pending.get() || !notice.get().is_empty()>
            <form action="/logout" method="post" on:submit=move |event| {
                event.prevent_default();
                if !pending.get_untracked() && !auth.busy.get_untracked() { notice.set(String::new()); logout.dispatch(()); }
            }>
                <input type="hidden" name="_csrf" value=move || auth.session.get().csrf/>
                <SubmitButton label="Sign out" pending blocked=auth.busy pending_label="Signing out…"/>
            </form>
            </Show>
            <StatusMessage message=notice/>
        </div>
    }
}

// Only application paths may survive sign-in; never accept a network path.
fn protected_destination(path: &str) -> bool {
    path.len() <= 2048
        && !path.chars().any(|c| c.is_control() || c == '\\')
        && ["/orders", "/exports", "/components"].iter().any(|prefix| {
            path == *prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('?'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        PendingDelete,
        api::{DeleteState, EditorState},
        auth::Principal,
    };
    use std::collections::BTreeMap;

    fn session(key: Option<PrincipalKey>, csrf: &str) -> SessionView {
        SessionView {
            principal: key.map(|key| Principal {
                key,
                display_name: "Account".into(),
            }),
            csrf: csrf.into(),
        }
    }

    fn fixture() -> (Owner, Auth, Drafts, PendingDeletes, PrincipalKey) {
        let owner = Owner::new();
        let (auth, drafts, deletes, key) = owner.with(|| {
            let key = PrincipalKey {
                user_id: 1,
                tenant_id: 1,
                generation: 1,
            };
            let drafts = Drafts(RwSignal::new(BTreeMap::new()));
            let deletes = PendingDeletes(RwSignal::new(BTreeMap::new()));
            provide_context(deletes);
            let order = crate::domain::new_order();
            let mut draft = order.draft();
            draft.customer = "Private unsaved customer".into();
            draft.quantity_one = "not a number".into();
            drafts.0.update(|drafts| {
                drafts.insert(
                    (key, 0),
                    EditorState {
                        order: order.clone(),
                        draft,
                        error: None,
                        csrf: "old".into(),
                        uncertain: true,
                    },
                );
            });
            deletes.0.update(|deletes| {
                deletes.insert(
                    (key, 7),
                    PendingDelete {
                        state: DeleteState {
                            order,
                            expected_revision: "3".into(),
                            error: None,
                            csrf: "old".into(),
                        },
                        uncertain: true,
                    },
                );
            });
            (
                Auth::new(session(Some(key), "old"), drafts),
                drafts,
                deletes,
                key,
            )
        });
        (owner, auth, drafts, deletes, key)
    }

    #[test]
    fn expiry_reauthentication_retains_only_same_identity_raw_state() {
        let (_owner, auth, drafts, deletes, key) = fixture();
        let original = AuthScope::new(auth, key);
        auth.apply(session(None, "expired"));
        assert!(!original.current());
        assert_eq!(auth.quarantined.get_untracked(), Some(key));
        auth.apply(session(Some(key), "fresh"));
        assert!(
            !original.current(),
            "old responses stay invalid after reauthentication"
        );
        assert_eq!(
            drafts.0.get_untracked()[&(key, 0)].draft.quantity_one,
            "not a number"
        );
        assert_eq!(
            deletes.0.get_untracked()[&(key, 7)].state.expected_revision,
            "3"
        );
        assert_eq!(auth.session.get_untracked().csrf, "fresh");
    }

    #[test]
    fn changing_user_tenant_or_database_generation_discards_editor_and_delete_state() {
        for next in [
            PrincipalKey {
                user_id: 2,
                tenant_id: 1,
                generation: 1,
            },
            PrincipalKey {
                user_id: 1,
                tenant_id: 2,
                generation: 1,
            },
            PrincipalKey {
                user_id: 1,
                tenant_id: 1,
                generation: 2,
            },
        ] {
            let (_owner, auth, drafts, deletes, key) = fixture();
            let old = AuthScope::new(auth, key);
            auth.apply(session(Some(next), "fresh"));
            assert!(!old.current());
            assert!(drafts.0.get_untracked().is_empty());
            assert!(deletes.0.get_untracked().is_empty());
        }
    }

    #[test]
    fn explicit_logout_discards_state_even_when_same_account_returns() {
        let (_owner, auth, drafts, deletes, key) = fixture();
        let old = AuthScope::new(auth, key);
        auth.clear();
        auth.apply(session(Some(key), "fresh"));
        assert!(!old.current());
        assert!(drafts.0.get_untracked().is_empty());
        assert!(deletes.0.get_untracked().is_empty());
    }

    #[test]
    fn same_identity_token_rotation_keeps_live_scope_and_drafts() {
        let (_owner, auth, drafts, _, key) = fixture();
        let scope = AuthScope::new(auth, key);
        auth.apply(session(Some(key), "rotated"));
        assert!(scope.current());
        assert_eq!(drafts.0.get_untracked().len(), 1);
        assert_eq!(auth.session.get_untracked().csrf, "rotated");
    }
}
