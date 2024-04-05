#![allow(clippy::needless_return)]

use logger::logger;
use rand::Rng;
use std::io::Write;
use std::collections::HashMap;
use std::path::Path;
use thirtyfour::prelude::*;
use clap::Parser;

#[logger]
async fn type_word(word: String, keys: &HashMap<char, WebElement>) -> Result<(), String> {
	for c in word.chars() {
		let e = keys[&c].click().await;
		if e.is_err() {
			log_return!{ Err(e.unwrap_err().to_string()) };
		}
	}
	keys[&'\n'].click().await.unwrap();
	log_return!{ Ok(()) };

}

macro_rules! aw {
	($s:expr, $tt:block) => {
		$s.block_on(async {
			$tt
		})
	};
}

macro_rules! sleep {
	($ms:expr) => {
		std::thread::sleep(std::time::Duration::from_millis($ms))
	};
}

struct Browser {
	rt: tokio::runtime::Runtime,
	client: Option<Box<WebDriver>>,
	driver_proc: Option<std::process::Child>,
}

#[allow(dead_code)]
impl Browser {
	#[logger]
	fn new() -> Self {
		log_return!{ Self {
			rt: tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap(),
			client: None,
			driver_proc: None
		}};
	}

	#[logger]
	fn get_ready(&mut self, headless: bool) {
		log!("Downloading chromedriver");

		#[cfg(unix)]
		let out: String = String::from_utf8(std::process::Command::new("/tmp/selenium-manager").arg("--browser").arg("chrome").arg("--output").arg("json").output().unwrap().stdout).unwrap();
		#[cfg(not(unix))]
		let out: String = String::from_utf8(std::process::Command::new("./selenium-manager.exe").arg("--browser").arg("chrome").arg("--output").arg("json").output().unwrap().stdout).unwrap();

		let json: serde_json::Value = serde_json::from_str(out.as_str()).unwrap();

		let chromedriver = json["result"]["driver_path"].as_str().unwrap();
		let chrome = json["result"]["browser_path"].as_str().unwrap();

		log!("Starting {}", chromedriver);
		let driver_cmd = std::process::Command::new(chromedriver).arg("--port=4444")
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::null())
			.stdin(std::process::Stdio::null())
			.spawn().unwrap();

		self.driver_proc = Some(driver_cmd);

		let mut caps = DesiredCapabilities::chrome();
		caps.set_binary(chrome).unwrap();

		#[cfg(unix)]
		caps.add_extension(Path::new("/tmp/ublock.crx")).unwrap();
		#[cfg(not(unix))]
		caps.add_extension(Path::new("./ublock.crx")).unwrap();

		if headless {
			caps.set_headless().unwrap();
			caps.add_chrome_arg("--window-size=1920,1080").unwrap();
		}

		aw!(self.rt, {
			self.client = Some(Box::new(WebDriver::new("http://localhost:4444", caps).await.unwrap()));
		});
	}

	async fn find(&self, selector: &str) -> WebElement {
		return self.client.as_ref().unwrap().find(By::Css(selector)).await.unwrap();
	}

	async fn find_all(&self, selector: &str) -> Vec<WebElement> {
		return self.client.as_ref().unwrap().find_all(By::Css(selector)).await.unwrap();
	}

	async fn wait_element(&self, selector: &str) -> WebElement {
		loop {
			if let Ok(o) = self.client.as_ref().unwrap().find(By::Css(selector)).await {
				return o;
			}
		}
	}

	async fn go_to(&self, url: &str) {
		self.client.as_ref().unwrap().goto(url).await.unwrap();
	}

	#[logger]
	async fn screenshot(&self) -> Vec<u8> {
		log_return!{ self.client.as_ref().unwrap().screenshot_as_png().await.unwrap() };
	}

}

impl Drop for Browser {
	fn drop(&mut self) {

		aw!(self.rt, {
			let client = self.client.take().unwrap();
			client.quit().await.unwrap();
		});

		self.driver_proc.as_mut().unwrap().kill().unwrap();

	}
}

