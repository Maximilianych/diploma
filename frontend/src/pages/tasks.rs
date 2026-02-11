use leptos::*;
use crate::api;
use crate::models::{Task, User};

#[component]
pub fn TasksPage(user: User, on_logout: WriteSignal<Option<User>>) -> impl IntoView {
    let (tasks, set_tasks) = create_signal(Vec::<Task>::new());
    let (new_title, set_new_title) = create_signal(String::new());
    let (new_desc, set_new_desc) = create_signal(String::new());
    let (loading, set_loading) = create_signal(true);

    // Загрузка задач при монтировании
    create_effect(move |_| {
        spawn_local(async move {
            if let Ok(fetched) = api::get_tasks().await {
                set_tasks.set(fetched);
            }
            set_loading.set(false);
        });
    });

    let create_task = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let title = new_title.get();
        let desc = new_desc.get();

        if title.is_empty() {
            return;
        }

        spawn_local(async move {
            let description = if desc.is_empty() { None } else { Some(desc) };

            if let Ok(task) = api::create_task(title, description).await {
                set_tasks.update(|t| t.push(task));
                set_new_title.set(String::new());
                set_new_desc.set(String::new());
            }
        });
    };

    let update_status = move |id: i64, status: String| {
        spawn_local(async move {
            if let Ok(updated) = api::update_task_status(id, status).await {
                set_tasks.update(|tasks| {
                    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                        *task = updated;
                    }
                });
            }
        });
    };

    let delete = move |id: i64| {
        spawn_local(async move {
            if api::delete_task(id).await.is_ok() {
                set_tasks.update(|tasks| tasks.retain(|t| t.id != id));
            }
        });
    };

    let logout = move |_| {
        api::clear_token();
        on_logout.set(None);
    };

    let statuses = ["todo", "in_progress", "done"];
    let status_labels = [("todo", "To Do"), ("in_progress", "In Progress"), ("done", "Done")];

    view! {
        <div class="min-h-screen bg-gray-100">
            // Header
            <header class="bg-white shadow">
                <div class="max-w-7xl mx-auto px-4 py-4 flex justify-between items-center">
                    <h1 class="text-xl font-bold">"Task Tracker"</h1>
                    <div class="flex items-center gap-4">
                        <span class="text-gray-600">{user.name.clone()}</span>
                        <button
                            on:click=logout
                            class="text-red-600 hover:underline"
                        >
                            "Logout"
                        </button>
                    </div>
                </div>
            </header>

            <main class="max-w-7xl mx-auto px-4 py-6">
                // Форма создания задачи
                <form on:submit=create_task class="bg-white p-4 rounded-lg shadow mb-6">
                    <h2 class="font-semibold mb-3">"New Task"</h2>
                    <div class="flex gap-2">
                        <input
                            type="text"
                            placeholder="Title"
                            class="flex-1 border rounded px-3 py-2"
                            prop:value=new_title
                            on:input=move |ev| set_new_title.set(event_target_value(&ev))
                        />
                        <input
                            type="text"
                            placeholder="Description (optional)"
                            class="flex-1 border rounded px-3 py-2"
                            prop:value=new_desc
                            on:input=move |ev| set_new_desc.set(event_target_value(&ev))
                        />
                        <button
                            type="submit"
                            class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
                        >
                            "Add"
                        </button>
                    </div>
                </form>

                // Kanban доска
                {move || {
                    if loading.get() {
                        view! { <p>"Loading..."</p> }.into_view()
                    } else {
                        view! {
                            <div class="grid grid-cols-3 gap-4">
                                {status_labels.iter().map(|(status, label)| {
                                    let status = status.to_string();
                                    let label = label.to_string();

                                    view! {
                                        <div class="bg-gray-200 rounded-lg p-4">
                                            <h3 class="font-semibold mb-3">{label}</h3>
                                            <div class="space-y-2">
                                                <For
                                                    each=move || {
                                                        let s = status.clone();
                                                        tasks.get().into_iter().filter(move |t| t.status == s).collect::<Vec<_>>()
                                                    }
                                                    key=|task| task.id
                                                    children=move |task| {
                                                        let task_id = task.id;
                                                        let current_status = task.status.clone();

                                                        view! {
                                                            <TaskCard
                                                                task=task
                                                                on_status_change=move |s| update_status(task_id, s)
                                                                on_delete=move || delete(task_id)
                                                            />
                                                        }
                                                    }
                                                />
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_view()
                    }
                }}
            </main>
        </div>
    }
}

#[component]
fn TaskCard<F, D>(task: Task, on_status_change: F, on_delete: D) -> impl IntoView
where
    F: Fn(String) + 'static,
    D: Fn() + 'static,
{
    let statuses = ["todo", "in_progress", "done"];

    view! {
        <div class="bg-white p-3 rounded shadow">
            <div class="flex justify-between items-start mb-2">
                <h4 class="font-medium">{task.title.clone()}</h4>
                <button
                    on:click=move |_| on_delete()
                    class="text-red-500 hover:text-red-700 text-sm"
                >
                    "×"
                </button>
            </div>

            {task.description.clone().map(|d| view! {
                <p class="text-sm text-gray-600 mb-2">{d}</p>
            })}

            {task.predicted_hours.map(|h| view! {
                <p class="text-xs text-gray-500 mb-2">"Predicted: " {format!("{:.1}h", h)}</p>
            })}

            <select
                class="w-full text-sm border rounded px-2 py-1"
                on:change=move |ev| on_status_change(event_target_value(&ev))
            >
                {statuses.iter().map(|s| {
                    let selected = *s == task.status;
                    view! { <option value=*s selected=selected>{*s}</option> }
                }).collect_view()}
            </select>
        </div>
    }
}