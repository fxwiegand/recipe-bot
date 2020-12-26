use log::warn;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use telegram_bot::*;
use tokio::stream::StreamExt;
use voca_rs::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("SPOONACULAR_API_KEY").expect("SPOONACULAR_API_KEY not set");

    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let api = Api::new(token);

    // Fetch new updates via long poll method
    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        // If the received update contains a new message...
        let update = update?;
        if let UpdateKind::Message(message) = update.kind {
            if let MessageKind::Text { ref data, .. } = message.kind {
                let recipent = message.from.first_name.clone();
                let mut tags = vec![
                    "vegetarian",
                    "vegan",
                    "dessert",
                    "keto",
                    "low carb",
                    "soup",
                    "italian",
                    "spanish",
                    "mexican",
                ];
                tags = tags
                    .into_iter()
                    .filter(|t| data.to_lowercase().contains(*t))
                    .collect();

                let answer = if data.to_lowercase().contains("recipe")
                    && data.to_lowercase().contains("random")
                {
                    let client = reqwest::Client::new();
                    let mut query = HashMap::new();
                    query.insert("tags", tags.join(","));
                    query.insert("number", "1".to_string());

                    match client
                        .get("https://api.spoonacular.com/recipes/random")
                        .query(&[("apiKey", api_key.clone())])
                        .query(&query)
                        .send()
                        .await?
                        .json::<Value>()
                        .await
                    {
                        Ok(resp) => {
                            if !resp.get("recipes").unwrap().as_array().unwrap().is_empty() {
                                let dish = resp.get("recipes").unwrap()[0]
                                    .get("title")
                                    .unwrap()
                                    .to_string();
                                let summary = strip::strip_tags(
                                    &resp.get("recipes").unwrap()[0]
                                        .get("summary")
                                        .unwrap()
                                        .to_string()
                                        .trim_start_matches('\"')
                                        .trim_end_matches('\"'),
                                );
                                let source_url = resp.get("recipes").unwrap()[0]
                                    .get("sourceUrl")
                                    .unwrap()
                                    .to_string();

                                format!("What about some {} today. {} You can find the full recipe here: {}.", dish.trim_start_matches('\"').trim_end_matches('\"'), summary, source_url.trim_start_matches('\"').trim_end_matches('\"'))
                            } else {
                                "I couldn't find any recipe based on your given tags. Maybe ask for a more general recipe.".to_string()
                            }
                        }
                        Err(e) => {
                            warn!("{}", e.to_string());
                            "It seems like something went wrong. I am really sorry.".to_string()
                        }
                    }
                } else {
                    "I can not quite understand you. Try asking me for a random recipe.".to_string()
                };

                // Answer message with recipe.
                api.send(message.text_reply(format!("Hello {}! {}", recipent, answer)))
                    .await?;
            }
        }
    }
    Ok(())
}
