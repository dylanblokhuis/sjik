use std::ops::{Add, Sub};

use dioxus::prelude::*;

pub fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 3);

    cx.render(rsx! {
      div {
        class: "w-500 h-600 bg-black flex flex-col",


          div {
            onclick: move |_| count.modify(|v| {v.add(1)}),
            "Add image"
          }

          div {
            onclick: move |_| count.modify(|v| {v.sub(1)}),
            "Remove image"
          }

          div {
            class: "text-red-500",
            "Test kinda crazy how this just works"
          }

          div {
            class: "flex justify-between bg-white rounded-500",

            (0..*count.get()).map(|_| rsx! {
              img {
                class: "h-100 w-200",
                src: "test-1.png"
              }
            })

          }
      }
    })
}
