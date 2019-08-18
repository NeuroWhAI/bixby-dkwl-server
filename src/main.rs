#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate rocket;
extern crate reqwest;
extern crate serde_json;

use {
    std::{
        env,
        thread,
        time::Duration,
        sync::RwLock,
    },
    serde_json::{
        json,
        Value as JsonValue,
    },
};

lazy_static! {
    static ref API_KEY: String = env::var("API_KEY").expect("API key required");
    static ref RECENT_LIST_CACHE: RwLock<String> = RwLock::new(String::new());
}

#[get("/")]
fn index() -> &'static str {
    "ㅇㅇㅈ! ㅇㅇㅈ! ㅌㅋㄴ! ㅌㅋㄴ!"
}

#[get("/recent")]
fn get_recent_list() -> String {
    RECENT_LIST_CACHE.read().unwrap().clone()
}

fn main() {
    update_recent_list()
        .expect("Couldn't init the recent list");

    println!("Open server!");

    thread::spawn(|| -> ! {
        loop {
            // 1번 Search API 호출하면 102 쓰고 하루 API 할당량 10000이므로
            // 여유있게 계산해서 나온 시간마다 갱신.
            thread::sleep(Duration::from_secs(960));

            match update_recent_list() {
                Err(err) => eprintln!("Error: {}", err),
                _ => ()
            }
        }
    });

    rocket::ignite()
        .mount("/", routes![
            index,
            get_recent_list,
        ])
        .launch();
}

fn update_recent_list() -> Result<(), String> {
    let api = "https://www.googleapis.com/youtube/v3/search";
    let channel_id = "UCmMxEFwIOMGGoThkmtZZOvQ";

    let result = reqwest::get(&format!("{}?key={}&part=snippet&channelId={}&order=date&maxResults=3",
        api, *API_KEY, channel_id))
        .and_then(|mut res| res.text())
        .map_err(|err| err.to_string())
        .and_then(|text| {
            // Get video list.
            serde_json::from_str::<JsonValue>(&text)
                .map_err(|err| err.to_string())
                .and_then(|val| {
                    val.get("items")
                    .and_then(|val| val.as_array())
                    .map(|arr| arr.clone())
                    .ok_or("There is no 'items' property".into())
                })
        })
        .map(|items| items.iter()
            .map(|val| {
                let video_id = val.get("id")
                    .and_then(|id| id.get("videoId"));
                let time = val.get("snippet")
                    .and_then(|snip| snip.get("publishedAt"));
                let title = val.get("snippet")
                    .and_then(|snip| snip.get("title"));
                let img_url = val.get("snippet")
                    .and_then(|snip| snip.get("thumbnails"))
                    .and_then(|thumb| thumb.get("high"))
                    .and_then(|high| high.get("url"));

                match (video_id, time, title, img_url) {
                    (Some(video_id), Some(time), Some(title), Some(img_url)) => {
                        Some((video_id, time, title, img_url))
                    },
                    _ => None
                }
            })
            .filter(|opt| opt.is_some())
            .map(|opt| {
                let (video_id, time, title, img_url) = opt.unwrap();
                let parsing = (
                    video_id.as_str(),
                    time.as_str(),
                    title.as_str(),
                    img_url.as_str(),
                );

                match parsing {
                    (Some(video_id), Some(time), Some(title), Some(img_url)) => {
                        Some(json!({
                            "id": video_id,
                            "time": time,
                            "title": title,
                            "img_url": img_url,
                        }))
                    },
                    _ => None
                }
            })
            .filter(|opt| opt.is_some())
            .map(|opt| opt.unwrap())
            .collect::<Vec<JsonValue>>())
        .and_then(|list|
            if list.len() > 0 {
                Ok(json!({
                    "size": list.len(),
                    "list": list,
                }))
            }
            else {
                Err("There is no video".into())
            });

    match result {
        Ok(data) => {
            *RECENT_LIST_CACHE.write().unwrap() = data.to_string();
            Ok(())
        },
        Err(err) => Err(err),
    }
}
