use leptos::task::spawn_local;
use leptos::{prelude::*, ev::SubmitEvent};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use leptos::web_sys;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_without_args(cmd: &str) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "notification"], js_name = isPermissionGranted)]
    async fn notif_is_permission_granted() -> JsValue;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "notification"], js_name = requestPermission)]
    async fn notif_request_permission() -> JsValue;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "notification"], js_name = sendNotification)]
    fn notif_send(options: JsValue);
}

#[component]
pub fn App() -> impl IntoView {
    let (countdown_minutes, set_countdown_minutes) = signal(String::new());
    let (specific_time, set_specific_time) = signal(String::new());
    let (remaining, set_remaining) = signal(Option::<u64>::None);
    let (status_msg, set_status_msg) = signal(String::new());
    let (now_str, set_now_str) = signal(String::new());

    let update_minutes = move |ev| {
        set_countdown_minutes.set(event_target_value(&ev));
    };
    let update_time = move |ev| {
        set_specific_time.set(event_target_value(&ev));
    };

    let start_countdown = move |ev: SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let mins_str = countdown_minutes.get_untracked();
            if let Ok(mins) = mins_str.parse::<u64>() {
                let args = js_sys::Object::new();
                js_sys::Reflect::set(&args, &JsValue::from_str("minutes"), &JsValue::from_f64(mins as f64)).unwrap();
                let _ = invoke("schedule_countdown", JsValue::from(args)).await;
                set_status_msg.set(format!("已设置: {} 分钟后关机", mins));
            } else {
                set_status_msg.set("请输入合法的分钟数".to_string());
            }
        });
    };

    let start_at_time = move |ev: SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let t = specific_time.get_untracked();
            let parts: Vec<&str> = t.split(':').collect();
            if parts.len() == 2 {
                if let (Ok(h), Ok(m)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    let args = js_sys::Object::new();
                    js_sys::Reflect::set(&args, &JsValue::from_str("hour"), &JsValue::from_f64(h as f64)).unwrap();
                    js_sys::Reflect::set(&args, &JsValue::from_str("minute"), &JsValue::from_f64(m as f64)).unwrap();
                    let _ = invoke("schedule_at", JsValue::from(args)).await;
                    set_status_msg.set(format!("已设置: 指定时间 {:02}:{:02} 关机", h, m));
                } else {
                    set_status_msg.set("请输入合法的时间".to_string());
                }
            } else {
                set_status_msg.set("请输入合法的时间".to_string());
            }
        });
    };

    let cancel = move |_| {
        spawn_local(async move {
            let _ = invoke_without_args("cancel_shutdown").await;
            set_status_msg.set("已取消关机".to_string());
        });
    };

    let window = web_sys::window().unwrap();
    let set_remaining_clone = set_remaining;
    let set_status_clone = set_status_msg;
    let poll = Closure::wrap(Box::new(move || {
        let set_remaining_inner = set_remaining_clone;
        let set_status_inner = set_status_clone;
        spawn_local(async move {
            let res = invoke_without_args("remaining_seconds").await;
            if let Some(obj) = js_sys::Object::try_from(&res) {
                let seconds = js_sys::Reflect::get(obj, &JsValue::from_str("seconds")).ok();
                if let Some(val) = seconds {
                    if val.is_undefined() || val.is_null() {
                        set_remaining_inner.set(None);
                    } else {
                        let s = val.as_f64().unwrap_or(0.0) as u64;
                        if s == 60 {
                            let perm = notif_is_permission_granted().await.as_bool().unwrap_or(false);
                            if !perm {
                                let _ = notif_request_permission().await;
                            }
                            let payload = js_sys::Object::new();
                            js_sys::Reflect::set(&payload, &JsValue::from_str("title"), &JsValue::from_str("关机提醒")).unwrap();
                            js_sys::Reflect::set(&payload, &JsValue::from_str("body"), &JsValue::from_str("还有 1 分钟将自动关机" )).unwrap();
                            notif_send(JsValue::from(payload));
                            set_status_inner.set("还有 1 分钟将自动关机".to_string());
                        }
                        set_remaining_inner.set(Some(s));
                    }
                }
            }
        });
    }) as Box<dyn FnMut()>);
    window.set_interval_with_callback_and_timeout_and_arguments_0(poll.as_ref().unchecked_ref(), 1000).unwrap();
    poll.forget();

    let format_now = || {
        let d = js_sys::Date::new_0();
        let year = d.get_full_year() as u32;
        let month = (d.get_month() as u32) + 1;
        let day = d.get_date() as u32;
        let hour = d.get_hours() as u32;
        let minute = d.get_minutes() as u32;
        let second = d.get_seconds() as u32;
        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second)
    };

    set_now_str.set(format_now());
    let tick_time = Closure::wrap(Box::new(move || {
        set_now_str.set(format_now());
    }) as Box<dyn FnMut()>);
    window
        .set_interval_with_callback_and_timeout_and_arguments_0(tick_time.as_ref().unchecked_ref(), 1000)
        .unwrap();
    tick_time.forget();

    view! {
        <main class="container">
            <section class="page-header">
                <h1 class="page-title">"跨平台定时关机工具"</h1>
                <p class="time-display" id="current-time">{ move || now_str.get() }</p>
            </section>

            <section class="status-row">{ move || status_msg.get() }</section>

            <section class="actions-grid">
                <form class="form-card" on:submit=start_countdown>
                    <input id="minutes-input" placeholder="X分钟后" on:input=update_minutes />
                    <button type="submit">"开始倒计时"</button>
                </form>

                <form class="form-card" on:submit=start_at_time>
                    <input id="time-input" type="time" on:input=update_time />
                    <button type="submit">"按指定时间关机"</button>
                </form>
            </section>

            <div class="row">
                <button on:click=cancel>"取消关机"</button>
            </div>

            <p class="remaining-row">{ move || remaining.get().map(|s| format!("剩余时间: {}分{}秒", s/60, s%60)).unwrap_or_else(|| "未设置关机任务".to_string()) }</p>
        </main>
    }
}
