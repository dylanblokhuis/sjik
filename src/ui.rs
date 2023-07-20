use crate::ui_render_pass::scratch::*;
use leptos_reactive::*;

pub fn app(cx: Scope, button_width: ReadSignal<u32>, set_button_width: WriteSignal<u32>) {
    div("bg-white p-15 flex-col ")
        .child(
            div("bg-red-200 flex-col")
                .child(div("bg-blue-500 p-15"))
                .child(div("bg-red-500 p-15"))
                .child(div("bg-green-500 p-15"))
                .child(div("bg-red-500 p-15"))
                .child(div("bg-blue-500 p-15").child(div("p-15 bg-black w-full"))),
        )
        .child(btn(
            format!("w-{} h-40 bg-red-500", button_width.get()).as_str(),
            Events::default().on_click(move || {
                set_button_width.set(button_width.get() + 10);
                println!("Clicked {}", button_width.get());
            }),
        ));
}
