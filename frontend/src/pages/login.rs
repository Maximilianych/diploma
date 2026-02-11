use leptos::*;
use crate::api;
use crate::models::User;

#[component]
pub fn LoginPage(on_login: WriteSignal<Option<User>>) -> impl IntoView {
    let (email, set_email) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (loading, set_loading) = create_signal(false);

    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        let email_val = email.get();
        let password_val = password.get();

        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match api::login(email_val, password_val).await {
                Ok(auth) => {
                    on_login.set(Some(auth.user));
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }

            set_loading.set(false);
        });
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-100">
            <div class="bg-white p-8 rounded-lg shadow-md w-full max-w-md">
                <h1 class="text-2xl font-bold mb-6 text-center">"Task Tracker"</h1>

                <form on:submit=submit class="space-y-4">
                    {move || error.get().map(|e| view! {
                        <div class="bg-red-100 text-red-700 p-3 rounded">{e}</div>
                    })}

                    <div>
                        <label class="block text-sm font-medium mb-1">"Email"</label>
                        <input
                            type="email"
                            class="w-full border rounded px-3 py-2"
                            prop:value=email
                            on:input=move |ev| set_email.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium mb-1">"Password"</label>
                        <input
                            type="password"
                            class="w-full border rounded px-3 py-2"
                            prop:value=password
                            on:input=move |ev| set_password.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <button
                        type="submit"
                        class="w-full bg-blue-600 text-white py-2 rounded hover:bg-blue-700 disabled:opacity-50"
                        disabled=loading
                    >
                        {move || if loading.get() { "Loading..." } else { "Login" }}
                    </button>
                </form>
            </div>
        </div>
    }
}