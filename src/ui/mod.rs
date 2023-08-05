use std::{
    ops::{Add, Sub},
    rc::Rc,
};

use dioxus::prelude::*;
use dioxus_beuk::DomContext;

pub fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 3);

    let ctx = use_context::<Rc<DomContext>>(cx).unwrap();
    let window_width = ctx.window_size.borrow().width;
    let window_height = ctx.window_size.borrow().height;

    cx.render(rsx! {
      div {
        class: "w-{window_width} h-{window_height} bg-transparent flex flex-col justify-end",

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
            class: "flex bg-white/50 justify-start gap-y-30  items-center h-100 flex-col",

            div {
              class: "w-full bg-white/30 h-5"
            }

            div {
              class: "bg-white/50 rounded-5 p-5 w-40 h-40 flex items-center justify-center",
              onclick: move |_| println!("Clicked"),

              // img {
              //   class: "w-24 h-24",
              //   src: "play.svg",
              // }
            }
            // (0..5).map(|_| {
            //   rsx! {

            //     div {
            //       class: "w-26 h-26 bg-white/50 rounded-10 p-5",
            //       img {
            //         class: "w-24 h-24",
            //         src: "play.svg",
            //       }
            //     }

            //   }
            // })


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
