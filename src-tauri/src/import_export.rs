use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use encoding_rs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Answer {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Question {
    #[serde(default)]
    pub id: String,
    pub question: String,
    #[serde(default)]
    pub images: Vec<String>,
    pub answers: Vec<Answer>,
    pub correct: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BaseData {
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: String,
    #[serde(rename = "questionCount")]
    pub question_count: usize,
    pub questions: Vec<Question>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BaseIndex {
    pub slug: String,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: String,
    #[serde(rename = "questionCount")]
    pub question_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct BasesIndexFile {
    pub bases: Vec<BaseIndex>,
}

fn normalize_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;

    for ch in value.trim().chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                slug.push(lower);
            }
            last_was_separator = false;
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !slug.is_empty() && !last_was_separator {
            slug.push('-');
            last_was_separator = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn derive_slug(base_data: &BaseData, file_path: &Path) -> Result<String, String> {
    let mut candidates = vec![
        base_data.slug.as_str(),
        base_data.name.as_str(),
        base_data.display_name.as_str(),
    ];
    if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
        candidates.push(stem);
    }

    for candidate in candidates {
        let slug = normalize_slug(candidate);
        if !slug.is_empty() {
            return Ok(slug);
        }
    }

    Err("Could not derive a valid slug for imported base".to_string())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn read_file_with_fallback(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    
    // Check various encodings
    let encs = [
        encoding_rs::UTF_8,
        encoding_rs::WINDOWS_1250,
        encoding_rs::ISO_8859_2,
        encoding_rs::WINDOWS_1252,
    ];
    
    for enc in encs {
        let (cow, _, had_errors) = enc.decode(&bytes);
        if !had_errors {
            let s = cow.into_owned();
            let lower = s.to_lowercase();
            if lower.contains('ą') || lower.contains('ę') || lower.contains('ó') || lower.contains('ś') || lower.contains('ł') || lower.contains('ż') || lower.contains('ź') || lower.contains('ć') || lower.contains('ń') {
                return Ok(s);
            }
        }
    }
    
    let (cow, _, _) = encoding_rs::WINDOWS_1250.decode(&bytes);
    Ok(cow.into_owned())
}

fn parse_answer_code(code: &str) -> Vec<String> {
    let code = code.trim().to_uppercase();
    let code = code.trim_start_matches('\u{FEFF}')
                   .trim_start_matches('\u{200B}')
                   .trim_start_matches('\u{200C}')
                   .trim_start_matches('\u{200D}')
                   .trim_start_matches('\u{2060}');
    if !code.starts_with('X') || code.len() < 5 {
        return vec!["a".to_string()];
    }
    
    let mut correct = Vec::new();
    let options = ["a", "b", "c", "d", "e", "f", "g", "h"];
    for (i, char) in code.chars().skip(1).enumerate() {
        if char == '1' && i < options.len() {
            correct.push(options[i].to_string());
        }
    }
    if correct.is_empty() { vec!["a".to_string()] } else { correct }
}

fn parse_question(filepath: &Path) -> Option<Question> {
    let content = read_file_with_fallback(filepath).ok()?;
    let content = content.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = content.split('\n').filter(|l| !l.trim().is_empty()).collect();
    
    if lines.len() < 2 {
        return None;
    }
    
    let answer_code = lines[0].trim();
    let correct = parse_answer_code(answer_code);
    
    let mut rest = lines[1..].join("\n");
    let mut question_images = Vec::new();
    
    let main_img_re = Regex::new(r"(?i)^\[img\](.*?)\[/img\]").unwrap();
    if let Some(caps) = main_img_re.captures(&rest) {
        question_images.push(caps.get(1).unwrap().as_str().to_string());
        rest = rest[caps.get(0).unwrap().range().end..].trim().to_string();
    }
    
    let mut answers = Vec::new();
    let answer_pattern = FancyRegex::new(r"(?is)(?:^|\n)\s*([a-hA-H])\s*[\)\.\:]\s*(\[img\].*?\[/img\]|.+?)(?=\n\s*[a-hA-H]\s*[\)\.\:]|\z)").unwrap();
    let img_match_re = Regex::new(r"(?i)\[img\](.*?)\[/img\]").unwrap();
    let strip_img_re = Regex::new(r"(?i)\[img\].*?\[/img\]").unwrap();
    let strip_num_re = Regex::new(r"(?i)^\d+[\.\)\:]?\s*").unwrap();
    let strip_letter_re = Regex::new(r"(?i)^[a-hA-H][\)\.\:]\s*").unwrap();
    
    for mat in answer_pattern.captures_iter(&rest) {
        if let Ok(caps) = mat {
            let key = caps.get(1).unwrap().as_str().to_lowercase();
            let answer_content = caps.get(2).unwrap().as_str().trim();
            
            if let Some(img_caps) = img_match_re.captures(answer_content) {
                answers.push(Answer {
                    key,
                    text: None,
                    image: Some(img_caps.get(1).unwrap().as_str().to_string()),
                });
            } else {
                let text = strip_img_re.replace_all(answer_content, "").trim().to_string();
                if !text.is_empty() {
                    answers.push(Answer {
                        key,
                        text: Some(text),
                        image: None,
                    });
                }
            }
        }
    }
    
    let mut question_text = String::new();
    
    if answers.len() < 2 {
        let lines_rest: Vec<&str> = rest.split('\n').filter(|l| !l.trim().is_empty()).collect();
        let num_options = answer_code.trim().len().saturating_sub(1);
        
        let (q_text_raw, answer_lines) = if num_options > 0 && lines_rest.len() > num_options {
            let q_lines_count = lines_rest.len() - num_options;
            (lines_rest[..q_lines_count].join("\n"), &lines_rest[q_lines_count..])
        } else {
            let qt = if !lines_rest.is_empty() { lines_rest[0].to_string() } else { String::new() };
            let al = if lines_rest.len() > 1 { &lines_rest[1..] } else { &lines_rest[0..0] };
            (qt, al)
        };
        
        question_text = strip_num_re.replace_all(&q_text_raw, "").to_string();
        question_text = strip_img_re.replace_all(&question_text, "").trim().to_string();
        
        answers.clear();
        let keys = ["a", "b", "c", "d", "e", "f", "g", "h"];
        for (i, line) in answer_lines.iter().enumerate() {
            if i >= keys.len() { break; }
            if let Some(img_caps) = img_match_re.captures(line) {
                answers.push(Answer {
                    key: keys[i].to_string(),
                    text: None,
                    image: Some(img_caps.get(1).unwrap().as_str().to_string()),
                });
            } else {
                let clean = strip_letter_re.replace_all(line, "").trim().to_string();
                if !clean.is_empty() {
                    answers.push(Answer {
                        key: keys[i].to_string(),
                        text: Some(clean),
                        image: None,
                    });
                }
            }
        }
    } else {
        if let Ok(Some(first_answer)) = answer_pattern.find(&rest) {
            question_text = rest[..first_answer.start()].trim().to_string();
        } else {
            question_text = rest.trim().to_string();
        }
        question_text = strip_num_re.replace_all(&question_text, "").to_string();
        question_text = strip_img_re.replace_all(&question_text, "").trim().to_string();
    }
    
    if question_text.is_empty() && question_images.is_empty() {
        return None;
    }
    
    if answers.len() < 2 {
        return None;
    }
    
    let mut valid_correct: Vec<String> = correct.into_iter()
        .filter(|c| answers.iter().any(|a| a.key == *c))
        .collect();
    
    if valid_correct.is_empty() {
        valid_correct.push(answers[0].key.clone());
    }
    
    Some(Question {
        id: filepath.file_stem().unwrap().to_string_lossy().to_string(),
        question: question_text,
        images: question_images,
        answers,
        correct: valid_correct,
    })
}

fn update_bases_index(app_data_dir: &Path, slug: &str, display_name: &str, name: &str, question_count: usize) -> Result<(), String> {
    let data_dir = app_data_dir.join("data");
    fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let index_file = data_dir.join("bases.json");
    
    let mut index: BasesIndexFile = if index_file.exists() {
        let content = fs::read_to_string(&index_file).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(BasesIndexFile { bases: vec![] })
    } else {
        BasesIndexFile { bases: vec![] }
    };
    
    index.bases.retain(|b| b.slug != slug);
    index.bases.push(BaseIndex {
        slug: slug.to_string(),
        name: name.to_string(),
        display_name: display_name.to_string(),
        description: "".to_string(),
        question_count,
    });
    index.bases.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    
    fs::write(&index_file, serde_json::to_string_pretty(&index).unwrap()).map_err(|e| e.to_string())?;
    Ok(())
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(extension))
        .unwrap_or(false)
}

fn directory_contains_extension(dir: &Path, extension: &str) -> Result<bool, String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry in '{}': {}", dir.display(), e))?;
        let path = entry.path();
        if path.is_file() && has_extension(&path, extension) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn list_visible_subdirs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut subdirs = Vec::new();
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry in '{}': {}", dir.display(), e))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to read entry type in '{}': {}", dir.display(), e))?;
        if file_type.is_dir() && !entry.file_name().to_string_lossy().starts_with('.') {
            subdirs.push(entry.path());
        }
    }
    Ok(subdirs)
}

