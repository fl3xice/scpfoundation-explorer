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
const URL_SCP_OBJECT_PAGE: &str = "https://scpfoundation.net/api/articles/scp-";

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

/**
## Example

{"pageId": "scp-002", "title": "SCP-002 - «Живая» комната", "source": "
[[>]]
[[module Rate]]
[[/>]]

[[div_ class="rimg"]]
[[image 800px-SCP002-new.jpg width="300" ]]
[[span]]SCP-002 в месте содержания.[[/span]]
[[/div]]

**Объект №:** SCP-002

**Класс объекта:** [[[euclid |Евклид]]]

**Особые условия содержания:** SCP-002 должен быть постоянно подсоединён к подходящей электросети, чтобы оставаться в состоянии, которое, судя по всему, является режимом подзарядки. В случае отключения электроэнергии аварийный барьер между объектом и комплексом должен быть закрыт, а персонал - немедленно эвакуирован. Как только подача энергии в Зоне будет восстановлена, всю область следует бомбардировать чередующимися лучами рентгеновского и ультрафиолетового излучения, пока SCP-002 вновь не подключится к электросети и не войдёт в режим подзарядки. В Зоне содержания надлежит постоянно поддерживать отрицательное давление воздуха.

В пределах 20 метров от SCP-002 или места его содержания необходимо присутствие команды из двух (2) человек. Они должны поддерживать постоянный физический контакт друг с другом, чтобы удостоверяться в существовании другого человека, поскольку восприятие под влиянием объекта может притупляться, искажаться или претерпевать изменения.

К SCP-002 не допускаются сотрудники с уровнем допуска ниже 3. Данное требование может быть проигнорировано при наличии письменного разрешения от двух (2) сотрудников с 4 уровнем допуска, находящихся вне Зоны. Во время контакта с объектом командный состав с таким разрешением должен сопровождаться как минимум пятью (5) сотрудниками службы безопасности 3 уровня, у которых на время контакта с объектом отзываются все допуски и звания. После контакта командный состав должен быть отвезён на расстояние как минимум 5 километров от SCP-002 и помещен в карантин на семьдесят два (72) часа, а также подвергнут психологическому обследованию. Если состояние сотрудников будет признано удовлетворительным, звания и допуски могут быть возвращены им по истечении времени карантина.

**Описание:** SCP-002 напоминает мясистую опухоль объёмом примерно 60 м³. На одной из сторон находится железный люк, который ведёт внутрь объекта. Интерьер представляет собой стандартную дешёвую комнату небольших размеров. На одной из стен находится окно, которое снаружи не видимо. В комнате находится мебель, которая, как можно выяснить после ближайшего рассмотрения, сделана из костей, волос и других биологических веществ, которые производит человеческое тело. Все исследования показали, что ДНК отдельно взятых вещей не совпадают или фрагментированы.

Подробные сведения об обнаружении объекта приведены в отчёте Мюльхаузена [документ00.023.603].

**Справка:** На сегодняшний день объект SCP-002 является причиной исчезновения семи сотрудников. Также за время нахождения на содержании он обставил себя двумя лампами, ковриком, телевизором, радио, креслом-мешком, тремя книгами на неизвестном языке, четырьмя детскими игрушками и маленьким растением в горшке. Тесты с различными подопытными животными (в том числе и высшими приматами) провалились из-за полного отсутствия реакции SCP-002 на них. На трупы объект также не реагирует. Какой бы процесс ни использовал SCP-002 для преобразования органического материала в мебель, активируется он только присутствием живого человека.

> просмотреть отчёт Мюльхаузена, идентификатор документа: 00.023.603

> Отчёт Мюльхаузена [00.023.603]
>
> Нижеследующее является отчётом об обнаружении SCP-002.
>
> Объект был найден в маленьком кратере в северной Португалии, куда он упал с орбиты Земли. От удара о поверхность толстая каменная оболочка объекта треснула, и показалось её мясистое содержимое. Местный фермер, оказавшийся на месте падения объекта, сообщил о своей находке старосте деревни. Объект привлёк внимание Фонда, когда агент с 4 уровнем допуска, приписанный к данной области, заметил небольшую радиоактивную аномалию, производимую объектом.
>
> Группа сбора во главе с генералом Мюльхаузеном была немедленно отправлена на место происшествия, где объект был быстро найден и помещён в большой контейнер. Также было проведено начальное тестирование с использованием подопытных, рекрутированных в ближайшей деревне. Три человека, отправляемые в объект по одному, исчезли. Узнав о смертельном воздействии объекта, генерал Мюльхаузен немедленно отдал приказ об уничтожении уровня 4а, который подразумевает уничтожение всех свидетелей (примерно ⅓ деревни), чтобы удостовериться, что никакая информация об объекте не станет известна общественности, и начал транспортировку объекта на базу Фонда [ДАННЫЕ УДАЛЕНЫ].
>
> Во время подготовки к перевозке четыре сотрудника службы безопасности были необъяснимым образом затянуты внутрь объекта, где они немедленно исчезли. Последовавший осмотр показал, что объект «отрастил» несколько новых предметов мебели и начал принимать вид жилой комнаты. Генерал Мюльхаузен немедленно отдал приказ о доставке нескольких костюмов класса III HAZMAT для оставшихся сотрудников службы безопасности, которые должны были погрузить контейнер на грузовой корабль для транспортировки на базу Фонда.
>
> [ДАННЫЕ УДАЛЕНЫ]
>
> [ДАННЫЕ УДАЛЕНЫ]
>
> После устранения генерала Мюльхаузена SCP-002 вновь оказался на содержании Фонда и был доставлен в специальное хранилище [ЗАСЕКРЕЧЕНО], где и находится на данный момент. После инцидента Мюльхаузена сотрудникам с уровнем допуска ниже 3 был запрещён доступ к SCP-002 без предварительного разрешения как минимум двух сотрудников с 4 уровнем допуска.

----
= [[size 90%]]**<< [[[SCP-001]]] | SCP-002 | [[[SCP-003]]] >>**[[/size]]
", "tags": ["аномалия:превращение", "класс:евклид", "свойство:локация", "свойство:существо", "структура:объект"], "parent": null, "locked": false}

**/
#[allow(unused)]
#[derive(Deserialize, Debug)]
pub struct ApiObjectResult {
    #[serde(rename(deserialize = "pageId"))]
    pub page_id: String,
    pub title: String,
    /**
    ### This field is a written by FTML
    */
    pub source: String,
    pub tags: Vec<String>,
    pub locked: bool,
}

pub async fn debug() {}

pub async fn parse_object_page(id: &str) -> Option<ApiObjectResult> {
    let path = format!("{}{}", URL_SCP_OBJECT_PAGE, id);

    let response = reqwest::get(path).await;

    match response {
        Ok(r) => {
            if r.status() == StatusCode::OK {
                return Some(serde_json::from_str(&r.text().await.unwrap()).unwrap());
            }

            None
        }
        Err(_) => None,
    }
}
