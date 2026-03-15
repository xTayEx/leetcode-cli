use crate::{Error, Result};
use clap::Args;
use std::collections::HashMap;

static CATEGORY_HELP: &str = r#"Problem category
[algorithms, database, shell, concurrency, lcp, lcr, lcs, lcof]
Defaults to "algorithms" if not specified"#;

#[derive(Args)]
#[command(group = clap::ArgGroup::new("question-id").args(&["id", "daily"]).required(true))]
pub struct NoteArgs {
    /// Question id
    #[arg(value_parser = clap::value_parser!(i32))]
    pub id: Option<i32>,

    /// Open note for today's daily challenge
    #[arg(short = 'd', long)]
    pub daily: bool,

    /// Problem category
    #[arg(short, long, help = CATEGORY_HELP)]
    pub category: Option<String>,
}

impl NoteArgs {
    pub async fn run(&self) -> Result<()> {
        use crate::{Cache, cache::models::Question};
        use std::fs::File;
        use std::io::Write;
        use std::path::Path;

        let cache = Cache::new()?;

        let daily_id = if self.daily {
            Some(cache.get_daily_problem_id().await?)
        } else {
            None
        };

        let id = self.id.or(daily_id).ok_or(Error::NoneError)?;
        let cat = self.category.as_deref();

        let problem = cache.get_problem(id, cat)?;
        let conf = cache.to_owned().0.conf;

        let path = crate::helper::note_path(&problem)?;

        if !Path::new(&path).exists() {
            let mut qr = serde_json::from_str(&problem.desc);
            if qr.is_err() {
                qr = Ok(cache.get_question(id, cat).await?);
            }

            let question: Question = qr?;
            let desc = question.desc();

            let level = match problem.level {
                1 => "Easy",
                2 => "Medium",
                3 => "Hard",
                _ => "Unknown",
            };

            let mut file = File::create(&path)?;
            write!(
                file,
                "# {fid}. {name}\n\n\
                 - **Category**: {category}\n\
                 - **Difficulty**: {level}\n\
                 - **Acceptance**: {percent:.2}%\n\n\
                 ## Description\n\n\
                 {desc}\n\n\
                 ## Notes\n\n",
                fid = problem.fid,
                name = problem.name,
                category = problem.category,
                level = level,
                percent = problem.percent,
                desc = desc.trim(),
            )?;
        }

        let mut args: Vec<String> = Default::default();
        if let Some(editor_args) = conf.code.editor_args {
            args.extend_from_slice(&editor_args);
        }

        let mut envs: HashMap<String, String> = Default::default();
        if let Some(editor_envs) = &conf.code.editor_envs {
            for env in editor_envs.iter() {
                let parts: Vec<&str> = env.split('=').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim();
                    let value = parts[1].trim();
                    envs.insert(name.to_string(), value.to_string());
                } else {
                    return Err(
                        anyhow::anyhow!("Invalid editor environment variable: {env}").into(),
                    );
                }
            }
        }

        args.push(path);
        std::process::Command::new(conf.code.editor)
            .envs(envs)
            .args(args)
            .status()?;
        Ok(())
    }
}
