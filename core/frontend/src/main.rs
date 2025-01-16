mod tab_view;

use crate::tab_view::TabView;
use gloo_net::http::Request;
use hirola::dom::app::App;
use hirola::dom::*;
use hirola::prelude::*;
use web_sys::window;

pub struct State {
    // config: Config,
}

#[component]
fn Logo() -> Dom {
    html! { <span class="i-logo h-8 w-8"></span> }
}

#[component]
fn SideBar() -> Dom {
    html! {
        <aside class="w-full" aria-label="Sidebar" un-cloak="">
            <div class="h-full px-3 py-4">
                <ul class="space-2 font-medium">
                    <li>
                        <a
                            href="/"
                            class="flex items-center p-2 text-gray-700 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                        >
                            <span class="i-carbon-home"></span>
                            <span class="ms-3">"Dashboard"</span>
                        </a>
                        <a
                            href="/m/tasks"
                            class="flex items-center p-2 text-gray-700 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                        >
                            <span class="i-carbon-task"></span>
                            <span class="ms-3">"Tasks"</span>
                        </a>
                        <a
                            href="/m/notes"
                            class="flex items-center p-2 text-gray-700 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                        >
                            <span class="i-carbon-notebook"></span>
                            <span class="ms-3">"Notes"</span>
                        </a>
                        <a
                            href="/m/calendar"
                            class="flex items-center p-2 text-gray-700 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                        >
                            <span class="i-carbon-calendar-heat-map"></span>
                            <span class="ms-3">"Calendar"</span>
                        </a>
                    </li>
                </ul>
                <ul class="border-t my-2">
                    <h2 class="font-thin font-sans text-gray-700 pt-1">"Vaults"</h2>
                    <li>
                        <a
                            href="/vaults/my-project"
                            class="flex items-center p-2 text-gray-700 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                        >
                            <span class="i-carbon-directory-domain"></span>
                            <span class="ms-3">"My Projects"</span>
                        </a>
                        <a
                            href="/vaults/work"
                            class="flex items-center p-2 text-gray-700 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                        >
                            <span class="i-carbon-directory-domain"></span>
                            <span class="ms-3">"Work"</span>
                        </a>
                        <a
                            href="#"
                            class="flex items-center p-2 text-sm text-gray-700 rounded-lg dark:text-white hover:bg-blue-100 dark:hover:bg-blue-700 group"
                        >
                            <span class="i-carbon-intent-request-create"></span>
                            <span class="ms-3">"Add Vault"</span>
                        </a>
                    </li>
                </ul>
                <ul class="border-t my-2">
                    <h2 class="font-thin font-sans text-gray-700 pt-1">"Settings"</h2>
                    <li>
                        <a
                            href="/settings/config"
                            class="flex items-center p-2 text-gray-700 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                        >
                            <span class="i-carbon-cloud-satellite-config"></span>
                            <span class="ms-3">"Config"</span>
                        </a>
                        <a
                            href="/settings/plugins"
                            class="flex items-center p-2 text-gray-700 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                        >
                            <span class="i-carbon-plug-filled"></span>
                            <span class="ms-3">"Plugins"</span>
                        </a>
                    </li>
                </ul>
            </div>
        </aside>
    }
}

