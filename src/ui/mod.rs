use std::{
    ops::{Add, Sub},
    rc::Rc,
};

use dioxus::prelude::*;
use dioxus_beuk::DomContext;

pub fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 3);

    let ctx = use_context::<Rc<DomContext>>(cx).unwrap();
    let window_height = ctx.window_size.borrow().height;

    cx.render(rsx! {
      div {
        class: "w-full h-{window_height} bg-transparent flex flex-col justify-end",

          // div {
          //   onclick: move |_| count.modify(|v| {v.add(1)}),
          //   "Add image"
          // }

          // div {
          //   onclick: move |_| count.modify(|v| {v.sub(1)}),
          //   "Remove image"
          // }

          // div {
          //   class: "text-red-500",
          //   "Test kinda crazy how this just works"
          // }

          div {
            class: "flex justify-between bg-black/50 flex-wrap w-full h-100",

            // (0..*count.get()).map(|_| rsx! {
            //   img {
            //     class: "h-100 w-200",
            //     src: "test-1.png"
            //   }
            // })

          }
      }
    })
}