fn discover_old_base_question_dirs(base_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut dirs = Vec::new();

    let root_baza = base_dir.join("baza");
    if root_baza.is_dir() && directory_contains_extension(&root_baza, "txt")? {
        dirs.push(root_baza);
        return Ok(dirs);
    }

    if directory_contains_extension(base_dir, "txt")? {
        dirs.push(base_dir.to_path_buf());
        return Ok(dirs);
    }

    for subdir in list_visible_subdirs(base_dir)? {
        let sub_baza = subdir.join("baza");
        if sub_baza.is_dir() && directory_contains_extension(&sub_baza, "txt")? {
            dirs.push(sub_baza);
        } else if directory_contains_extension(&subdir, "txt")? {
            dirs.push(subdir);
        }
    }

    dirs.sort();
    dirs.dedup();
    Ok(dirs)
}

fn resolve_old_base_sources(selected_dir: &Path) -> Result<(String, Vec<PathBuf>), String> {
    let selected_name = selected_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("imported-base")
        .to_string();

    let direct_dirs = discover_old_base_question_dirs(selected_dir)?;
    if !direct_dirs.is_empty() {
        return Ok((selected_name, direct_dirs));
    }

    let mut wrapped_candidates: Vec<(String, Vec<PathBuf>)> = Vec::new();
    for subdir in list_visible_subdirs(selected_dir)? {
        let dirs = discover_old_base_question_dirs(&subdir)?;
        if !dirs.is_empty() {
            let candidate_name = subdir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("imported-base")
                .to_string();
            wrapped_candidates.push((candidate_name, dirs));
        }
    }

    match wrapped_candidates.len() {
        0 => Ok((selected_name, Vec::new())),
        1 => {
            if let Some(candidate) = wrapped_candidates.pop() {
                Ok(candidate)
            } else {
                Ok((selected_name, Vec::new()))
            }
        }
        _ => {
            let mut merged_dirs = Vec::new();
            for (_, dirs) in wrapped_candidates {
                merged_dirs.extend(dirs);
            }
            merged_dirs.sort();
            merged_dirs.dedup();
            Ok((selected_name, merged_dirs))
        }
    }
}