#[logger]
fn solve(words: &mut Vec<&String>, headless: bool, ss: Option<String>) -> Option<String> {

	let mut rnd = rand::thread_rng();

	log!("Init browser");
	let mut browser = Browser::new();
	browser.get_ready(headless);

	return aw!(browser.rt, {
		browser.go_to("https://www.nytimes.com/games/wordle/index.html").await;
		browser.wait_element(".Welcome-module_button__ZG0Zh").await.click().await.unwrap();
		browser.wait_element(".Modal-module_closeIcon__TcEKb").await.click().await.unwrap();

		let mut keys: HashMap<char, WebElement> = HashMap::new();

		for elem in browser.find_all(".Keyboard-module_keyboard__uYuqf > div > button").await {
			let letter = elem.attr("data-key").await.unwrap().unwrap();
			match letter.as_str() {
				"←" => {  },
				"↵" => {
					keys.insert('\n', elem);
				},
				_ => {
					keys.insert(letter.chars().next().unwrap(), elem);
				}
			}
		}

		let rows = browser.find_all(".Row-module_row__pwpBq").await;
		for row in rows {
			let word_i = rnd.gen_range(0..words.len());
			let word = words[word_i];
			type_word(word.clone(), &keys).await.unwrap();
			sleep!(3000);

			let letters = row.find_all(By::ClassName("Tile-module_tile__UWEHN")).await.unwrap();
			let mut states: Vec<String> = Vec::new();
			for elem in letters {
				states.push(elem.attr("data-state").await.unwrap().unwrap());
			}

			if states.iter().all(|x| *x == "correct") {
				if let Some(ss) = ss {
					sleep!(5000);
					browser.find(".Modal-module_closeIconButton__y9b6c").await.click().await.unwrap();
					sleep!(500);
					let t = browser.find_all(".Modal-module_closeIconButton__y9b6c").await;
					if !t.is_empty() { t[0].click().await.unwrap(); }
					sleep!(1000);
					std::fs::write(&ss, browser.screenshot().await).unwrap();
				}
				log_return!{ Some(word.clone()) };
			} else {
				words.remove(word_i);
			}

			if states.iter().all(|x| *x == "absent") {
				words.retain(|x| {
					for c in word.chars() {
						if x.contains(c) {
							return false;
						}
					}
					return true;
				});
			}

			for (i, c) in word.chars().enumerate() {
				words.retain(|x| {
					return !(
						(states[i] == "correct" && c != x.chars().nth(i).unwrap()) ||
						(states[i] == "present" && (!x.contains(c) || x.chars().nth(i).unwrap() == c)) ||
						(states[i] == "absent" && x.chars().nth(i).unwrap() == c)
					);
				});
			}

			let mut line = String::new();
			for (i, c) in word.chars().enumerate() {
				line.push(if states[i] == "correct" { c } else { '_' });
			}
			log!("Result: {} {}", word, line);

		}

		log_return!{ None };

	});

}

struct IncludeFile {
	path: std::path::PathBuf,
	did_exist: bool,
}

macro_rules! include_file {
	($dest:expr, $src:literal) => {
		{
			let a = IncludeFile::new($dest);
			std::fs::write($dest, include_bytes!($src)).unwrap();
			a
		}
	};
}

impl IncludeFile {
	fn new(path: &str) -> Self {
		return Self {
			path: std::path::Path::new(path).to_path_buf(),
			did_exist: std::path::Path::new(path).exists(),
		}
	}
}

impl Drop for IncludeFile {
	fn drop(&mut self) {
		if !self.did_exist {
			std::fs::remove_file(&self.path).unwrap();
		}
	}
}

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
	#[arg(long, default_value_t = false, help = "Disable headless mode")]
	not_headless: bool,

	#[arg(short, long, value_name = "PATH", help = "Screenshot path")]
	screenshot: Option<String>,
}

#[logger]
fn main() {

	let args = Args::parse();

	let words_before = include_str!("../words.txt");

	let orig_words: Vec<String> = words_before.split('\n').map(|x| x.replace('\r', "")).collect();
	let mut words: Vec<&String> = orig_words.iter().collect();

	let _ublock: IncludeFile;
	let _manager: IncludeFile;

	if cfg!(unix) {
		log!("Writing to /tmp/");
		_ublock = include_file!("/tmp/ublock.crx", "../ublock.crx");
		_manager = include_file!("/tmp/selenium-manager", "../selenium-manager");
		log!("chmod +x /tmp/selenium-manager: {}", std::process::Command::new("/usr/bin/chmod").arg("+x").arg("/tmp/selenium-manager").status().unwrap());
	} else {
		_ublock = include_file!("ublock.crx", "../ublock.crx");
		_manager = include_file!("selenium-manager.exe", "../selenium-manager.exe");
	}

	loop {
		if let Some(word) = solve(&mut words, !args.not_headless, args.screenshot.clone()) {
			answer!("Correct answer: {}", word);
			break;
		}
	}

}
