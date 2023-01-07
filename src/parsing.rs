use core::fmt;

use reqwest::StatusCode;
use scraper::Selector;
use serde::{Deserialize, Serialize};

use crate::caching::{cache_objects, decache_objects};

/**
 **One value must be greater than**
 */
const MAX_SERIES: u8 = 9;
const URL_SERIES: &str = "https://scpfoundation.net/scp-series";
const URL_SCP_OBJECT_PAGE: &str = "https://scpfoundation.net/scp-";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ClassificationScp {
    None,
    Safe,
    Euclid,
    Keter,
    Thaumiel,
    Neutralized,
    NonStandard,
}

impl fmt::Display for ClassificationScp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClassificationScp::Euclid => write!(f, "Евклид"),
            ClassificationScp::Keter => write!(f, "Кетер"),
            ClassificationScp::Neutralized => write!(f, "Нейтрализован"),
            ClassificationScp::None => write!(f, "Отсутствует"),
            ClassificationScp::Safe => write!(f, "Безопасный"),
            ClassificationScp::Thaumiel => write!(f, "Таумиэль"),
            ClassificationScp::NonStandard => write!(f, "Нестандартный класс"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScpObject {
    class: ClassificationScp,
    name: String,
    id: String,
}

impl ScpObject {
    fn new(class: ClassificationScp, name: String, id: String) -> Self {
        ScpObject { class, id, name }
    }

    pub fn get_document_name(&self) -> String {
        String::from(format!("SCP-{}", self.id))
    }

    pub fn get_name(&self) -> String {
        let r = format!("{}", &self.name);
        r.clone()
    }

    pub fn get_class(&self) -> &ClassificationScp {
        &self.class
    }

    pub fn get_id(&self) -> String {
        let r = format!("{}", &self.id);
        r.clone()
    }
}

pub async fn parse_all() -> Vec<ScpObject> {
    let mut objects: Vec<ScpObject> = Vec::new();

    match decache_objects() {
        Ok(o) => o,
        Err(_) => {
            objects.append(&mut parse_series(URL_SERIES).await);

            for i in 2..MAX_SERIES {
                objects.append(&mut parse_series(format!("{}-{}", URL_SERIES, i).as_str()).await);
            }

            cache_objects(objects.clone());

            objects
        }
    }
}

pub async fn parse_series(url: &str) -> Vec<ScpObject> {
    let mut objects: Vec<ScpObject> = Vec::new();

    let response = reqwest::get(url).await.unwrap().text().await.unwrap();

    // let response = fs::read_to_string(URL_SERIES).expect("Should have been able to read the file");
    let document = scraper::Html::parse_document(&response);

    let paragraph_selector: Selector = scraper::Selector::parse("#page-content>p").unwrap();
    let paragraphs = document.select(&paragraph_selector).map(|x| x);

    paragraphs.zip(1..101).for_each(|(i, _)| {
        let childrens: Vec<_> = i
            .children()
            .filter(|c| {
                c.value().is_element()
                    && c.value().as_element().unwrap().name() == "a"
                    && c.children().next().unwrap().value().as_text().is_some()
                    && c.children()
                        .next()
                        .unwrap()
                        .value()
                        .as_text()
                        .unwrap()
                        .starts_with("SCP")
            })
            .map(|e| e)
            .collect();

        childrens.iter().for_each(|c| {
            let name_d = c.first_child().unwrap().value().as_text().unwrap();
            let name_d: Vec<_> = name_d.split("-").collect();

            let id = name_d.get(1).unwrap().trim();

            if c.next_sibling().unwrap().value().is_text()
                || c.next_sibling().unwrap().value().is_element()
                    && c.next_sibling()
                        .unwrap()
                        .value()
                        .as_element()
                        .unwrap()
                        .name()
                        == "span"
            {
                let span = c.next_sibling().unwrap().next_sibling();

                let mut name: Option<&str> = None;

                if !span.is_none() && span.unwrap().value().is_element() {
                    let elm = span.unwrap().value().as_element().unwrap();
                    if elm.name() == "span" {
                        let text_from_span = span.unwrap().children().next();
                        if !text_from_span.is_none() && text_from_span.unwrap().value().is_text() {
                            name = Some(text_from_span.unwrap().value().as_text().unwrap().trim());
                        }
                    }
                }

                if name == None {
                    name = Some(
                        c.next_sibling()
                            .unwrap()
                            .value()
                            .as_text()
                            .unwrap()
                            .trim()
                            .strip_prefix("—")
                            .unwrap_or("NOT FOUND")
                            .trim(),
                    );
                }

                let this = c
                    .prev_sibling()
                    .unwrap()
                    .prev_sibling()
                    .unwrap()
                    .value()
                    .as_element()
                    .unwrap()
                    .attr("alt");

                let class: ClassificationScp = match this {
                    Some(val) => match val {
                        "na.png" => ClassificationScp::Neutralized,
                        "safe.png" => ClassificationScp::Safe,
                        "euclid.png" => ClassificationScp::Euclid,
                        "keter.png" => ClassificationScp::Keter,
                        "thaumiel.png" => ClassificationScp::Thaumiel,
                        "nonstandard.png" => ClassificationScp::NonStandard,
                        _ => ClassificationScp::None,
                    },
                    None => ClassificationScp::None,
                };

                objects.push(ScpObject::new(
                    class,
                    name.unwrap().to_string(),
                    id.to_string(),
                ));
            }
        });
    });

    objects
}

pub async fn debug() {
    let r = parse_object_page("3042").await;

    println!("{:?}", r);
}

// TODO: Сделать парсинг страницы объекта

pub async fn parse_object_page(id: &str) -> Option<String> {
    let path = format!("{}{}", URL_SCP_OBJECT_PAGE, id);

    let response = reqwest::get(path).await;

    match response {
        Ok(r) => {
            if r.status() == StatusCode::OK {
                return Some(r.text().await.unwrap().to_string());
            }

            None
        }
        Err(_) => None,
    }
}
