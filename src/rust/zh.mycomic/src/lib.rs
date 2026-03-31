#![no_std]
extern crate alloc;

use aidoku::{
	error::Result,
	helpers::{substring::Substring, uri::encode_uri},
	prelude::*,
	std::{
		json,
		net::{HttpMethod, Request},
		String, Vec,
	},
	Chapter, Filter, FilterType, Manga, MangaContentRating, MangaPageResult, MangaStatus,
	MangaViewer, Page,
};
use alloc::string::ToString;
use regex::Regex;

const WWW_URL: &str = "https://mycomic.com/cn";

const FILTER_TAG: [&str; 39] = [
	"",
	"mohuan",
	"mofa",
	"rexue",
	"maoxian",
	"xuanyi",
	"zhentan",
	"aiqing",
	"xiaoyuan",
	"gaoxiao",
	"sige",
	"kehuan",
	"shengui",
	"wudao",
	"yinyue",
	"baihe",
	"hougong",
	"jizhan",
	"gedou",
	"kongbu",
	"mengxi",
	"wuxia",
	"shehui",
	"lishi",
	"danmei",
	"lizhi",
	"zhichang",
	"shenghuo",
	"zhiyu",
	"weiniang",
	"heidao",
	"zhanzheng",
	"jingji",
	"tiyu",
	"meishi",
	"funv",
	"zhainan",
	"tuili",
	"zazhi",
];
const FILTER_COUNTRY: [&str; 7] = ["", "japan", "hongkong", "europe", "china", "korea", "other"];
const FILTER_AUDIENCE: [&str; 6] = ["", "shaonv", "shaonian", "qingnian", "ertong", "tongyong"];
const FILTER_YEAR: [&str; 21] = [
	"", "2025", "2024", "2023", "2022", "2021", "2020", "2019", "2018", "2017", "2016", "2015",
	"2014", "2013", "2012", "2011", "2010", "200x", "199x", "198x", "197x",
];
const FILTER_END: [&str; 3] = ["", "0", "1"];
const FILTER_SORT: [&str; 3] = ["", "update", "views"];

fn extract_chapter_number(title: &str) -> Option<f32> {
	let re = Regex::new(r"(?:第\s*)(\d+(?:\.\d+)?)|(\d+(?:\.\d+)?)\s*(?:话|話|章|回|卷|册|冊)").unwrap();
	if let Some(captures) = re.captures(title) {
		let num_match = captures.get(1).or_else(|| captures.get(2));
		if let Some(num_match) = num_match {
			if let Ok(num) = num_match.as_str().parse::<f32>() {
				return Some(num);
			}
		}
	}
	None
}

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
	let mut query = String::new();
	let mut tag = String::new();
	let mut country = String::new();
	let mut audience = String::new();
	let mut year = String::new();
	let mut end = String::new();
	let mut sort = String::new();

	for filter in filters {
		match filter.kind {
			FilterType::Title => {
				query = filter.value.as_string()?.read();
			}
			FilterType::Select => {
				let index = filter.value.as_int()? as usize;
				match filter.name.as_str() {
					"类型" => {
						tag = FILTER_TAG[index].to_string();
					}
					"地区" => {
						country = FILTER_COUNTRY[index].to_string();
					}
					"受众" => {
						audience = FILTER_AUDIENCE[index].to_string();
					}
					"年份" => {
						year = FILTER_YEAR[index].to_string();
					}
					"进度" => {
						end = FILTER_END[index].to_string();
					}
					_ => continue,
				}
			}
			FilterType::Sort => {
				let value = match filter.value.as_object() {
					Ok(value) => value,
					Err(_) => continue,
				};
				let index = value.get("index").as_int()? as usize;
				let ascending = value.get("ascending").as_bool()?;
				sort = FILTER_SORT[index].to_string();

				if sort.is_empty() && ascending {
					sort.push_str("time");
				} else if !sort.is_empty() && !ascending {
					sort = format!("-{}", sort)
				}
			}
			_ => continue,
		}
	}

	let url = if query.is_empty() {
		format!(
			"{}/comics?filter[tag]={}&filter[country]={}&filter[audience]={}&filter[year]={}&filter[end]={}&sort={}&page={}",
			WWW_URL,
			tag,
			country,
			audience,
			year,
			end,
			sort,
			page
		)
	} else {
		format!(
			"{}/comics?q={}&page={}",
			WWW_URL,
			encode_uri(query.clone()),
			page
		)
	};
	let html = Request::new(url, HttpMethod::Get).html()?;
	let has_more = true;
	let mut mangas: Vec<Manga> = Vec::new();

	for item in html.select(".group").array() {
		let item = match item.as_node() {
			Ok(node) => node,
			Err(_) => continue,
		};
		let id = item
			.select("a")
			.attr("href")
			.read()
			.split("/")
			.map(|a| a.to_string())
			.filter(|a| !a.is_empty())
			.collect::<Vec<String>>()
			.pop()
			.unwrap();
		let img = item.select("a>img");
		let mut cover = img.attr("data-src").read();
		if cover.is_empty() {
			cover = img.attr("src").read();
		}
		let title = img.attr("alt").read();
		mangas.push(Manga {
			id,
			cover,
			title,
			..Default::default()
		});
	}

	Ok(MangaPageResult {
		manga: mangas,
		has_more,
	})
}

