use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    error::Error,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Color {
    Blue,
    Green,
    Amber,
    Purple,
    Rose,
    Teal,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Prompt {
    pub id: String,
    pub title: String,
    pub content: String,
    pub color: Color,
}

pub(crate) struct Library {
    path: PathBuf,
    pub prompts: Vec<Prompt>,
}

impl Library {
    pub fn open(app: &AppHandle) -> Result<Self, Box<dyn Error>> {
        let portable = std::env::current_exe()?.with_file_name("prompts.json");
        let (path, contents) = match fs::read(&portable) {
            Ok(contents) => (portable, Some(contents)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let path = app.path().app_config_dir()?.join("prompts.json");
                let contents = match fs::read(&path) {
                    Ok(contents) => Some(contents),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                    Err(error) => return Err(file_error(&path, error).into()),
                };
                (path, contents)
            }
            Err(error) => return Err(file_error(&portable, error).into()),
        };
        let prompts: Vec<Prompt> = match &contents {
            Some(bytes) => {
                serde_json::from_slice(bytes).map_err(|error| file_error(&path, error))?
            }
            None => serde_json::from_str(include_str!("../../prompts.example.json"))?,
        };
        let mut ids = HashSet::new();
        for prompt in &prompts {
            if prompt.id.trim().is_empty() || !ids.insert(&prompt.id) {
                return Err(file_error(&path, "指令 ID 为空或重复").into());
            }
        }
        if contents.is_none() {
            write_prompts(&path, &prompts)?;
        }
        Ok(Self { path, prompts })
    }

    pub fn replace(&mut self, prompts: Vec<Prompt>) -> Result<(), String> {
        write_prompts(&self.path, &prompts)?;
        self.prompts = prompts;
        Ok(())
    }
}

fn file_error(path: &Path, error: impl std::fmt::Display) -> String {
    format!("{}：{error}", path.display())
}

fn write_prompts(path: &Path, prompts: &[Prompt]) -> Result<(), String> {
    let parent = path.parent().ok_or("找不到指令文件的目录。")?;
    let write = || -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(temp.as_file_mut(), prompts)?;
        temp.write_all(b"\n")?;
        temp.as_file().sync_all()?;
        temp.persist(path)?;
        Ok(())
    };
    write().map_err(|error| file_error(path, error))
}
