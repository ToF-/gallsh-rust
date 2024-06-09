use clap_num::number_range;
use clap::Parser;
use crate::Order;

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
    #[arg(short, long)]
    pub pattern: Option<String>,

    /// Maximized window
    #[arg(short, long, default_value_t = false, help("show the images in full screen"))]
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

    /// Size ordered display
    #[arg(short, long, default_value_t = false)]
    pub size: bool,

    /// Colors size ordered display
    #[arg(short, long, default_value_t = false)]
    pub colors: bool,

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

    /// Window width (default is set with GALLSHWIDTH)
    #[arg(short, long)]
    pub width: Option<i32>,
    ///
    /// Window width (default is set with GALLSHHEIGHT)
    #[arg(short, long)]
    pub height: Option<i32>,
}

