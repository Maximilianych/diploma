mod api;
mod models;
mod pages;

use leptos::*;
use pages::{login::LoginPage, tasks::TasksPage};
use crate::models::User;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (user, set_user) = create_signal(Option::<User>::None);

    view! {
        {move || {
            match user.get() {
                None => view! { <LoginPage on_login=set_user /> }.into_view(),
                Some(u) => view! { <TasksPage user=u on_logout=set_user /> }.into_view(),
            }
        }}
    }
}