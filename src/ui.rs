use dioxus::prelude::*;

pub fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 100);

    cx.render(rsx! {
      div {
        width: "{count}",
        height: "100px",
        background: "red",
        onclick: move |_| {
          count.set(count.get() + 10);
        }
      }
      div {
        width: "100px",
        height: "100px",
        background: "blue",
      }
      div {
        width: "100px",
        height: "100px",
        background: "blue",
      }
      div {
        width: "100px",
        height: "100px",
        background: "green",
      }
    })
}
