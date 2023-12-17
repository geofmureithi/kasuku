use hirola::{dom::Dom, prelude::*};

#[component]
fn TabHeader<'a>(title: &'a str, icon: &'a str) -> Dom {
    html! {
        <li>
            <a
                href="/"
                class="font-light inline-flex items-center justify-center px-3 py-2 border-r-1 border-gray-300 hover:text-gray-600 dark:hover:text-gray-300 group"
            >
                <span class=format!(
                    "i-{icon} w-4 h-4 text-gray-500 group-hover:text-gray-700 dark:text-gray-300 dark:group-hover:text-gray-300 mr-1"
                )></span>

                {title}
                <span class="i-carbon-close invisible group-hover:visible h-4 w-4"></span>
            </a>
        </li>
    }
}

#[component]
pub fn TabView() -> Dom {
    let items = vec![
        ("Dashboard", "ic-sharp-dashboard"),
        ("today.md", "ri-markdown-fill"),
        ("test.pdf", "bi-file-earmark-pdf-fill"),
    ];
    html! {
        <div class="text-sm font-medium text-center text-gray-500 border-b border-gray-200 dark:text-gray-400 dark:border-gray-700 mt-14 ml-64">
            <ul class="flex flex-wrap -mb-px inline-block">
                {for (title, icon) in items {
                    html! {
                        <>
                            <TabHeader title=title icon=icon/>
                        </>
                    }
                }}

            </ul>
        </div>
    }
}
