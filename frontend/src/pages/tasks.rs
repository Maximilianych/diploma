use leptos::*;
use std::rc::Rc;
use crate::api;
use crate::models::{Task, User, UpdateTaskRequest};

fn user_name_by_id(users: &[User], user_id: Option<i64>) -> String {
    match user_id {
        None => "—".to_string(),
        Some(id) => users.iter().find(|u| u.id == id)
            .map(|u| u.name.clone())
            .unwrap_or_else(|| format!("#{}", id)),
    }
}

fn status_class(status: &str) -> &'static str {
    match status {
        "todo" => "bg-yellow-100 text-yellow-800",
        "in_progress" => "bg-blue-100 text-blue-800",
        "done" => "bg-green-100 text-green-800",
        _ => "bg-gray-100 text-gray-800",
    }
}

#[component]
pub fn TasksPage(users: Vec<User>) -> impl IntoView {
    let (tasks, set_tasks) = create_signal(Vec::<Task>::new());
    let (loading, set_loading) = create_signal(true);
    let (editing_task, set_editing_task) = create_signal(Option::<Task>::None);
    let (new_title, set_new_title) = create_signal(String::new());
    let (new_desc, set_new_desc) = create_signal(String::new());
    let (new_assignee, set_new_assignee) = create_signal(Option::<i64>::None);

    let users_for_modal = users.clone();
    let users_for_select = users.clone();
    let users_rc = Rc::new(users);

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
        let assignee = new_assignee.get();

        if title.is_empty() { return; }

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

    view! {
        <div>
            <h2 class="text-lg font-semibold mb-4">"All Tasks"</h2>

            // Форма создания
            <form on:submit=create_task class="bg-white p-4 rounded-lg shadow mb-6">
                <h3 class="font-semibold mb-3">"New Task"</h3>
                <div class="flex gap-2 flex-wrap">
                    <input type="text" placeholder="Title"
                        class="flex-1 min-w-48 border rounded px-3 py-2"
                        prop:value=new_title
                        on:input=move |ev| set_new_title.set(event_target_value(&ev))
                    />
                    <input type="text" placeholder="Description (optional)"
                        class="flex-1 min-w-48 border rounded px-3 py-2"
                        prop:value=new_desc
                        on:input=move |ev| set_new_desc.set(event_target_value(&ev))
                    />
                    <select class="border rounded px-3 py-2"
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            set_new_assignee.set(val.parse().ok());
                        }
                    >
                        <option value="">"Unassigned"</option>
                        <For
                            each=move || users_for_select.clone()
                            key=|u| u.id
                            children=move |u| {
                                view! { <option value={u.id.to_string()}>{u.name.clone()}</option> }
                            }
                        />
                    </select>
                    <button type="submit"
                        class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
                    >"Add"</button>
                </div>
            </form>

            // Таблица задач
            {move || {
                let users_ref = users_rc.clone();

                if loading.get() {
                    view! { <p>"Loading..."</p> }.into_view()
                } else {
                    view! {
                        <div class="bg-white rounded-lg shadow overflow-x-auto">
                            <table class="w-full text-sm">
                                <thead class="bg-gray-50">
                                    <tr>
                                        <th class="px-4 py-3 text-left">"Title"</th>
                                        <th class="px-4 py-3 text-left w-12">"Status"</th>
                                        <th class="px-4 py-3 text-left w-12">"Assignee"</th>
                                        <th class="px-4 py-3 text-left w-12">"Predicted"</th>
                                        <th class="px-4 py-3 text-left w-12">"Actual"</th>
                                        <th class="px-4 py-3 text-left w-12">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || tasks.get()
                                        key=|task| (
                                            task.id, task.title.clone(),
                                            task.status.clone(), task.assignee_id,
                                            task.actual_hours.map(|h| h.to_bits()),
                                        )
                                        children=move |task| {
                                            let users_inner = users_ref.clone();
                                            let task_for_edit = task.clone();
                                            let task_id = task.id;
                                            let assignee = user_name_by_id(&users_inner, task.assignee_id);
                                            let sc = status_class(&task.status).to_string();

                                            view! {
                                                <tr class="border-t hover:bg-gray-50">
                                                    <td class="px-4 py-3 max-w-xs">
                                                        <div class="font-medium">{task.title.clone()}</div>
                                                        {task.description.clone().map(|d| view! {
                                                            <div class="text-xs text-gray-500 truncate">{d}</div>
                                                        })}
                                                    </td>
                                                    <td class="px-4 py-3">
                                                        <span class={format!("px-2 py-1 rounded text-xs {}", sc)}>
                                                            {task.status.clone()}
                                                        </span>
                                                    </td>
                                                    <td class="px-4 py-3">{assignee}</td>
                                                    <td class="px-4 py-3">
                                                        {task.predicted_hours.map(|h| format!("{:.1}h", h)).unwrap_or_else(|| "—".to_string())}
                                                    </td>
                                                    <td class="px-4 py-3">
                                                        {task.actual_hours.map(|h| format!("{:.1}h", h)).unwrap_or_else(|| "—".to_string())}
                                                    </td>
                                                    <td class="px-4 py-3">
                                                        <div class="flex gap-2">
                                                            <button
                                                                on:click=move |_| set_editing_task.set(Some(task_for_edit.clone()))
                                                                class="text-blue-600 hover:underline text-xs"
                                                            >"Edit"</button>
                                                            <button
                                                                on:click=move |_| {
                                                                    spawn_local(async move {
                                                                        if api::delete_task(task_id).await.is_ok() {
                                                                            set_tasks.update(|t| t.retain(|t| t.id != task_id));
                                                                        }
                                                                    });
                                                                }
                                                                class="text-red-600 hover:underline text-xs"
                                                            >"Delete"</button>
                                                        </div>
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    />
                                </tbody>
                            </table>
                        </div>
                    }.into_view()
                }
            }}

            // Модалка редактирования
            {move || {
                editing_task.get().map(|task| {
                    view! {
                        <EditTaskModal
                            task=task
                            users=users_for_modal.clone()
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
                        let new_tasks: Vec<Task> = tasks.iter()
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
                    <button on:click=move |_| close() class="text-gray-500 hover:text-gray-700 text-xl">"×"</button>
                </div>
                <form on:submit=submit class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium mb-1">"Title"</label>
                        <input type="text" class="w-full border rounded px-3 py-2"
                            prop:value=title on:input=move |ev| set_title.set(event_target_value(&ev)) required />
                    </div>
                    <div>
                        <label class="block text-sm font-medium mb-1">"Description"</label>
                        <textarea class="w-full border rounded px-3 py-2" rows="3"
                            prop:value=description on:input=move |ev| set_description.set(event_target_value(&ev)) />
                    </div>
                    <div>
                        <label class="block text-sm font-medium mb-1">"Status"</label>
                        <select class="w-full border rounded px-3 py-2"
                            on:change=move |ev| set_status.set(event_target_value(&ev))>
                            {statuses.iter().map(|s| {
                                let selected = *s == status.get();
                                view! { <option value=*s selected=selected>{*s}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium mb-1">"Assignee"</label>
                        <select class="w-full border rounded px-3 py-2"
                            on:change=move |ev| { set_assignee_id.set(event_target_value(&ev).parse().ok()); }>
                            <option value="" selected=assignee_id.get().is_none()>"Unassigned"</option>
                            {users.iter().map(|u| {
                                let selected = assignee_id.get() == Some(u.id);
                                view! { <option value={u.id.to_string()} selected=selected>{u.name.clone()}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium mb-1">"Actual Hours"</label>
                        <input type="number" step="0.5" min="0" class="w-full border rounded px-3 py-2"
                            prop:value=actual_hours on:input=move |ev| set_actual_hours.set(event_target_value(&ev)) />
                    </div>
                    <div class="flex gap-2 justify-end">
                        <button type="button" on:click=move |_| close()
                            class="px-4 py-2 border rounded hover:bg-gray-100">"Cancel"</button>
                        <button type="submit"
                            class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
                            disabled=saving>
                            {move || if saving.get() { "Saving..." } else { "Save" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}