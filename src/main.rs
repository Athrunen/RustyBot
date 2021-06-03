#![allow(non_snake_case)]

use thirtyfour::prelude::*;
use teloxide::prelude::*;
use thirtyfour::common::types::WindowHandle;
use tokio;
use serde_json;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use teloxide::types::{ChatId, InputFile};
use std::borrow::Cow;
use rand::Rng;

#[derive(Debug, Clone)]
struct Product<'a> {
  element: WebElement<'a>,
  info: HashMap<String, String>
}

async fn get_products_info(products: &mut HashMap<String, Product<'_>>) -> color_eyre::Result<()> {
  // Price
  // Status
  // Name
  for name in products.clone().keys() {
    let product = products.get_mut(name).unwrap();
    let delivery_info = product.element.find_element(By::Css(".delivery-info")).await?;
    let product_info = product.element.find_element(By::Css(".product-info")).await?;
    product.info.insert(String::from("status"), delivery_info.text().await?);
    product.info.insert(String::from("memory"), String::from(product_info.find_element(By::XPath("//li[contains(text(), 'Speicher:')]")).await?.text().await?.split(":").collect::<Vec<_>>()[1]));
  }
  Ok(())
}

async fn buy_product(driver: &WebDriver, bot: &AutoSend<Bot>, link: &str) -> color_eyre::Result<()> {
  driver.get(link).await?;
  // Add to cart
  driver.find_element(By::Css("a.add-to-cart")).await?.click().await?;
  wait_for_page(&driver, "https://www.alternate.de/addToCart.xhtml").await?;
  tokio::time::sleep(Duration::from_secs(1)).await;
  // Get to cart
  driver.get("https://www.alternate.de/cart.xhtml?t=&q=").await?;
  // Select paypal
  driver.find_element(By::Css("form#express-payments-form a")).await?.click().await?;
  tokio::time::sleep(Duration::from_secs(10)).await;
  if driver.find_element(By::Css("div#gdprCookieBanner")).await?.is_displayed().await? {
    driver.find_element(By::Css("button.gdprCookieBanner_button#acceptAllButton")).await?.click().await?;
  }
  tokio::time::sleep(Duration::from_secs(1)).await;
  // Accept
  send_screenshot(driver, bot).await?;
  driver.find_element(By::XPath("//button[@id='payment-submit-btn' and text()='Weiter']")).await?.click().await?;
  tokio::time::sleep(Duration::from_secs(5)).await;
  send_screenshot(driver, bot).await?;

  // Buy
  //driver.find_element(By::XPath("//input[@class='buyNowBtn' and @value='Jetzt kaufen']")).await?.click().await?;
  //send_screenshot(driver, bot).await?;

  // Empty cart
  driver.get("https://www.alternate.de/cart.xhtml?t=&q=").await?;
  tokio::time::sleep(Duration::from_secs(1)).await;
  if driver.current_url().await?.contains("https://www.alternate.de/checkouterror.xhtml") {
    driver.find_element(By::XPath("//input[@value='Warenkorb leeren']")).await?.click().await?;
  } else {
    driver.find_element(By::Css("form#clear-cart-form > a")).await?.click().await?;
  }
  tokio::time::sleep(Duration::from_secs(1)).await;
  Ok(())
}

async fn get_products<'a>(driver: &'a WebDriver, filters: &Vec<&str>, products: &mut HashMap<String, Product<'a>>) -> color_eyre::Result<()> {
  println!("{:?}", filters);
  let listing = driver.find_elements(By::Css(".listing > .card")).await?;
  'listing: for l in &listing {
    let name = l.find_element(By::Css(".product-name")).await?.text().await?;
    for f in filters {
      if name.to_lowercase().contains(&f.to_lowercase()) == false {
        continue 'listing;
      }
    }
    products.insert(name,  Product{element: l.to_owned(), info: HashMap::new()});
  }
  Ok(())
}

async fn open_page(driver: &WebDriver, pages: &mut HashMap<String, WindowHandle>, name: &str, url: &str) -> color_eyre::Result<()> {
  if pages.len() > 0 {
        driver.execute_script(format!("window.open(\"{}\",\"_blank\");", url).as_str()).await?;
    } else {
        driver.get(url).await?;
    }
  tokio::time::sleep(Duration::from_secs(1)).await;
  let handles = driver.window_handles().await?;
  pages.insert(name.to_string().to_owned(), handles[handles.len() - 1].to_owned());
  driver.switch_to().window(&handles[handles.len() - 1]).await?;
  Ok(())
}

async fn switch_page(driver: &WebDriver, pages: &HashMap<String, WindowHandle>, name: &str) -> color_eyre::Result<()> {
  let wh = &pages.get(name).unwrap();
  driver.switch_to().window(wh).await?;
  tokio::time::sleep(Duration::from_millis(100)).await;

  Ok(())
}