#[component]
fn MarkdownPage() -> Dom {
    let fut = async {
        let response = Request::get("/api/v1/vaults/Default/file/vault/test.md")
            .build()
            .expect("Failed to build request")
            .send()
            .await
            .map_err(|err| err.to_string())
            .unwrap()
            .text()
            .await;

        let element = window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("content")
            .unwrap();
        element.set_inner_html(&response.unwrap());
    };
    html! {
        <>
            <TabView/>
            <article class="text-base prose prose-truegray container mx-auto p-4 pt-8" use:future={fut} id="content" un-cloak="">
            </article>

        </>
    }
}
#[component]
fn Nav() -> Dom {
    html! {
        <nav
            class="z-50 top-0 fixed w-full bg-white border-b border-gray-200 dark:bg-gray-800 dark:border-gray-700"
            un-cloak=""
        >
            <div class="px-3 py-3 lg:px-5 lg:pl-3">
                <div class="flex items-center justify-between">
                    <div class="flex items-center justify-start rtl:justify-end">
                        <button
                            data-drawer-target="logo-sidebar"
                            data-drawer-toggle="logo-sidebar"
                            aria-controls="logo-sidebar"
                            type="button"
                            class="inline-flex items-center p-2 text-sm text-gray-500 rounded-lg sm:hidden hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-gray-200 dark:text-gray-400 dark:hover:bg-gray-700 dark:focus:ring-gray-600"
                        >
                            <span class="sr-only">"Open sidebar"</span>
                            //www.w3.org/2000/svg">
                            <svg
                                class="w-6 h-6"
                                aria-hidden="true"
                                fill="currentColor"
                                viewBox="0 0 20 20"
                                //www.w3.org/2000/svg"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path
                                    clip-rule="evenodd"
                                    fill-rule="evenodd"
                                    d="M2 4.75A.75.75 0 012.75 4h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 4.75zm0 10.5a.75.75 0 01.75-.75h7.5a.75.75 0 010 1.5h-7.5a.75.75 0 01-.75-.75zM2 10a.75.75 0 01.75-.75h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 10z"
                                ></path>
                            </svg>
                        </button>
                        <div class="flex items-center">
                            <Logo/>
                            <h1 class="font-extrabold text-2xl">"Kasuku"</h1>
                        </div>
                    </div>
                    <div class="flex items-center">
                        <div class="flex items-center ms-3">
                            <div>
                                <button
                                    type="button"
                                    class="flex text-sm bg-gray-800 rounded-full focus:ring-4 focus:ring-gray-300 dark:focus:ring-gray-600"
                                    aria-expanded="false"
                                    data-dropdown-toggle="dropdown-user"
                                >
                                    <span class="sr-only">"Open user menu"</span>
                                    <img class="w-8 h-8 rounded-full" alt="user photo"/>
                                </button>
                            </div>
                            <div
                                class="z-50 hidden my-4 text-base list-none bg-white divide-y divide-gray-100 rounded shadow dark:bg-gray-700 dark:divide-gray-600"
                                id="dropdown-user"
                            >
                                <div class="px-4 py-3" role="none">
                                    <p class="text-sm text-gray-900 dark:text-white" role="none">
                                        "Neil Sims"
                                    </p>
                                    <p
                                        class="text-sm font-medium text-gray-900 truncate dark:text-gray-300"
                                        role="none"
                                    >
                                        "neil.sims@flowbite.com"
                                    </p>
                                </div>
                                <ul class="py-1" role="none">
                                    <li>
                                        <a
                                            href="#"
                                            class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-600 dark:hover:text-white"
                                            role="menuitem"
                                        >
                                            "Dashboard"
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="#"
                                            class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-600 dark:hover:text-white"
                                            role="menuitem"
                                        >
                                            "Settings"
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="#"
                                            class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-600 dark:hover:text-white"
                                            role="menuitem"
                                        >
                                            "Earnings"
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="#"
                                            class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-600 dark:hover:text-white"
                                            role="menuitem"
                                        >
                                            "Sign out"
                                        </a>
                                    </li>
                                </ul>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </nav>
    }
}

fn home(_: &App<()>) -> Dom {
    html! {
        <>
            <Nav/>
            <div class="grid grid-cols-6">
            <div class="col-span-1 min-width-48 bg-gray-50 dark:bg-gray-800 h-screen overflow-scroll pt-10">
                <SideBar/>
            </div>
            <div class="col-span-5 flex h-screen">
                <div class="flex-1 overflow-scroll pt-14">
                    <MarkdownPage/>
                </div>
            </div>
            </div>
        </>
    }
}
fn main() {
    let mut app = App::new(());
    app.route("/", home);
    // app.route("/vault/:vault/:file", home); // View a specific file
    // app.route("/quick/:plugin/:view", home); // Render a specific quick view
    // app.route("/plugins", home); // View plugins
    // app.route("/plugins/:plugin", home); // View specific plugin
    // app.route("/plugins/:plugin/config", home); // Config specific plugin

    let parent_node = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("app")
        .unwrap();

    app.mount_to(&parent_node);

    // std::mem::forget(root);
    // routes:
    // / -> home
    // /plugins -> list plugins
    // /markdown/documents/notes.md
    // /tasks/documents/2023-
    // /recipes/
    // /entertainment/
    // /new?type=md&base=/recipes/
    //
    // let app =
    // /tasks/overview              Get overview of overdue, starred, today's, and tomorrow's tasks.
    // /tasks/overdue               Search *.md files for tasks due on previous dates.  (@due(YYYY-MM-DD) format.)
    // /tasks/process               Move lines from $MARKDO_INBOX to other files, one at a time.
    // /tasks/tag "string"          Search *.md files for @string.
    // /tasks/today                 Search *.md files for tasks due today.  (@due(YYYY-MM-DD) format.)
    // /tasks/tomorrow              Search *.md files for tasks due tomorrow.  (@due(YYYY-MM-DD) format.)
    // /tasks/star, starred         Search *.md files for @star.
    // /tasks/summary               Display counts.
    // /tasks/query, q "string"     Search *.md files for string.
    // /tasks/week                  Search *.md files for due dates in the next week.  (@due(YYYY-MM-DD) format.)
}