#[get_manga_details]
fn get_manga_details(id: String) -> Result<Manga> {
	let url = format!("{}/comics/{}", WWW_URL, id.clone());
	let html = Request::new(url.clone(), HttpMethod::Get).html()?;
	let cdn_url = html.select("body[x-data]").attr("x-data").read();
	let cdn_base = if let Some(start) = cdn_url.find("cdnUrl: '") {
		if let Some(end) = cdn_url[start + 9..].find("'") {
			format!("https://{}", &cdn_url[start + 9..start + 9 + end])
		} else {
			"https://biccam.com".to_string()
		}
	} else {
		"https://biccam.com".to_string()
	};

	let mut cover = html.select("meta[name='og:image']").attr("content").read();
	if cover.is_empty() {
		cover = html.select("img.object-cover").attr("src").read();
	}
	if !cover.is_empty() && !cover.starts_with("http") {
		if cover.starts_with("//") {
			cover = format!("https:{}", cover);
		} else if cover.starts_with("/") {
			cover = format!("{}{}", cdn_base, cover);
		}
	}
	let title = html
		.select("title")
		.text()
		.read()
		.replace(" - MYCOMIC - 我的漫画", "");
	let author = html.select("meta[name='author']").attr("content").read();
	let artist = String::new();
	let mut description = html
		.select("div[x-show='show']")
		.text()
		.read()
		.trim()
		.to_string();
	if description.is_empty() {
		description = html
			.select("meta[name='description']")
			.attr("content")
			.read()
			.trim()
			.to_string();
	}
	let categories = html
		.select("a[href*='tag']")
		.array()
		.map(|a| a.as_node().unwrap().text().read())
		.collect::<Vec<String>>();
	let status = match html.select("div[data-flux-badge]").text().read().trim() {
		"连载中" => MangaStatus::Ongoing,
		"已完结" => MangaStatus::Completed,
		_ => MangaStatus::Unknown,
	};
	let nsfw = MangaContentRating::Safe;
	let viewer = MangaViewer::Rtl;

	Ok(Manga {
		id,
		cover,
		title,
		author,
		artist,
		description,
		url,
		categories,
		status,
		nsfw,
		viewer,
	})
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
	let url = format!("{}/comics/{}", WWW_URL, id.clone());
	let html = Request::new(url.clone(), HttpMethod::Get).html()?;
	let mut all_chapters: Vec<Chapter> = Vec::new();

	for element in html.select("div[x-data*='chapters']").array() {
		let element = match element.as_node() {
			Ok(node) => node,
			Err(_) => continue,
		};

		let scanlator = element
			.select("div[data-flux-subheading] div")
			.first()
			.text()
			.read()
			.trim()
			.to_string();

		let data = element.attr("x-data").read();
		let mut text = match data.substring_after("chapters:") {
			Some(t) => t,
			None => continue,
		};
		text = match text.substring_before("],") {
			Some(t) => t,
			None => continue,
		};
		let text = text.trim().to_string() + "]";
		let data = match json::parse(&text) {
			Ok(d) => d,
			Err(_) => continue,
		};
		let list = match data.as_array() {
			Ok(l) => l,
			Err(_) => continue,
		};
		let len = list.len();

		for (index, item) in list.enumerate() {
			let item = match item.as_object() {
				Ok(item) => item,
				Err(_) => continue,
			};
			let chapter_id = item.get("id").as_int().unwrap().to_string();
			let title = item.get("title").as_string().unwrap().read();

			let chapter_num = (len - index) as f32;
			let chapter_or_volume = extract_chapter_number(&title).unwrap_or(chapter_num);
			let (ch, vo) = if title.trim().ends_with('卷') {
				(-1.0, chapter_or_volume)
			} else {
				(chapter_or_volume, -1.0)
			};

			let chapter_url = format!("{}/chapters/{}", WWW_URL, chapter_id);
			all_chapters.push(Chapter {
				id: chapter_id,
				title,
				volume: vo,
				chapter: ch,
				url: chapter_url,
				scanlator: scanlator.clone(),
				..Default::default()
			});
		}
	}

	Ok(all_chapters)
}

#[get_page_list]
fn get_page_list(_: String, chapter_id: String) -> Result<Vec<Page>> {
	let url = format!("{}/chapters/{}", WWW_URL, chapter_id.clone());
	let html = Request::new(url.clone(), HttpMethod::Get).html()?;
	let mut pages: Vec<Page> = Vec::new();

	for (index, item) in html.select("img.page").array().enumerate() {
		let item = match item.as_node() {
			Ok(node) => node,
			Err(_) => continue,
		};
		let index = index as i32;
		let url = if item.has_attr("data-src") {
			item.attr("data-src").read()
		} else {
			item.attr("src").read()
		};
		pages.push(Page {
			index,
			url,
			..Default::default()
		})
	}

	Ok(pages)
}

#[modify_image_request]
fn modify_image_request(request: Request) {
	request.header("Referer", WWW_URL);
}
