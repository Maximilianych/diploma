use crate::api;
use crate::models::{Task, UpdateTaskRequest, User};
use leptos::*;

#[component]
pub fn TasksPage(user: User, on_logout: WriteSignal<Option<User>>) -> impl IntoView {
    let (tasks, set_tasks) = create_signal(Vec::<Task>::new());
    let (users, set_users) = create_signal(Vec::<User>::new());
    let (new_title, set_new_title) = create_signal(String::new());
    let (new_desc, set_new_desc) = create_signal(String::new());
    let (new_assignee, set_new_assignee) = create_signal(Option::<i64>::None);
    let (loading, set_loading) = create_signal(true);
    let (editing_task, set_editing_task) = create_signal(Option::<Task>::None);

    // Загрузка данных при монтировании
    create_effect(move |_| {
        spawn_local(async move {
            let tasks_result = api::get_tasks().await;
            let users_result = api::get_users().await;

            if let Ok(fetched) = tasks_result {
                set_tasks.set(fetched);
            }
            if let Ok(fetched) = users_result {
                set_users.set(fetched);
            }
            set_loading.set(false);
        });
    });

    let create_task = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let title = new_title.get();
        let desc = new_desc.get();
        let assignee = new_assignee.get();

        if title.is_empty() {
            return;
        }

        spawn_local(async move {
            let description = if desc.is_empty() { None } else { Some(desc) };

            if let Ok(task) = api::create_task(title, description, assignee).await {
                set_tasks.update(|t| t.push(task));
                set_new_title.set(String::new());
                set_new_desc.set(String::new());
                set_new_assignee.set(None);
            }
        });
    };

    let update_status = move |id: i64, status: String| {
        spawn_local(async move {
            let req = UpdateTaskRequest {
                title: None,
                description: None,
                status: Some(status),
                assignee_id: None,
                actual_hours: None,
            };
            if let Ok(updated) = api::update_task(id, req).await {
                set_tasks.update(|tasks| {
                    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                        *task = updated;
                    }
                });
            }
        });
    };

    let save_task = move |updated: Task| {
        spawn_local(async move {
            println!("Saving task");
            let req = UpdateTaskRequest {
                title: Some(updated.title.clone()),
                description: Some(updated.description.clone()),
                status: Some(updated.status.clone()),
                assignee_id: Some(updated.assignee_id),
                actual_hours: updated.actual_hours,
            };
            if let Ok(saved) = api::update_task(updated.id, req).await {
                set_tasks.update(|tasks| {
                    if let Some(task) = tasks.iter_mut().find(|t| t.id == saved.id) {
                        *task = saved;
                    }
                });
                set_editing_task.set(None);
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

    let get_user_name = move |user_id: Option<i64>| -> String {
        match user_id {
            None => "Unassigned".to_string(),
            Some(id) => users
                .get()
                .iter()
                .find(|u| u.id == id)
                .map(|u| u.name.clone())
                .unwrap_or_else(|| format!("User #{}", id)),
        }
    };

    let status_labels = [
        ("todo", "To Do"),
        ("in_progress", "In Progress"),
        ("done", "Done"),
    ];

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
                    <div class="flex gap-2 flex-wrap">
                        <input
                            type="text"
                            placeholder="Title"
                            class="flex-1 min-w-48 border rounded px-3 py-2"
                            prop:value=new_title
                            on:input=move |ev| set_new_title.set(event_target_value(&ev))
                        />
                        <input
                            type="text"
                            placeholder="Description (optional)"
                            class="flex-1 min-w-48 border rounded px-3 py-2"
                            prop:value=new_desc
                            on:input=move |ev| set_new_desc.set(event_target_value(&ev))
                        />
                        <select
                            class="border rounded px-3 py-2"
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                set_new_assignee.set(val.parse().ok());
                            }
                        >
                            <option value="">"Unassigned"</option>
                            <For
                                each=move || users.get()
                                key=|u| u.id
                                children=move |u| {
                                    view! { <option value={u.id.to_string()}>{u.name.clone()}</option> }
                                }
                            />
                        </select>
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
                            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
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
                                                    key=|task| (task.id, task.title.clone(), task.description.clone(), task.assignee_id, task.status.clone(), task.actual_hours.map(|h| h.to_bits()))
                                                    children=move |task| {
                                                        let task_for_edit = task.clone();
                                                        let task_id = task.id;
                                                        let assignee_name = get_user_name(task.assignee_id);

                                                        view! {
                                                            <TaskCard
                                                                task=task
                                                                assignee_name=assignee_name
                                                                on_status_change=move |s| update_status(task_id, s)
                                                                on_edit=move || set_editing_task.set(Some(task_for_edit.clone()))
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

            // Модальное окно редактирования
            {move || {
                editing_task.get().map(|task| {
                    view! {
                        <EditTaskModal
                            task=task
                            users=users.get()
                            set_tasks=set_tasks
                            set_editing_task=set_editing_task
                        />
                    }
                })
            }}
        </div>
    }
}

#[component]
fn TaskCard<S, E, D>(
    task: Task,
    assignee_name: String,
    on_status_change: S,
    on_edit: E,
    on_delete: D,
) -> impl IntoView
where
    S: Fn(String) + 'static,
    E: Fn() + 'static,
    D: Fn() + 'static,
{
    let statuses = ["todo", "in_progress", "done"];

    view! {
        <div class="bg-white p-3 rounded shadow">
            <div class="flex justify-between items-start mb-2">
                <h4 class="font-medium">{task.title.clone()}</h4>
                <div class="flex gap-1">
                    <button
                        on:click=move |_| on_edit()
                        class="text-blue-500 hover:text-blue-700 text-sm"
                        title="Edit"
                    >
                        "✎"
                    </button>
                    <button
                        on:click=move |_| on_delete()
                        class="text-red-500 hover:text-red-700 text-sm"
                        title="Delete"
                    >
                        "×"
                    </button>
                </div>
            </div>

            {task.description.clone().map(|d| view! {
                <p class="text-sm text-gray-600 mb-2">{d}</p>
            })}

            <p class="text-xs text-gray-500 mb-1">"Assignee: " {assignee_name}</p>

            {task.predicted_hours.map(|h| view! {
                <p class="text-xs text-gray-500 mb-1">"Predicted: " {format!("{:.1}h", h)}</p>
            })}

            {task.actual_hours.map(|h| view! {
                <p class="text-xs text-gray-500 mb-1">"Actual: " {format!("{:.1}h", h)}</p>
            })}

            <select
                class="w-full text-sm border rounded px-2 py-1 mt-2"
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

#[component]
fn EditTaskModal(
    task: Task,
    users: Vec<User>,
    set_tasks: WriteSignal<Vec<Task>>,
    set_editing_task: WriteSignal<Option<Task>>,
) -> impl IntoView {
    let (title, set_title) = create_signal(task.title.clone());
    let (description, set_description) = create_signal(task.description.clone().unwrap_or_default());
    let (status, set_status) = create_signal(task.status.clone());
    let (assignee_id, set_assignee_id) = create_signal(task.assignee_id);
    let (actual_hours, set_actual_hours) = create_signal(
        task.actual_hours.map(|h| h.to_string()).unwrap_or_default()
    );
    let (saving, set_saving) = create_signal(false);

    let task_id = task.id;

    let close = move || set_editing_task.set(None);

    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);

        let req = UpdateTaskRequest {
            title: Some(title.get()),
            description: Some(if description.get().is_empty() { None } else { Some(description.get()) }),
            status: Some(status.get()),
            assignee_id: Some(assignee_id.get()),
            actual_hours: actual_hours.get().parse().ok(),
        };

        spawn_local(async move {
            match api::update_task(task_id, req).await {
                Ok(updated) => {
                    set_editing_task.set(None);
                    
                    set_tasks.update(|tasks| {
                        let new_tasks: Vec<Task> = tasks
                            .iter()
                            .map(|t| if t.id == task_id { updated.clone() } else { t.clone() })
                            .collect();
                        *tasks = new_tasks;
                    });
                }
                Err(e) => {
                    web_sys::console::log_1(&format!("Error: {}", e).into());
                    set_saving.set(false);
                }
            }
        });
    };

    let statuses = ["todo", "in_progress", "done"];

    view! {
        <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div class="bg-white rounded-lg p-6 w-full max-w-md">
                <div class="flex justify-between items-center mb-4">
                    <h2 class="text-lg font-semibold">"Edit Task"</h2>
                    <button
                        on:click=move |_| close()
                        class="text-gray-500 hover:text-gray-700 text-xl"
                    >
                        "×"
                    </button>
                </div>

                <form on:submit=submit class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium mb-1">"Title"</label>
                        <input
                            type="text"
                            class="w-full border rounded px-3 py-2"
                            prop:value=title
                            on:input=move |ev| set_title.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium mb-1">"Description"</label>
                        <textarea
                            class="w-full border rounded px-3 py-2"
                            rows="3"
                            prop:value=description
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium mb-1">"Status"</label>
                        <select
                            class="w-full border rounded px-3 py-2"
                            on:change=move |ev| set_status.set(event_target_value(&ev))
                        >
                            {statuses.iter().map(|s| {
                                let selected = *s == status.get();
                                view! { <option value=*s selected=selected>{*s}</option> }
                            }).collect_view()}
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium mb-1">"Assignee"</label>
                        <select
                            class="w-full border rounded px-3 py-2"
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                set_assignee_id.set(val.parse().ok());
                            }
                        >
                            <option value="" selected=assignee_id.get().is_none()>"Unassigned"</option>
                            {users.iter().map(|u| {
                                let selected = assignee_id.get() == Some(u.id);
                                let id_str = u.id.to_string();
                                view! { <option value=id_str selected=selected>{u.name.clone()}</option> }
                            }).collect_view()}
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium mb-1">"Actual Hours"</label>
                        <input
                            type="number"
                            step="0.5"
                            min="0"
                            class="w-full border rounded px-3 py-2"
                            prop:value=actual_hours
                            on:input=move |ev| set_actual_hours.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="flex gap-2 justify-end">
                        <button
                            type="button"
                            on:click=move |_| close()
                            class="px-4 py-2 border rounded hover:bg-gray-100"
                        >
                            "Cancel"
                        </button>
                        <button
                            type="submit"
                            class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
                            disabled=saving
                        >
                            {move || if saving.get() { "Saving..." } else { "Save" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}