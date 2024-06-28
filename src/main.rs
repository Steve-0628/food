use std::fs;

use axum::{
    response::{Html, IntoResponse, Json}, routing::{get, post}, Router
};
use chrono::{Utc, FixedOffset};
use serde_json::{Value, json};
use encoding_rs::EUC_JP;

async fn plain_text() -> impl IntoResponse {
    match fs::read_to_string("index.html") {
        Ok(c) => Html(c),
        Err(e) => Html(e.to_string()),
    }
}

async fn search_jan_from_code(code: String) -> Vec<String> {
    let body = reqwest::Client::new();
    let res = body.post("https://www.janken.jp/gadgets/jan/JanSyohinKensaku.php")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("jan={}", code))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let res = EUC_JP.decode(&res).0;
    let dom = tl::parse(&res, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();
    let elements = dom.get_elements_by_class_name("goodsval");
    let response = elements.map(|element| element.get(parser).unwrap().inner_text(parser).to_string().trim().to_string()).collect::<Vec<String>>();
    response
}

async fn jan_search() -> Json<Value> {
    Json(json!({"data": "test", "res": search_jan_from_code("4987035092216".to_string()).await}))
}

#[derive(serde::Deserialize, serde::Serialize)]
struct AddJanBody {
    jan: String
}
async fn search_jan(Json(payload): Json<AddJanBody>) -> Json<Vec<String>> {
    
    Json(search_jan_from_code(payload.jan).await)
}

async fn record_food(Json(payload): Json<AddJanBody>) -> Json<Value> {
    let product_info = search_jan_from_code(payload.jan).await;
    if product_info.is_empty(){
        Json(json!({"error": "No results found for JAN code"}))
    } else if product_info.len() == 3 {
        let discord_webhook_json = json!(
            {
                "embeds": [
                  {
                    "title": product_info[0],
                    "description": product_info[2],
                    "fields": [],
                    "author": {
                      "name": product_info[1]
                    }
                  }
                ],
                "components": [],
                "actions": {},
                "username": format!("{}", Utc::now().with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).format("%Y/%m/%d %H:%M:%S"))
              }
            );
        let client = reqwest::Client::new();
        let _resp = client.post(
            std::env::var("DISCORD_WEBHOOK_URL").unwrap()
        ).json(&discord_webhook_json).send().await;
        Json(json!(product_info))
    } else {
        Json(json!({"error": "Undefined error occured. "}))    
    }
    // "".to_string()
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect(".env not found");
    let app = Router::new()
        .route("/", get(plain_text))
        .route("/jan", get(jan_search))
        .route("/lookup", post(search_jan))
        .route("/record", post(record_food));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