async fn wait_for_page(driver: &WebDriver, url: &str) -> color_eyre::Result<()> {
  loop {
    if driver.current_url().await?.contains(url) {
      break
    }
  }
  Ok(())
}

async fn paypal_login(driver: &WebDriver, bot: &AutoSend<Bot>) -> color_eyre::Result<()> {
  bot.send_message(ChatId::Id(-580425545), "Awaiting login!").await?;
  let mut now = SystemTime::now();
  loop {
    if driver.current_url().await?.contains("https://www.paypal.com/myaccount/summary") {
      break
    }
    if now.elapsed().unwrap_or(Duration::new(0, 0)).as_secs() > 300 {
      bot.send_message(ChatId::Id(-580425545), "Awaiting login!").await?;
      now = SystemTime::now();
    }
  }
  bot.send_message(ChatId::Id(-580425545), "Logged in!").await?;
  Ok(())
}

async fn load_targets(targets: &mut Vec<HashMap<String, String>>) -> color_eyre::Result<()> {
  let tarfile = Path::new("targets.json");
  if tarfile.exists() {
    let jdata: Vec<HashMap<String, String>> = serde_json::from_reader(fs::File::open(tarfile)?)?;
    targets.extend(jdata.to_owned());
  }
  Ok(())
}

async fn save_targets(targets: &mut Vec<HashMap<String, String>>) -> color_eyre::Result<()> {
  let tarfile = Path::new("targets.json");
  if tarfile.exists() {
    serde_json::to_writer(fs::OpenOptions::new().write(true).truncate(true).open(tarfile)?, targets)?;
  }
  Ok(())
}

async fn send_screenshot(driver: &WebDriver, bot: &AutoSend<Bot>) -> color_eyre::Result<()> {
  let screenshot = driver.screenshot_as_png().await?;
  bot.send_video(ChatId::Id(-580425545), InputFile::Memory {
    file_name: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string() + ".png",
    data: Cow::from(screenshot)
  }).await?;
  Ok(())
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
  color_eyre::install()?;

  teloxide::enable_logging!();
  log::info!("Starting BuyBot...");

  let bot = Bot::from_env().auto_send();

  bot.send_message(ChatId::Id(-580425545), "Hello").await?;

  let mut targets: Vec<HashMap<String, String>> = Vec::new();
  load_targets(&mut targets).await?;

  bot.send_message(ChatId::Id(-580425545), format!("Loaded targets: \n{:?}", targets)).await?;

  let caps = DesiredCapabilities::chrome();
  let driver = WebDriver::new("http://localhost:4444/wd/hub", &caps).await?;
  driver.set_implicit_wait_timeout(Duration::new(5, 0)).await?;
  driver.maximize_window().await?;

  let mut pages: HashMap<String, WindowHandle> = HashMap::new();
  
  open_page(&driver, &mut pages, "product", "https://www.alternate.de/html/index.html").await?;

  driver.find_element(By::Css("button.cookie-submit-all")).await?.click().await?;

  open_page(&driver, &mut pages, "search", "https://www.alternate.de/Grafikkarten/NVIDIA-Grafikkarten/html/listings/1486466143032?n=1486466143032&s=default&filter_2195=19&lv=list&lpf=999").await?;

  open_page(&driver, &mut pages, "paypal", "https://www.paypal.com/de/signin").await?;

  //paypal_login(&driver, &bot).await?;

  switch_page(&driver, &pages, "search").await?;

  let mut rng = rand::thread_rng();

  while targets.len() > 0 {

    let index = rng.gen_range(0..targets.len());

    let mut products: HashMap<String, Product> = HashMap::new();

    get_products(&driver, &targets[index]["name"].split(' ').collect::<Vec<_>>(), &mut products).await?;

    get_products_info(&mut products).await?;

    if products.len() > 0 {
      for (name, product) in products.clone() {
        if product.info["status"].contains("Auf Lager") && product.info["memory"].contains(&targets[index]["memory"])/*check_product(&driver, &name, &product)*/ {
          let link = product.element.get_property("href").await?.unwrap();
          switch_page(&driver, &pages, "product").await?;
          //buy_product(&driver, &bot, &link).await?;
          switch_page(&driver, &pages, "search").await?;
          bot.send_message(ChatId::Id(-580425545), format!("Bought: {}", name)).await?;
          targets.remove(index);
          println!("{:?}", targets);
          save_targets(&mut targets).await?;
          break;
        }
      }
    }

    println!("{:?}", products);

    tokio::time::sleep(Duration::from_secs(1)).await;
  }
  
  #[allow(unreachable_code)]
  Ok(())
}
