use yew::prelude::*;
fn use_user(){

}
#[function_component(NavBar)]
fn nav_bar() -> Html {
    html! {
        <nav>
            <a class="nav-tail" id="login-button" href="/login/github">{"Login with github"}</a>
        </nav>
    }
}
#[function_component(Home)]
fn home_page() -> Html {
    html! {
        <NavBar/>
    }
}
fn main() {
    yew::start_app::<Home>();
}