fn source_label_for_dir(dir: &Path) -> String {
    let is_baza = dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("baza"))
        .unwrap_or(false);
    if is_baza {
        if let Some(parent_name) = dir.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
            return parent_name.to_string();
        }
    }

    dir.file_name()
        .and_then(|name| name.to_str())
        .or_else(|| dir.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()))
        .unwrap_or("source")
        .to_string()
}

#[tauri::command]
pub fn import_old_base(app: AppHandle, path: String) -> Result<(), String> {
    let base_dir = PathBuf::from(path);
    if !base_dir.exists() || !base_dir.is_dir() {
        return Err("Path is not a valid directory".into());
    }

    let (resolved_base_name, dirs_to_search) = resolve_old_base_sources(&base_dir)?;
    if dirs_to_search.is_empty() {
        return Err(
            "No valid questions found. Expected one of: selected/baza/*.txt, selected/*.txt, selected/<base>/baza/*.txt, or selected/<base>/*.txt".into()
        );
    }

    let base_name = if resolved_base_name.trim().is_empty() {
        base_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("imported-base")
            .to_string()
    } else {
        resolved_base_name
    };
    let slug = normalize_slug(&base_name);
    if slug.is_empty() {
        return Err("Could not derive a valid slug from base directory name".into());
    }
    let mut questions = Vec::new();
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let bazy_dir = app_data_dir.join("bazy").join(&slug).join("baza");
    fs::create_dir_all(&bazy_dir).map_err(|e| e.to_string())?;
    let img_re = Regex::new(r"(?i)\.(png|jpg|jpeg|gif|webp)$").unwrap();

    for dir in &dirs_to_search {
        let entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read import source directory '{}': {}", dir.display(), e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry in '{}': {}", dir.display(), e))?;
            let file_path = entry.path();
            if file_path.is_file() && has_extension(&file_path, "txt") {
                if let Some(mut q) = parse_question(&file_path) {
                    if dirs_to_search.len() > 1 {
                        q.id = format!("{}_{}", source_label_for_dir(dir), q.id);
                    }
                    questions.push(q);
                }
            }
        }

        // Copy images in same folder
        let image_entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read image source directory '{}': {}", dir.display(), e))?;
        for sub_entry in image_entries {
            let sub_entry = sub_entry.map_err(|e| format!("Failed to read image entry in '{}': {}", dir.display(), e))?;
            let sub_path = sub_entry.path();
            if sub_path.is_file() && img_re.is_match(&sub_path.to_string_lossy()) {
                if let Some(file_name) = sub_path.file_name() {
                    let _ = fs::copy(&sub_path, bazy_dir.join(file_name));
                }
            }
        }
    }
    
    if questions.is_empty() {
        return Err("No valid questions found after scanning detected directories. Expected .txt files in selected/baza, selected root, or one-level wrapped base directory.".into());
    }
    
    let base_data = BaseData {
        name: base_name.clone(),
        slug: slug.clone(),
        display_name: base_name.replace("-", " "),
        description: "".to_string(),
        question_count: questions.len(),
        questions,
    };
    
    let data_dir = app_data_dir.join("data");
    fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    fs::write(data_dir.join(format!("{}.json", slug)), serde_json::to_string_pretty(&base_data).unwrap()).map_err(|e| e.to_string())?;
    
    update_bases_index(&app_data_dir, &slug, &base_data.display_name, &base_name, base_data.question_count)?;
    
    Ok(())
}

