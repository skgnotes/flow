use std::fs;
use std::path::PathBuf;
use chrono::Local;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct EntryInfo {
    filename: String,
    title: String,
    date: String,
}

fn get_journal_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join("Documents").join("Project Data Files").join("Journal")
}

fn parse_frontmatter(content: &str) -> (String, String) {
    let frontmatter_regex = regex::Regex::new(r"(?s)^---\n(.*?)\n---").unwrap();

    if let Some(captures) = frontmatter_regex.captures(content) {
        let frontmatter = captures.get(1).map_or("", |m| m.as_str());

        let title_regex = regex::Regex::new(r"(?m)^title:\s*(.*)$").unwrap();
        let date_regex = regex::Regex::new(r"(?m)^date:\s*(.*)$").unwrap();

        let title = title_regex.captures(frontmatter)
            .and_then(|c| c.get(1))
            .map_or("", |m| m.as_str().trim())
            .to_string();

        let date = date_regex.captures(frontmatter)
            .and_then(|c| c.get(1))
            .map_or("", |m| m.as_str().trim())
            .to_string();

        (title, date)
    } else {
        (String::new(), String::new())
    }
}

#[tauri::command]
fn list_entries() -> Result<Vec<EntryInfo>, String> {
    let journal_dir = get_journal_dir();

    // Create directory if it doesn't exist
    if !journal_dir.exists() {
        fs::create_dir_all(&journal_dir).map_err(|e| e.to_string())?;
    }

    let mut entries = Vec::new();

    match fs::read_dir(&journal_dir) {
        Ok(dir) => {
            for entry in dir {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                            // Read file to get metadata
                            if let Ok(content) = fs::read_to_string(&path) {
                                let (title, date) = parse_frontmatter(&content);
                                entries.push(EntryInfo {
                                    filename: filename.to_string(),
                                    title,
                                    date,
                                });
                            }
                        }
                    }
                }
            }
        }
        Err(e) => return Err(e.to_string()),
    }

    // Sort entries by date (newest first)
    entries.sort_by(|a, b| {
        // Try to parse dates for proper sorting
        use chrono::NaiveDate;

        let parse_date = |date_str: &str| -> Option<NaiveDate> {
            // Try multiple date formats
            NaiveDate::parse_from_str(date_str, "%B %-d, %Y")
                .or_else(|_| NaiveDate::parse_from_str(date_str, "%B %d, %Y"))
                .ok()
        };

        match (parse_date(&b.date), parse_date(&a.date)) {
            (Some(date_b), Some(date_a)) => date_b.cmp(&date_a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.filename.cmp(&a.filename),
        }
    });

    Ok(entries)
}

#[tauri::command]
fn read_entry(filename: String) -> Result<String, String> {
    let journal_dir = get_journal_dir();
    let file_path = journal_dir.join(&filename);

    fs::read_to_string(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_entry(filename: String, content: String) -> Result<(), String> {
    let journal_dir = get_journal_dir();

    // Create directory if it doesn't exist
    if !journal_dir.exists() {
        fs::create_dir_all(&journal_dir).map_err(|e| e.to_string())?;
    }

    let file_path = journal_dir.join(&filename);
    fs::write(&file_path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_entry() -> Result<String, String> {
    let now = Local::now();
    let date_string = now.format("%B %-d, %Y").to_string();
    let filename = format!("{}.md", date_string);

    let journal_dir = get_journal_dir();

    // Create directory if it doesn't exist
    if !journal_dir.exists() {
        fs::create_dir_all(&journal_dir).map_err(|e| e.to_string())?;
    }

    let file_path = journal_dir.join(&filename);

    // Create file with frontmatter header
    let initial_content = format!(
        "---\ntitle: \ndate: {}\n---\n\n",
        date_string
    );
    fs::write(&file_path, initial_content).map_err(|e| e.to_string())?;

    Ok(filename)
}

#[tauri::command]
fn update_entry_metadata(filename: String, title: String, date: String, content: String) -> Result<String, String> {
    let journal_dir = get_journal_dir();

    // Determine new filename based on title or date
    let new_filename = if title.trim().is_empty() {
        format!("{}.md", date)
    } else {
        format!("{}.md", title.trim())
    };

    let old_path = journal_dir.join(&filename);
    let new_path = journal_dir.join(&new_filename);

    // Check if old file exists
    if !old_path.exists() {
        return Err("File does not exist".to_string());
    }

    // Check if new filename already exists (and it's not the same file)
    if new_path.exists() && filename != new_filename {
        return Err("A file with that name already exists".to_string());
    }

    // Update file content with new frontmatter
    let updated_content = format!(
        "---\ntitle: {}\ndate: {}\n---\n\n{}",
        title,
        date,
        content
    );
    fs::write(&old_path, &updated_content).map_err(|e| e.to_string())?;

    // Rename file if needed
    if filename != new_filename {
        fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    }

    Ok(new_filename)
}

#[tauri::command]
fn rename_entry(old_filename: String, new_filename: String) -> Result<(), String> {
    let journal_dir = get_journal_dir();

    // Ensure new filename ends with .md
    let new_filename = if new_filename.ends_with(".md") {
        new_filename
    } else {
        format!("{}.md", new_filename)
    };

    let old_path = journal_dir.join(&old_filename);
    let new_path = journal_dir.join(&new_filename);

    // Check if old file exists
    if !old_path.exists() {
        return Err("File does not exist".to_string());
    }

    // Check if new filename already exists
    if new_path.exists() {
        return Err("A file with that name already exists".to_string());
    }

    fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_entry(filename: String) -> Result<(), String> {
    let journal_dir = get_journal_dir();
    let file_path = journal_dir.join(&filename);

    // Check if file exists
    if !file_path.exists() {
        return Err("File does not exist".to_string());
    }

    fs::remove_file(&file_path).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            list_entries,
            read_entry,
            save_entry,
            create_entry,
            rename_entry,
            update_entry_metadata,
            delete_entry
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
