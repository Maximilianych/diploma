mod api;
mod models;
mod pages;

use leptos::*;
use pages::{
    login::LoginPage,
    board::BoardPage,
    tasks::TasksPage,
    analytics::AnalyticsPage,
    admin::AdminPage,
};
use crate::models::User;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[derive(Clone, PartialEq)]
enum Page {
    Board,
    Tasks,
    Analytics,
    Admin,
}

#[component]
fn App() -> impl IntoView {
    let (user, set_user) = create_signal(Option::<User>::None);
    let (page, set_page) = create_signal(Page::Board);
    let (users, set_users) = create_signal(Vec::<User>::new());

    create_effect(move |_| {
        if user.get().is_some() {
            spawn_local(async move {
                if let Ok(fetched) = api::get_users().await {
                    set_users.set(fetched);
                }
            });
        }
    });

    view! {
        {move || {
            match user.get() {
                None => view! { <LoginPage on_login=set_user /> }.into_view(),
                Some(current_user) => {
                    let is_admin = current_user.role == "admin";
                    let user_for_board = current_user.clone();
                    let logout_user = current_user.clone();

                    view! {
                        <div class="min-h-screen bg-gray-100">
                            // Навигация
                            <header class="bg-white shadow">
                                <div class="max-w-7xl mx-auto px-4 py-3 flex justify-between items-center">
                                    <div class="flex items-center gap-6">
                                        <h1 class="text-xl font-bold">"Task Tracker"</h1>
                                        <nav class="flex gap-4">
                                            <NavButton page=Page::Board current_page=page set_page=set_page label="Board" />
                                            <NavButton page=Page::Tasks current_page=page set_page=set_page label="Tasks" />
                                            <NavButton page=Page::Analytics current_page=page set_page=set_page label="Analytics" />
                                            {is_admin.then(|| view! {
                                                <NavButton page=Page::Admin current_page=page set_page=set_page label="Admin" />
                                            })}
                                        </nav>
                                    </div>
                                    <div class="flex items-center gap-4">
                                        <span class="text-gray-600 text-sm">{logout_user.name.clone()}</span>
                                        <button
                                            on:click=move |_| {
                                                api::clear_token();
                                                set_user.set(None);
                                                set_page.set(Page::Board);
                                            }
                                            class="text-red-600 hover:underline text-sm"
                                        >"Logout"</button>
                                    </div>
                                </div>
                            </header>

                            <main class="max-w-7xl mx-auto px-4 py-6">
                                {move || match page.get() {
                                    Page::Board => view! {
                                        <BoardPage user=user_for_board.clone() users=users.get() />
                                    }.into_view(),
                                    Page::Tasks => view! {
                                        <TasksPage users=users.get() />
                                    }.into_view(),
                                    Page::Analytics => view! {
                                        <AnalyticsPage />
                                    }.into_view(),
                                    Page::Admin => view! {
                                        <AdminPage />
                                    }.into_view(),
                                }}
                            </main>
                        </div>
                    }.into_view()
                }
            }
        }}
    }
}

#[component]
fn NavButton(
    page: Page,
    current_page: ReadSignal<Page>,
    set_page: WriteSignal<Page>,
    label: &'static str,
) -> impl IntoView {
    let page_clone = page.clone();

    view! {
        <button
            on:click=move |_| set_page.set(page_clone.clone())
            class=move || {
                if current_page.get() == page {
                    "text-blue-600 font-medium border-b-2 border-blue-600 pb-1"
                } else {
                    "text-gray-600 hover:text-blue-600 pb-1"
                }
            }
        >
            {label}
        </button>
    }
}