use dioxus::prelude::*;

pub fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 10);

    println!("{:?}", count.get());

    cx.render(rsx! {
      div {
        class: "w-1280 h-722 flex-col items-center justify-end",

        div {
          class: "bg-slate-500 w-full py-{count} flex-col justify-center items-center",
          onclick: move |_| count.set(count.get() + 1),

          div {
            class: "bg-slate-400 border-2 rounded-50 border-blue-100 text-red-100 p-10",
            "Lorem Ipsum Dolor Sit amet!"
          }

          img {
            class: "w-100 h-100",
            src: "https://i.imgur.com/2bYg1hY.jpg"
          }
        }
      }
    })
}
