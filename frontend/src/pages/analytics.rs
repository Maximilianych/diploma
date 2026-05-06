use leptos::*;
use crate::api;
use crate::models::AnalyticsResponse;

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let (data, set_data) = create_signal(Option::<AnalyticsResponse>::None);
    let (loading, set_loading) = create_signal(true);

    create_effect(move |_| {
        spawn_local(async move {
            if let Ok(analytics) = api::get_analytics().await {
                set_data.set(Some(analytics));
            }
            set_loading.set(false);
        });
    });

    view! {
        <div>
            <h2 class="text-lg font-semibold mb-4">"Analytics"</h2>

            {move || {
                if loading.get() {
                    return view! { <p>"Loading..."</p> }.into_view();
                }

                match data.get() {
                    None => view! { <p class="text-gray-500">"No data available"</p> }.into_view(),
                    Some(analytics) => view! {
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                            <StatusChart data=analytics.tasks_by_status.clone() />
                            <UserTasksChart data=analytics.tasks_by_user.clone() />
                            <PredictionChart data=analytics.prediction_accuracy.clone() />
                            <AvgTimeChart data=analytics.avg_time_by_user.clone() />
                        </div>
                    }.into_view(),
                }
            }}
        </div>
    }
}

#[component]
fn StatusChart(data: crate::models::TasksByStatus) -> impl IntoView {
    let total = (data.todo + data.in_progress + data.done).max(1) as f64;

    let bar = move |label: &str, count: i64, color: &str| {
        let pct = (count as f64 / total * 100.0) as u32;
        let width = format!("{}%", pct.max(2));
        let bg = color.to_string();
        view! {
            <div class="mb-2">
                <div class="flex justify-between text-sm mb-1">
                    <span>{label.to_string()}</span>
                    <span>{count}</span>
                </div>
                <div class="w-full bg-gray-200 rounded h-6">
                    <div class={format!("h-6 rounded {}", bg)} style={format!("width: {}", width)}></div>
                </div>
            </div>
        }
    };

    view! {
        <div class="bg-white p-4 rounded-lg shadow">
            <h3 class="font-semibold mb-4">"Tasks by Status"</h3>
            {bar("To Do", data.todo, "bg-yellow-400")}
            {bar("In Progress", data.in_progress, "bg-blue-400")}
            {bar("Done", data.done, "bg-green-400")}
        </div>
    }
}

#[component]
fn UserTasksChart(data: Vec<crate::models::UserTaskStats>) -> impl IntoView {
    view! {
        <div class="bg-white p-4 rounded-lg shadow">
            <h3 class="font-semibold mb-4">"Tasks by User"</h3>
            {if data.is_empty() {
                view! { <p class="text-gray-500 text-sm">"No assigned tasks"</p> }.into_view()
            } else {
                view! {
                    <div class="space-y-3">
                        {data.iter().map(|u| {
                            let total = (u.todo + u.in_progress + u.done).max(1) as f64;
                            let w_todo = format!("{}%", (u.todo as f64 / total * 100.0) as u32);
                            let w_prog = format!("{}%", (u.in_progress as f64 / total * 100.0) as u32);
                            let w_done = format!("{}%", (u.done as f64 / total * 100.0) as u32);

                            view! {
                                <div>
                                    <div class="flex justify-between text-sm mb-1">
                                        <span>{u.user_name.clone()}</span>
                                        <span>{u.todo + u.in_progress + u.done}</span>
                                    </div>
                                    <div class="w-full flex h-5 rounded overflow-hidden">
                                        <div class="bg-yellow-400" style={format!("width: {}", w_todo)}></div>
                                        <div class="bg-blue-400" style={format!("width: {}", w_prog)}></div>
                                        <div class="bg-green-400" style={format!("width: {}", w_done)}></div>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                    <div class="flex gap-4 mt-3 text-xs text-gray-500">
                        <span class="flex items-center gap-1">
                            <span class="w-3 h-3 bg-yellow-400 rounded"></span> "To Do"
                        </span>
                        <span class="flex items-center gap-1">
                            <span class="w-3 h-3 bg-blue-400 rounded"></span> "In Progress"
                        </span>
                        <span class="flex items-center gap-1">
                            <span class="w-3 h-3 bg-green-400 rounded"></span> "Done"
                        </span>
                    </div>
                }.into_view()
            }}
        </div>
    }
}

#[component]
fn PredictionChart(data: Vec<crate::models::PredictionPoint>) -> impl IntoView {
    view! {
        <div class="bg-white p-4 rounded-lg shadow">
            <h3 class="font-semibold mb-4">"Prediction vs Actual"</h3>
            {if data.is_empty() {
                view! { <p class="text-gray-500 text-sm">"No completed tasks with predictions"</p> }.into_view()
            } else {
                let max_val = data.iter()
                    .flat_map(|p| [p.predicted_hours, p.actual_hours])
                    .fold(1.0_f64, f64::max);

                view! {
                    <div class="space-y-2">
                        <div class="flex text-xs text-gray-500 mb-2">
                            <span class="flex items-center gap-1">
                                <span class="w-3 h-3 bg-blue-400 rounded"></span> "Predicted"
                            </span>
                            <span class="flex items-center gap-1 ml-4">
                                <span class="w-3 h-3 bg-green-400 rounded"></span> "Actual"
                            </span>
                        </div>
                        {data.iter().map(|p| {
                            let pred_w = format!("{}%", (p.predicted_hours / max_val * 100.0).min(100.0) as u32);
                            let act_w = format!("{}%", (p.actual_hours / max_val * 100.0).min(100.0) as u32);

                            view! {
                                <div class="text-xs">
                                    <span class="text-gray-600">{format!("Task #{}", p.task_id)}</span>
                                    <div class="flex items-center gap-2 mt-1">
                                        <div class="w-full bg-gray-100 rounded h-3">
                                            <div class="bg-blue-400 h-3 rounded" style={format!("width: {}", pred_w)}></div>
                                        </div>
                                        <span class="w-12 text-right">{format!("{:.1}h", p.predicted_hours)}</span>
                                    </div>
                                    <div class="flex items-center gap-2 mt-0.5">
                                        <div class="w-full bg-gray-100 rounded h-3">
                                            <div class="bg-green-400 h-3 rounded" style={format!("width: {}", act_w)}></div>
                                        </div>
                                        <span class="w-12 text-right">{format!("{:.1}h", p.actual_hours)}</span>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_view()
            }}
        </div>
    }
}

#[component]
fn AvgTimeChart(data: Vec<crate::models::UserAvgTime>) -> impl IntoView {
    view! {
        <div class="bg-white p-4 rounded-lg shadow">
            <h3 class="font-semibold mb-4">"Avg Completion Time by User"</h3>
            {if data.is_empty() {
                view! { <p class="text-gray-500 text-sm">"No completed tasks"</p> }.into_view()
            } else {
                let max_avg = data.iter().map(|u| u.avg_hours).fold(1.0_f64, f64::max);

                view! {
                    <div class="space-y-2">
                        {data.iter().map(|u| {
                            let w = format!("{}%", (u.avg_hours / max_avg * 100.0) as u32);
                            view! {
                                <div>
                                    <div class="flex justify-between text-sm mb-1">
                                        <span>{u.user_name.clone()}</span>
                                        <span>{format!("{:.1}h ({} tasks)", u.avg_hours, u.task_count)}</span>
                                    </div>
                                    <div class="w-full bg-gray-200 rounded h-5">
                                        <div class="bg-purple-400 h-5 rounded" style={format!("width: {}", w)}></div>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_view()
            }}
        </div>
    }
}