use crate::determine_path;
use clap_num::number_range;
use std::env;
use clap::Parser;
use crate::Order;
use crate::is_valid_path;

const DEFAULT_WIDTH: i32   = 1000;
const DEFAULT_HEIGHT: i32  = 1000;
const WIDTH_ENV_VAR :&str  = "GALLSHWIDTH";
const HEIGHT_ENV_VAR :&str = "GALLSHHEIGHT";

fn less_than_11(s: &str) -> Result<usize, String> {
    number_range(s,1,10)
}

/// Gallery Show
#[derive(Parser, Clone, Debug)]
#[command(infer_subcommands = true, infer_long_args = true, author, version, about, long_about = None)]
/// Pattern that displayed files must have
pub struct Args {

    /// Directory to search (default is set with variable GALLSHDIR)
    pub directory: Option<String>,

    /// Pattern (only files with names matching the regular expression will be displayed)
    #[arg(long)]
    pub pattern: Option<String>,

    /// Maximized window
    #[arg(long, default_value_t = false, help("show the images in full screen"))]
    pub maximized: bool,

    /// Ordered display (or random)
    #[arg(short, long,value_name("order"), ignore_case(true), default_value_t = Order::Random)]
    pub order: Order,

    /// Date ordered display
    #[arg(short, long, default_value_t = false)]
    pub date: bool,

    /// Name ordered display
    #[arg(short, long, default_value_t = false)]
    pub name: bool,

    /// Rank value ordered display
    #[arg(short, long, default_value_t = false)]
    pub value:bool,

    /// Palette value ordered display
    #[arg(short, long, default_value_t = false)]
    pub palette:bool,

    /// Size ordered display
    #[arg(short, long, default_value_t = false)]
    pub size: bool,

    /// Colors size ordered display
    #[arg(short, long, default_value_t = false)]
    pub colors: bool,

    /// Label ordered display
    #[arg(short, long, default_value_t = false)]
    pub label: bool,

    /// Timer delay for next picture
    #[arg(long)]
    pub timer: Option<u64>,

    /// Reading List (only files in the list are displayed)
    #[arg(short, long)]
    pub reading: Option<String>,

    /// Index of first image to read
    #[arg(short, long)]
    pub index: Option<usize>,

    /// Grid Size
    #[arg(short, long, value_parser=less_than_11)]
    pub grid: Option<usize>,

    /// From index number
    #[arg(long)]
    pub from: Option<usize>,

    /// To index number
    #[arg(long)]
    pub to: Option<usize>,

    /// File to view
    #[arg(short, long)]
    pub file: Option<String>,

    /// Sample
    #[arg(long, default_value_t = false)]
    pub sample: bool,

    /// Thumbnails only
    #[arg(short,long)]
    pub thumbnails: bool,

    /// Update image data and then quit
    #[arg(short,long)]
    pub update_image_data: bool,

    /// Copy selection to a target folder
    #[arg(long)]
    pub copy_selection: Option<String>,

    /// Move selection to a target folder
    #[arg(long)]
    pub move_selection: Option<String>,

    /// Move entries with labels to target folder
    #[arg(short,long,value_delimiter=' ',num_args=2, value_names(&["LABEL","DIRECTORY"]))]
    pub move_label: Option<Vec<String>>,

    /// All labels move target directory
    #[arg(short, long, value_name("TARGET"))]
    pub all_label_move_target: Option<String>,

    /// Window width (default is set with GALLSHWIDTH)
    #[arg(long)]
    pub width: Option<i32>,
    ///
    /// Window width (default is set with GALLSHHEIGHT)
    #[arg(long)]
    pub height: Option<i32>,

    /// Show palette extraction
    #[arg(short, long)]
    pub extraction: bool,
}

impl Args {
    pub fn width(&self) -> i32 {
        let candidate_width = match self.width {
            Some(n) => n,
            None => match env::var(WIDTH_ENV_VAR) {
                Ok(s) => match s.parse::<i32>() {
                    Ok(n) => n,
                    _ => {
                        println!("illegal width value, setting to default");
                        DEFAULT_WIDTH
                    }
                },
                _ => {
                    DEFAULT_WIDTH
                }
            }
        };
        if candidate_width < 3000 && candidate_width > 100 {
            candidate_width
        } else {
            println!("illegal width value, setting to default");
            DEFAULT_WIDTH
        }
    }

    pub fn height(&self) -> i32 {
        let candidate_height = match self.height {
            Some(n) => n,
            None => match env::var(HEIGHT_ENV_VAR) {
                Ok(s) => match s.parse::<i32>() {
                    Ok(n) => n,
                    _ => {
                        println!("illegal height value, setting to default");
                        DEFAULT_HEIGHT
                    }
                },
                _ => {
                    DEFAULT_HEIGHT
                }
            }
        };
        if candidate_height < 3000 && candidate_height > 100 {
            candidate_height
        } else {
            println!("illegal height value, setting to default");
            DEFAULT_HEIGHT
        }
    }

    pub fn copy_selection_target(&self) -> Result<Option<String>, String> {
        selection_target(&self.copy_selection)
    }

    pub fn move_selection_target(&self) -> Result<Option<String>, String> {
        selection_target(&self.move_selection)
    }

    pub fn all_label_move_target(&self) -> Result<Option<String>, String> {
        selection_target(&self.all_label_move_target)
    }

    pub fn grid_size(&self) -> usize {
        match self.grid {
            Some(size) => if size > 0 && size <= 10 { size } else { if self.thumbnails { 10 } else { 1 } },
            None => if self.thumbnails { 10 } else { 1 },
        }
    }

    pub fn order(&self) -> Order {
        Order::from_options(self.name, self.date, self.size, self.colors, self.value, self.palette, self.label)
    }

    pub fn path(&self) -> String {
        determine_path(self.directory.clone())
    }

    pub fn sample(&self) -> bool {
        self.sample
    }

}

pub fn selection_target(target_arg: &Option<String>) -> Result<Option<String>, String> {
    match target_arg {
        Some(target) => {
            if is_valid_path(target) {
                Ok(Some(target.to_string()))
            } else {
                Err(format!("path {} doesn't exist", target))
            }
        },
        None => Ok(None),
    }
}



