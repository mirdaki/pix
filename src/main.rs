use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};
use human_regex::{one_or_more, punctuation};
use photon_rs::{
    multiple::watermark,
    native::{open_image, open_image_from_bytes, save_image},
    transform::{compress, seam_carve},
    PhotonImage,
};
use std::{
    error::Error,
    fs::{self, File},
    io::{self, BufRead, Write},
    path::PathBuf,
};
use stop_words::{get, LANGUAGE};
use tera::{Context, Tera};

const WATERMARK: &[u8; 9144] = include_bytes!("../assets/Watermark.png");
const POST_TEMPLATE: &str = include_str!("../assets/post.md");

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds my watermark and compress the image for upload
    Mark,
    /// Creates a post for each .jpg in the directory, uses the date passed to create the posts.
    Post { date: NaiveDate },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Mark => mark(),
        Commands::Post { date } => {
            if let Err(e) = post(date) {
                let mut cmd = Cli::command();
                cmd.error(ErrorKind::InvalidValue, e).exit()
            };
        }
    }
}

// Mark related functions

fn mark() {
    let watermark_image =
        open_image_from_bytes(WATERMARK).expect("File watermark should be stored");

    let build_path = make_build_folder().unwrap();

    let files = get_jpg_files_in_current_directory().unwrap();

    for file in files {
        let mut image = open_image(file.to_str().unwrap()).expect("File should open");
        let output_path = build_path.join(file.file_name().unwrap());
        image = compress_image(&image);
        image = add_watermark(&mut image, &watermark_image);
        let _ = save_image(image, output_path.to_str().unwrap());
    }
}

fn get_jpg_files_in_current_directory() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let entries = fs::read_dir(".")?
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".jpg") {
                        return Some(PathBuf::from(file_name));
                    }
                }
            }
            None
        })
        .collect();

    Ok(entries)
}

fn make_build_folder() -> Result<PathBuf, Box<dyn Error>> {
    let build_path = PathBuf::from("build");
    fs::create_dir_all(&build_path)?;
    Ok(build_path.clone())
}

fn compress_image(image: &PhotonImage) -> PhotonImage {
    // let update_image = seam_carve(image, 2560, 1440);
    compress(image, 90)
}

fn add_watermark(image: &mut PhotonImage, watermark_image: &PhotonImage) -> PhotonImage {
    let x = image.get_width() - watermark_image.get_width();
    let y = image.get_height() - watermark_image.get_height();

    watermark(image, watermark_image, x, y);

    image.clone()
}

fn post(date: &NaiveDate) -> Result<(), String> {
    // Verify it is a Monday, Wednesday, or Friday
    if date.weekday() != chrono::Weekday::Mon
        && date.weekday() != chrono::Weekday::Wed
        && date.weekday() != chrono::Weekday::Fri
    {
        return Err("Date must be a Monday, Wednesday, or Friday".to_string());
    }

    // Constant values
    let stop_words = get(LANGUAGE::English);
    let regex_for_punctuation = one_or_more(punctuation()).to_regex();

    // Get the list of images in the current directory
    let files = get_jpg_files_in_current_directory().unwrap();

    let mut post_date_time: NaiveDateTime =
        date.and_time(NaiveTime::from_hms_opt(7, 0, 0).unwrap());

    for file in files {
        let file_name = file.file_stem().unwrap().to_str().unwrap();

        // Verify the file name is an i32
        if file_name.parse::<i32>().is_err() {
            return Err("File name must be an integer".to_string());
        }

        // Remove the last two characters from the file name
        let significant_digits = &file_name[..file_name.len() - 2];

        // Take in the alt text from the user
        println!("Enter the alt text for {}: ", file_name);
        let stdin = io::stdin();
        let description = stdin.lock().lines().next().unwrap().unwrap();

        // Remove punctuation and lowercase the text to make parsing easier
        let lowercase_description = description.to_ascii_lowercase();
        let description_without_punctuation =
            regex_for_punctuation.replace_all(&lowercase_description, "");

        // Remove stop words
        let tags = description_without_punctuation
            .split_whitespace()
            .collect::<Vec<&str>>();
        let tags = tags
            .iter()
            .filter(|tag| !stop_words.contains(&tag.to_string()))
            .collect::<Vec<&&str>>();

        // Build template of post
        let template_id = "post_template";
        let mut post_template = Tera::default();
        post_template
            .add_raw_template(template_id, POST_TEMPLATE)
            .unwrap();

        let mut template_context = Context::new();
        template_context.insert("title", file_name);
        template_context.insert("significant_digit_title", significant_digits);
        template_context.insert("upload_date", &post_date_time.timestamp());
        template_context.insert("description", &description);
        template_context.insert("tags", &tags);

        let post_content = post_template
            .render(template_id, &template_context)
            .unwrap();

        // Save template of post in current directory
        let mut file = File::create(format!("{}-{}.md", post_date_time, file_name)).unwrap();
        file.write_all(post_content.as_bytes()).unwrap();

        // Bump date_time to the next Monday, Wednesday, or Friday
        if post_date_time.weekday() == chrono::Weekday::Mon
            || post_date_time.weekday() == chrono::Weekday::Wed
        {
            post_date_time += chrono::Duration::days(2);
        } else if post_date_time.weekday() == chrono::Weekday::Fri {
            post_date_time += chrono::Duration::days(3);
        }
    }

    Ok(())
}
