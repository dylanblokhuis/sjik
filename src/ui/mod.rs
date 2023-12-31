use dioxus::prelude::*;
use dioxus_beuk::hooks::{animation::Animation, use_animation};

use crate::{decoder::MediaCommands, AppContextRef};

pub fn app(cx: Scope) -> Element {
    let ctx = use_context::<AppContextRef>(cx).unwrap();
    let animation = use_animation(cx, 0.0);
    let progress = animation.value();

    use_effect(cx, (&progress,), move |(val,)| {
        if val == 100.0 {
            animation.start(Animation::new_linear(100.0..=0.0, 1000));
        }

        if val == 0.0 {
            animation.start(Animation::new_linear(0.0..=100.0, 1000));
        }
        async move {}
    });

    cx.render(rsx! {
      div {
        class: "w-full h-full bg-transparent flex flex-col justify-end",

          // div {
          //   onclick: move |_| count.modify(|v| {v.add(1)}),
          //   "Add image"
          // }

          // div {
          //   onclick: move |_| count.modify(|v| {v.sub(1)}),
          //   "Remove image"
          // }


          div {
            class: "flex bg-white/50 h-100 flex-col",

            div {
              class: "bg-white/30 h-5",

              div {
                class: "bg-sky-500 h-5 w-{progress}%",
              }
            }

            div {
              class: "justify-center pt-10 gap-x-10",

              div {
                class: "bg-white/50 h-64 px-20 flex items-center justify-center text-sky-900 rounded-5 hover:bg-white/60",
                onclick: move |_| {
                  ctx.read().unwrap().command_sender.as_ref().unwrap().send(MediaCommands::Play).unwrap();
                },

                span {
                  "Play"
                }
              }

              div {
                class: "bg-white/50 h-64 px-20 flex items-center justify-center text-sky-900 rounded-5 hover:bg-white/60",
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
                class: "bg-white/50 h-64 px-20 flex items-center justify-center text-sky-900 rounded-5 hover:bg-white/60",
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
