use std::time::Duration;
use std::io::{BufReader, BufRead};
use std::fs;
use chan::Sender;
use scheduler::Task;

use block::{Block, ConfigBlock};
use config::Config;
use de::deserialize_duration;
use errors::*;
use widgets::text::TextWidget;
use widget::{I3BarWidget, State};
use uuid::Uuid;

extern crate nix;
extern crate mailparse;

use self::mailparse::*;

pub struct Maildir {
    maildir: TextWidget,
    id: String,
    update_interval: Duration,
    label: String,
    path: String
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MaildirConfig {
    /// Path to collect information from
    #[serde(default = "MaildirConfig::default_path")]
    pub path: String,

    /// Alias that is displayed for path
    #[serde(default = "MaildirConfig::default_label")]
    pub label: String,

    /// Update interval in seconds
    #[serde(default = "MaildirConfig::default_interval", deserialize_with = "deserialize_duration")]
    pub interval: Duration,
}

impl MaildirConfig {
    fn default_path() -> String {
        "/".to_owned()
    }

    fn default_label() -> String {
        "/".to_owned()
    }

    fn default_interval() -> Duration {
        Duration::from_secs(20)
    }
}

impl Maildir {
    fn compute_state(&self, count: usize) -> State {
        if count > 0 {
            State::Warning
        } else {
            State::Idle
        }
    }
}

impl ConfigBlock for Maildir {
    type Config = MaildirConfig;

    fn new(block_config: Self::Config, config: Config, _tx_update_request: Sender<Task>) -> Result<Self> {
        Ok(Maildir {
            id: Uuid::new_v4().simple().to_string(),
            update_interval: block_config.interval,
            maildir: TextWidget::new(config).with_text("Maildir"),
            label: block_config.label,
            path: block_config.path
        })
    }
}

impl Block for Maildir {
    fn update(&mut self) -> Result<Option<Duration>> {
        let mut mails = Vec::new();

        let entries = fs::read_dir(self.path.as_str()).block_error(
            "maildir",
            "Failed to open maildir",
        )?;

        for entry in entries {
            let entry = entry.block_error("maildir", "failed list files")?;
            let metadata = entry.metadata().block_error(
                "maildir",
                "failed to get file metadata",
            )?;
            if metadata.is_file() == false {
                continue;
            }
            let f = fs::File::open(entry.path()).block_error(
                "maildir",
                "no such file",
            )?;

            for line in BufReader::new(f).lines() {
                let line = line.block_error("maildir", "failed to read email")?;
                if line.starts_with("From: ") {
                    let (header, _) = parse_header(line.as_bytes()).block_error(
                        "maildir",
                        "failed to parse header",
                    )?;
                    let header = header.get_value().block_error("maildir", "failed to parse header")?;
                    mails.push(header);
                    break;
                }
            }
        }
        let mail_string = mails.join(", ");
        let count = mails.len();
        if count > 0 {
            self.maildir.set_text(format!(
                "{0}:{1} {2}",
                self.label,
                count,
                mail_string
            ));
        } else {
            self.maildir.set_text(format!(""));
        }

        let state = self.compute_state(count);
        self.maildir.set_state(state);

        Ok(Some(self.update_interval))
    }

    fn view(&self) -> Vec<&I3BarWidget> {
        vec![&self.maildir]
    }

    fn id(&self) -> &str {
        &self.id
    }
}
