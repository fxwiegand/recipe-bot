use log::warn;
use regex::Regex;
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

                let fridge_contains = Regex::new(r"^*contains\s(\s*(and)?[a-z]*,?)*.*$").unwrap();

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
                                let resp_content =
                                    obtain_information(resp.as_array().unwrap().to_vec());

                                format!("What about some {} today. {} You can find the full recipe here: {}.", resp_content.get("dish").unwrap(), resp_content.get("summary").unwrap(), resp_content.get("source_url").unwrap())
                            } else {
                                "I couldn't find any recipe based on your given tags. Maybe ask for a more general recipe.".to_string()
                            }
                        }
                        Err(e) => {
                            warn!("{}", e.to_string());
                            "It seems like something went wrong. I am really sorry.".to_string()
                        }
                    }
                } else if fridge_contains.is_match(&data.to_lowercase()) {
                    let matches = fridge_contains
                        .find(&data.to_lowercase())
                        .unwrap()
                        .as_str()
                        .to_string();
                    let items: Vec<_> = matches
                        .trim_start_matches("contains ")
                        .split_whitespace()
                        .map(|item| item.trim_matches(',').trim_matches('.'))
                        .filter(|item| item != &"and")
                        .collect();

                    dbg!(items.clone());

                    let client = reqwest::Client::new();
                    let mut query = HashMap::new();
                    query.insert("ingredients", items.join(","));
                    query.insert("number", "1".to_string());

                    match client
                        .get("https://api.spoonacular.com/recipes/findByIngredients")
                        .query(&[("apiKey", api_key.clone())])
                        .query(&query)
                        .send()
                        .await?
                        .json::<Value>()
                        .await
                    {
                        Ok(resp) => {
                            if !resp.as_array().unwrap().is_empty() {
                                let id = resp.as_array().unwrap()[0]
                                    .get("id")
                                    .unwrap()
                                    .as_i64()
                                    .unwrap();

                                match client
                                    .get(
                                        &(String::from("https://api.spoonacular.com/recipes/")
                                            + &id.to_string()
                                            + "/information"),
                                    )
                                    .query(&[("apiKey", api_key.clone())])
                                    .send()
                                    .await?
                                    .json::<Value>()
                                    .await
                                {
                                    Ok(s_res) => {
                                        let resp_content = obtain_information(vec![s_res]);

                                        format!("You could make some {} today. {} The full recipe is right here: {}.", resp_content.get("dish").unwrap(), resp_content.get("summary").unwrap(), resp_content.get("source_url").unwrap())
                                    }
                                    Err(e) => {
                                        warn!("{}", e.to_string());
                                        "It seems like something went wrong. I am really sorry."
                                            .to_string()
                                    }
                                }
                            } else {
                                "I couldn't find any recipe based on your given ingredients. Maybe ask with less ingredients or just ask for a random recipe.".to_string()
                            }
                        }
                        Err(e) => {
                            warn!("{}", e.to_string());
                            "It seems like something went wrong. I am really sorry.".to_string()
                        }
                    }
                } else {
                    "I can not quite understand you. Try asking me for a random recipe or tell me whats left in your fridge.".to_string()
                };

                // Answer message with recipe.
                api.send(message.text_reply(format!("Hello {}! {}", recipent, answer)))
                    .await?;
            }
        }
    }
    Ok(())
}

fn obtain_information(resp: Vec<Value>) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut dish = resp[0].get("title").unwrap().to_string();
    dish = dish.trim_matches('\"').to_string();
    result.insert("dish".to_string(), dish);
    let summary = strip::strip_tags(
        &resp[0]
            .get("summary")
            .unwrap()
            .to_string()
            .trim_matches('\"'),
    );
    result.insert("summary".to_string(), summary);
    let mut source_url = resp[0].get("sourceUrl").unwrap().to_string();
    source_url = source_url.trim_matches('\"').to_string();
    result.insert("source_url".to_string(), source_url);

    result
}
