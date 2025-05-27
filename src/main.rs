use ansi_term::Colour::{Blue, Green, Red, Yellow};
use clap::{Parser, Subcommand};
use dirs::home_dir;
use epub::doc::EpubDoc;
use html2text::from_read;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author = "您的名字", version, about = "一个简单的 EPUB 阅读器", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {

    Open {

        #[arg(required = true)]
        path: String,
    },

    Bookmarks,

    Continue,
}

#[derive(Serialize, Deserialize, Debug)]
struct BookProgress {
    path: String,
    current_page: usize,
    total_pages: usize,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct ReaderState {
    bookmarks: HashMap<String, BookProgress>,
    last_read: Option<String>,
}

struct Reader {
    state: ReaderState,
    state_file: PathBuf,
}

impl Reader {
    fn new() -> Self {
        let state_dir = home_dir().unwrap_or_default().join(".epub_reader");
        if !state_dir.exists() {
            fs::create_dir_all(&state_dir).expect("Failed to create config directory");
        }

        let state_file = state_dir.join("state.json");
        let state = if state_file.exists() {
            let file = File::open(&state_file).expect("Failed to open state file");
            serde_json::from_reader(file).unwrap_or_default()
        } else {
            ReaderState::default()
        };

        Reader { state, state_file }
    }

    fn save_state(&self) {
        let file = File::create(&self.state_file).expect("Failed to create state file");
        serde_json::to_writer_pretty(file, &self.state).expect("Failed to write state");
    }

    fn open_book(&mut self, path: &str) {
        let path_buf = PathBuf::from(path);
        let canonical_path = fs::canonicalize(&path_buf)
            .unwrap_or_else(|_| panic!("Failed to resolve path: {}", path));
        let path_str = canonical_path.to_string_lossy().to_string();

        let mut doc = match EpubDoc::new(&path_buf){
            Ok(doc) => doc,
            Err(e) => {
                eprintln!("{}: Failed to open EPUB file: {}", Red.paint("Error"), e);
                return;
            }
        };

        let total_pages = doc.get_num_pages();
        println!(
            "{} '{}' ({} pages)",
            Green.paint("Opened"),
            path_buf.file_name().unwrap_or_default().to_string_lossy(),
            total_pages
        );


        let current_page = match self.state.bookmarks.get(&path_str) {
            Some(progress) => {
                println!(
                    "{} at page {} of {}",
                    Blue.paint("Resuming"),
                    progress.current_page + 1,
                    progress.total_pages
                );
                progress.current_page
            }
            None => 0,
        };

        doc.set_current_page(current_page);
        self.state.last_read = Some(path_str.clone());
        self.save_state();

        self.read_book(doc, path_str);
    }

