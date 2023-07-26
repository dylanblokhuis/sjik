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
            class: "bg-slate-400 w-40 h-40 border-2 rounded-50 border-blue-100 text-blue-100",
            "Lorem Ipsum Dolor Sit amet!"
          }
        }
      }
    })
}