#[tauri::command]
pub fn import_new_base(app: AppHandle, path: String) -> Result<(), String> {
    let file_path = PathBuf::from(path);
    if !file_path.exists() || !file_path.is_file() {
        return Err("Path is not a valid file".into());
    }
    
    let content = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let mut base_data: BaseData = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let slug = derive_slug(&base_data, &file_path)?;
    base_data.slug = slug.clone();
    if base_data.name.trim().is_empty() {
        base_data.name = slug.clone();
    }

    let app_data_dir = app.path().app_data_dir().unwrap();
    let data_dir = app_data_dir.join("data");
    fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let serialized = serde_json::to_string_pretty(&base_data).map_err(|e| e.to_string())?;
    fs::write(data_dir.join(format!("{}.json", slug)), serialized).map_err(|e| e.to_string())?;
    
    // Copy baza images if they exist in the same folder or parent
    let parent_dir = file_path
        .parent()
        .ok_or_else(|| "Could not resolve imported file parent directory".to_string())?;
    let mut source_dirs = vec![parent_dir.join("bazy").join(&slug).join("baza")];
    source_dirs.push(parent_dir.join("bazy").join(&base_data.name).join("baza"));
    source_dirs.push(parent_dir.join("baza"));

    if let Some(existing_source) = source_dirs.into_iter().find(|p| p.exists()) {
        let bazy_dir = app_data_dir.join("bazy").join(&slug).join("baza");
        fs::create_dir_all(&bazy_dir).map_err(|e| e.to_string())?;
        let _ = copy_dir_all(&existing_source, &bazy_dir);
    }
    
    update_bases_index(&app_data_dir, &slug, &base_data.display_name, &base_data.name, base_data.question_count)?;
    
    Ok(())
}

