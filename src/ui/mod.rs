use std::{ops::Add, rc::Rc};

use dioxus::prelude::*;

use crate::{decoder::MediaCommands, AppContextRef};

pub fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 3);

    let ctx = use_context::<AppContextRef>(cx).unwrap();
    let size = ctx.read().unwrap().window_size;
    let window_width = size.width;
    let window_height = size.height;

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
            class: "flex bg-white/50 h-100 flex-col",

            div {
              class: "bg-white/30 h-5"
            }

            div {
              class: "justify-center pt-10 gap-x-10",

              div {
                class: "bg-white/50 h-64 px-20 flex items-center justify-center text-sky-900 rounded-100",
                onclick: move |_| {
                  ctx.read().unwrap().command_sender.as_ref().unwrap().send(MediaCommands::Play).unwrap();
                },

                span {
                  "Play"
                }
              }

              div {
                class: "bg-white/50 h-64 px-20 flex items-center justify-center rounded-100",
                onclick: move |_| {
                  ctx.read().unwrap().command_sender.as_ref().unwrap().send(MediaCommands::Pause).unwrap();
                },

                span {
                  "Pause"
                }
                // div {
                //   class: "h-6 w-6 bg-red-300",
                // }
              }

              div {
                class: "bg-white/50 h-64 px-20 flex items-center justify-center rounded-100",
                onclick: move |_| {
                  ctx.read().unwrap().command_sender.as_ref().unwrap().send(MediaCommands::Pause).unwrap();
                },

                span {
                  "Seek forward"
                }
                // div {
                //   class: "h-6 w-6 bg-red-300",
                // }
              }
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
