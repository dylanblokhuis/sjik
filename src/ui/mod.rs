use std::ops::Add;

use dioxus::prelude::*;

pub fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 500);

    cx.render(rsx! {
      div {
        class: "w-{count} h-600 bg-black flex flex-col",


          div {
            class: "bg-slate-400 border-4 rounded-500 border-blue-100 text-black w-200 h-200 p-10",
            onclick: move |_| count.modify(|v| {v.add(10)}),
          }

          div {
            class: "text-red-500",
            onclick: move |_| count.modify(|v| {v.add(100)}),
            "Test kinda crazy how this just works bro"
          }

          div {
            class: "flex justify-between bg-white rounded-500",
            img {
              class: "h-100 w-200",
              src: "test-1.png"
            }
            img {
              class: "h-100 w-200",
              src: "test-2.jpg"
            }
          }
      }
    })
}