#[tauri::command]
pub fn export_to_anki(app: AppHandle, slug: String, export_path: String) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().unwrap();
    let data_dir = app_data_dir.join("data");
    let base_file = data_dir.join(format!("{}.json", slug));
    
    if !base_file.exists() {
        return Err("Base not found".into());
    }
    
    let content = fs::read_to_string(&base_file).map_err(|e| e.to_string())?;
    let base_data: BaseData = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    
    let export_dir = PathBuf::from(export_path);
    let media_dir = export_dir.join("media");
    fs::create_dir_all(&media_dir).map_err(|e| e.to_string())?;
    
    let slug_img_dir = app_data_dir.join("bazy").join(&slug).join("baza");
    let source_img_dir = if slug_img_dir.exists() {
        slug_img_dir
    } else {
        app_data_dir.join("bazy").join(&base_data.name).join("baza")
    };
    
    let mut cards = Vec::new();
    
    for q in base_data.questions {
        let mut front_parts = Vec::new();
        if !q.question.trim().is_empty() {
            front_parts.push(format!("<b>{}</b>", q.question.trim()));
        }
        
        let prefix = format!("{}_{}", slug, q.id);
        
        for img in &q.images {
            let img_path = source_img_dir.join(img);
            if img_path.exists() {
                let safe_name = img.replace("/", "_").replace("\\", "_");
                let new_name = format!("{}_{}", prefix, safe_name);
                let _ = fs::copy(&img_path, media_dir.join(&new_name));
                front_parts.push(format!("<img src=\"{}\">", new_name));
            } else {
                front_parts.push(format!("[obrazek: {}]", img));
            }
        }
        
        front_parts.push("".to_string());
        
        for a in &q.answers {
            let key = a.key.to_uppercase();
            if let Some(img) = &a.image {
                let img_path = source_img_dir.join(img);
                if img_path.exists() {
                    let safe_name = img.replace("/", "_").replace("\\", "_");
                    let new_name = format!("{}_{}", prefix, safe_name);
                    let _ = fs::copy(&img_path, media_dir.join(&new_name));
                    front_parts.push(format!("{}) <img src=\"{}\">", key, new_name));
                } else {
                    front_parts.push(format!("{}) [obrazek: {}]", key, img));
                }
            } else if let Some(txt) = &a.text {
                front_parts.push(format!("{}) {}", key, txt));
            }
        }
        
        let mut back_parts = Vec::new();
        for c in &q.correct {
            let c_up = c.to_uppercase();
            if let Some(ans) = q.answers.iter().find(|a| a.key == *c) {
                if let Some(img) = &ans.image {
                    let img_path = source_img_dir.join(img);
                    if img_path.exists() {
                        let safe_name = img.replace("/", "_").replace("\\", "_");
                        let new_name = format!("{}_{}", prefix, safe_name);
                        back_parts.push(format!("{}) <img src=\"{}\">", c_up, new_name));
                    } else {
                        back_parts.push(format!("{}) [obrazek: {}]", c_up, img));
                    }
                } else if let Some(txt) = &ans.text {
                    back_parts.push(format!("{}) {}", c_up, txt));
                }
            } else {
                back_parts.push(c_up);
            }
        }
        
        let front = front_parts.join("<br>").replace('\t', " ");
        let back = back_parts.join("<br>").replace('\t', " ");
        cards.push(format!("{}\t{}", front, back));
    }
    
    let output_path = export_dir.join(format!("{}.txt", slug));
    fs::write(&output_path, cards.join("\n")).map_err(|e| e.to_string())?;
    
    Ok(())
}
