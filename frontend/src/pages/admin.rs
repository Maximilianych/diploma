use leptos::*;
use crate::api;
use crate::models::{User, CreateUserRequest, MlStatusResponse, MlRetrainResponse};

#[component]
pub fn AdminPage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <h2 class="text-lg font-semibold">"Admin Panel"</h2>
            <UsersSection />
            <MlSection />
            <ArchiveSection />
        </div>
    }
}

// ============ Users ============

#[component]
fn UsersSection() -> impl IntoView {
    let (users, set_users) = create_signal(Vec::<User>::new());
    let (show_form, set_show_form) = create_signal(false);
    let (new_name, set_new_name) = create_signal(String::new());
    let (new_email, set_new_email) = create_signal(String::new());
    let (new_password, set_new_password) = create_signal(String::new());
    let (new_role, set_new_role) = create_signal("member".to_string());
    let (error, set_error) = create_signal(Option::<String>::None);

    create_effect(move |_| {
        spawn_local(async move {
            if let Ok(fetched) = api::get_users().await {
                set_users.set(fetched);
            }
        });
    });

    let create_user = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_error.set(None);

        let req = CreateUserRequest {
            name: new_name.get(),
            email: new_email.get(),
            password: new_password.get(),
            role: new_role.get(),
        };

        spawn_local(async move {
            match api::create_user(req).await {
                Ok(user) => {
                    set_users.update(|u| u.push(user));
                    set_show_form.set(false);
                    set_new_name.set(String::new());
                    set_new_email.set(String::new());
                    set_new_password.set(String::new());
                    set_new_role.set("member".to_string());
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let delete_user = move |id: i64| {
        spawn_local(async move {
            if api::delete_user(id).await.is_ok() {
                set_users.update(|users| users.retain(|u| u.id != id));
            }
        });
    };

    view! {
        <div class="bg-white p-4 rounded-lg shadow">
            <div class="flex justify-between items-center mb-4">
                <h3 class="font-semibold">"Users"</h3>
                <button
                    on:click=move |_| set_show_form.update(|v| *v = !*v)
                    class="bg-blue-600 text-white px-3 py-1 rounded text-sm hover:bg-blue-700"
                >
                    {move || if show_form.get() { "Cancel" } else { "Add User" }}
                </button>
            </div>

            {move || show_form.get().then(|| view! {
                <form on:submit=create_user class="mb-4 p-3 bg-gray-50 rounded space-y-3">
                    {move || error.get().map(|e| view! {
                        <div class="bg-red-100 text-red-700 p-2 rounded text-sm">{e}</div>
                    })}
                    <div class="grid grid-cols-2 gap-2">
                        <input type="text" placeholder="Name" required
                            class="border rounded px-3 py-2"
                            prop:value=new_name
                            on:input=move |ev| set_new_name.set(event_target_value(&ev)) />
                        <input type="email" placeholder="Email" required
                            class="border rounded px-3 py-2"
                            prop:value=new_email
                            on:input=move |ev| set_new_email.set(event_target_value(&ev)) />
                        <input type="password" placeholder="Password" required
                            class="border rounded px-3 py-2"
                            prop:value=new_password
                            on:input=move |ev| set_new_password.set(event_target_value(&ev)) />
                        <select class="border rounded px-3 py-2"
                            on:change=move |ev| set_new_role.set(event_target_value(&ev))>
                            <option value="member" selected>"Member"</option>
                            <option value="admin">"Admin"</option>
                        </select>
                    </div>
                    <button type="submit"
                        class="bg-green-600 text-white px-4 py-2 rounded text-sm hover:bg-green-700"
                    >"Create"</button>
                </form>
            })}

            <table class="w-full text-sm">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-4 py-2 text-left">"Name"</th>
                        <th class="px-4 py-2 text-left">"Email"</th>
                        <th class="px-4 py-2 text-left">"Role"</th>
                        <th class="px-4 py-2 text-left">"Actions"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || users.get()
                        key=|u| u.id
                        children=move |user| {
                            let user_id = user.id;
                            view! {
                                <tr class="border-t">
                                    <td class="px-4 py-2">{user.name.clone()}</td>
                                    <td class="px-4 py-2">{user.email.clone()}</td>
                                    <td class="px-4 py-2">
                                        <span class={if user.role == "admin" { "text-red-600 font-medium" } else { "text-gray-600" }}>
                                            {user.role.clone()}
                                        </span>
                                    </td>
                                    <td class="px-4 py-2">
                                        <button
                                            on:click=move |_| delete_user(user_id)
                                            class="text-red-600 hover:underline text-xs"
                                        >"Delete"</button>
                                    </td>
                                </tr>
                            }
                        }
                    />
                </tbody>
            </table>
        </div>
    }
}

// ============ ML ============

#[component]
fn MlSection() -> impl IntoView {
    let (status, set_status) = create_signal(Option::<MlStatusResponse>::None);
    let (retrain_result, set_retrain_result) = create_signal(Option::<MlRetrainResponse>::None);
    let (retraining, set_retraining) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);

    create_effect(move |_| {
        spawn_local(async move {
            if let Ok(s) = api::get_ml_status().await {
                set_status.set(Some(s));
            }
        });
    });

    let retrain = move |_| {
        set_retraining.set(true);
        set_error.set(None);
        set_retrain_result.set(None);

        spawn_local(async move {
            match api::trigger_retrain().await {
                Ok(result) => {
                    set_retrain_result.set(Some(result));
                    // Обновить статус
                    if let Ok(s) = api::get_ml_status().await {
                        set_status.set(Some(s));
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_retraining.set(false);
        });
    };

    view! {
        <div class="bg-white p-4 rounded-lg shadow">
            <div class="flex justify-between items-center mb-4">
                <h3 class="font-semibold">"ML Model"</h3>
                <button
                    on:click=retrain
                    disabled=retraining
                    class="bg-purple-600 text-white px-3 py-1 rounded text-sm hover:bg-purple-700 disabled:opacity-50"
                >
                    {move || if retraining.get() { "Training..." } else { "Retrain" }}
                </button>
            </div>

            {move || error.get().map(|e| view! {
                <div class="bg-red-100 text-red-700 p-2 rounded text-sm mb-3">{e}</div>
            })}

            {move || status.get().map(|s| {
                view! {
                    <div class="grid grid-cols-2 gap-3 text-sm">
                        <div>
                            <span class="text-gray-500">"Active Model: "</span>
                            <span class="font-medium">{s.active_model.clone().unwrap_or_else(|| "None".to_string())}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">"Activated: "</span>
                            <span>{s.activated_at.clone().unwrap_or_else(|| "—".to_string())}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">"R²: "</span>
                            <span>{s.r2_val.map(|v| format!("{:.4}", v)).unwrap_or_else(|| "—".to_string())}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">"MdAPE: "</span>
                            <span>{s.mdape_val.map(|v| format!("{:.1}%", v)).unwrap_or_else(|| "—".to_string())}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">"MedAE: "</span>
                            <span>{s.medae_val.map(|v| format!("{:.1} min", v)).unwrap_or_else(|| "—".to_string())}</span>
                        </div>
                    </div>
                }
            })}

            {move || retrain_result.get().map(|r| {
                let bg = if r.activated { "bg-green-100 text-green-700" } else { "bg-yellow-100 text-yellow-700" };
                view! {
                    <div class={format!("mt-3 p-3 rounded text-sm {}", bg)}>
                        <p><strong>"Status: "</strong>{r.status.clone()}</p>
                        <p><strong>"Best Model: "</strong>{r.best_model.clone().unwrap_or_else(|| "—".to_string())}</p>
                        <p><strong>"Activated: "</strong>{if r.activated { "Yes" } else { "No" }}</p>
                        {r.message.clone().map(|m| view! { <p><strong>"Message: "</strong>{m}</p> })}
                    </div>
                }
            })}
        </div>
    }
}

// ============ Archive ============

#[component]
fn ArchiveSection() -> impl IntoView {
    let (result, set_result) = create_signal(Option::<String>::None);
    let (archiving, set_archiving) = create_signal(false);

    let archive = move |_| {
        set_archiving.set(true);
        set_result.set(None);

        spawn_local(async move {
            match api::archive_completed().await {
                Ok(r) => set_result.set(Some(format!("Archived {} tasks", r.archived))),
                Err(e) => set_result.set(Some(format!("Error: {}", e))),
            }
            set_archiving.set(false);
        });
    };

    view! {
        <div class="bg-white p-4 rounded-lg shadow">
            <div class="flex justify-between items-center">
                <div>
                    <h3 class="font-semibold">"Archive"</h3>
                    <p class="text-sm text-gray-500">"Move all completed tasks to archive"</p>
                </div>
                <button
                    on:click=archive
                    disabled=archiving
                    class="bg-orange-600 text-white px-3 py-1 rounded text-sm hover:bg-orange-700 disabled:opacity-50"
                >
                    {move || if archiving.get() { "Archiving..." } else { "Archive Completed" }}
                </button>
            </div>
            {move || result.get().map(|r| view! {
                <p class="mt-2 text-sm text-gray-600">{r}</p>
            })}
        </div>
    }
}