    fn read_book(&mut self, mut doc: EpubDoc<BufReader<File>>, path: String) {

        println!("正在加载书籍信息...");


        if let Some(title) = doc.metadata.get("title") {
            if !title.is_empty() {
                println!("书名: {}", Green.paint(title[0].clone()));
            }
        }


        if let Some(creator) = doc.metadata.get("creator") {
            if !creator.is_empty() {
                println!("作者: {}", Blue.paint(creator[0].clone()));
            }
        }


        println!("\n按回车键开始阅读...");
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let mut current_page = doc.get_current_page();
        let total_pages = doc.get_num_pages();

        loop {

            print!("\x1B[2J\x1B[1;1H");


            match doc.get_current() {
                Ok(content) => {
                    let text = from_read(content.as_slice(), 80);
                    println!("{}\n", text);
                }
                Err(_) => {
                    println!("{}: Failed to read page content", Red.paint("Error"));
                }
            }


            println!(
                "\n{} {} / {} ({}%)",
                Yellow.paint("页码"),
                current_page + 1,
                total_pages,
                (current_page + 1) * 100 / total_pages
            );
            println!("\n{}", Blue.paint("命令:"));
            println!("  n: 下一页      p: 上一页");
            println!("  b: 添加书签    g: 跳转到指定页");
            println!("  t: 查看目录    i: 显示书籍信息");
            println!("  q: 退出阅读");


            print!("\n> ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            match input.trim() {
                "n" => {
                    if current_page < total_pages - 1 {
                        doc.go_next();
                        current_page = doc.get_current_page();
                        self.update_progress(&path, current_page, total_pages);
                    } else {
                        println!("{}: 已经是最后一页了", Yellow.paint("提示"));
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
                "p" => {
                    if current_page > 0 {
                        doc.go_prev();
                        current_page = doc.get_current_page();
                        self.update_progress(&path, current_page, total_pages);
                    } else {
                        println!("{}: 已经是第一页了", Yellow.paint("提示"));
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
                "b" => {
                    println!("{} 当前页面", Green.paint("已添加书签"));
                    self.update_progress(&path, current_page, total_pages);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
                "g" => {
                    print!("请输入页码 (1-{}): ", total_pages);
                    io::stdout().flush().unwrap();
                    let mut page_input = String::new();
                    io::stdin().read_line(&mut page_input).unwrap();

                    if let Ok(page) = page_input.trim().parse::<usize>() {
                        if page >= 1 && page <= total_pages {
                            doc.set_current_page(page - 1);
                            current_page = doc.get_current_page();
                            self.update_progress(&path, current_page, total_pages);
                        } else {
                            println!("{}: 无效的页码", Red.paint("错误"));
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    } else {
                        println!("{}: 无效的输入", Red.paint("错误"));
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
                "t" => {

                    self.display_toc(&doc);

                    println!("\n按回车键继续阅读...");
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                }
                "i" => {

                    self.display_book_info(&doc);

                    println!("\n按回车键继续阅读...");
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                }
                "q" => {
                    self.update_progress(&path, current_page, total_pages);
                    break;
                }
                _ => {
                    println!("{}: 未知命令", Red.paint("错误"));
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
    }

    fn update_progress(&mut self, path: &str, current_page: usize, total_pages: usize) {
        self.state.bookmarks.insert(
            path.to_string(),
            BookProgress {
                path: path.to_string(),
                current_page,
                total_pages,
            },
        );
        self.save_state();
    }

    fn list_bookmarks(&self) {
        if self.state.bookmarks.is_empty() {
            println!("{}: No bookmarks found", Yellow.paint("Note"));
            return;
        }

        println!("{}", Green.paint("Your bookmarks:"));
        for (i, (path, progress)) in self.state.bookmarks.iter().enumerate() {
            let file_name = Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            println!(
                "{}. {} - Page {} of {} ({}%)",
                i + 1,
                file_name,
                progress.current_page + 1,
                progress.total_pages,
                (progress.current_page + 1) * 100 / progress.total_pages
            );
        }
    }

    fn continue_reading(&mut self) {
        if let Some(path) = self.state.last_read.clone() {
            println!("{} last book", Blue.paint("Continuing"));
            self.open_book(&path);
        } else {
            println!("{}: No book to continue", Yellow.paint("Note"));
        }
    }

    fn display_toc(&self, doc: &EpubDoc<std::io::BufReader<File>>) {
        println!("\n{}", Green.paint("目录:"));

        let toc = &doc.toc;
        if !toc.is_empty() {
            for (i, item) in toc.iter().enumerate() {
                println!("{}. {}", i + 1, item.label);
            }
        } else {
            println!("{}: 此书籍没有目录信息", Yellow.paint("提示"));
        }
    }

    fn display_book_info(&self, doc: &EpubDoc<std::io::BufReader<File>>) {
        println!("\n{}", Green.paint("书籍信息:"));


        if let Some(title) = doc.metadata.get("title") {
            if !title.is_empty() {
                println!("书名: {}", title[0]);
            }
        }


        if let Some(creator) = doc.metadata.get("creator") {
            if !creator.is_empty() {
                println!("作者: {}", creator[0]);
            }
        }


        if let Some(publisher) = doc.metadata.get("publisher") {
            if !publisher.is_empty() {
                println!("出版商: {}", publisher[0]);
            }
        }


        if let Some(language) = doc.metadata.get("language") {
            if !language.is_empty() {
                println!("语言: {}", language[0]);
            }
        }


        if let Some(description) = doc.metadata.get("description") {
            if !description.is_empty() {
                println!("描述: {}", description[0]);
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let mut reader = Reader::new();

    match &cli.command {
        Commands::Open { path } => {
            reader.open_book(path);
        }
        Commands::Bookmarks => {
            reader.list_bookmarks();
        }
        Commands::Continue => {
            reader.continue_reading();
        }
    }
}