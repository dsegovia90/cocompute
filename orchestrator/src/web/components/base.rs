// SPDX-License-Identifier: AGPL-3.0-only

use leptos::prelude::*;
use crate::web::asset_hash;

#[component]
pub fn Base(title: &'static str, children: Children) -> impl IntoView {
    let css_url = asset_hash::CSS.url();
    view! {
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>{title}</title>
                <link rel="stylesheet" href={css_url}/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
                <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&family=Geist+Mono:wght@400&display=swap" rel="stylesheet"/>
            </head>
            <body>
                {children()}
            </body>
        </html>
    }
}
