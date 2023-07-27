use std::ops::Add;

use dioxus::prelude::*;

pub fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 500);

    cx.render(rsx! {
      div {
        class: "w-{count} h-500 bg-red-500",


          div {
            class: "bg-slate-400 border-2 rounded-50 border-blue-100 text-red-100 w-100 h-100 p-10",
            onclick: move |_| count.modify(|v| {v.add(10)}),
          }

          div {
            onclick: move |_| count.modify(|v| {v.add(100)}),
            "Hello world!"
          }

          // img {
          //   class: "w-100 h-100",
          //   src: "https://i.imgur.com/2bYg1hY.jpg"
          // }
      }
    })
}
