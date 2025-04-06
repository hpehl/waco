use crate::command::CommandStatus;
use console::{style, truncate_str};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::process::Output;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdout};

#[derive(Clone)]
pub struct Progress {
    prefix: String,
    image_name: String,
    pub bar: ProgressBar,
}

impl Progress {
    pub fn join(multi_progress: &MultiProgress, prefix: &str, image_name: &str) -> Progress {
        let progress = Progress {
            prefix: prefix.to_string(),
            image_name: image_name.to_string(),
            bar: Self::spinner(prefix),
        };
        progress.bar.enable_steady_tick(Duration::from_millis(100));
        multi_progress.add(progress.bar.clone());
        progress
            .bar
            .set_message(format!("{:<41}", style(image_name).cyan()));
        progress
    }

    pub fn new(prefix: &str, image_name: &str) -> Progress {
        let progress = Progress {
            prefix: prefix.to_string(),
            image_name: image_name.to_string(),
            bar: Self::spinner(prefix).with_message(format!("{:<41}", style(image_name).cyan())),
        };
        progress.bar.enable_steady_tick(Duration::from_millis(100));
        progress
    }

    fn spinner(prefix: &str) -> ProgressBar {
        ProgressBar::new_spinner()
            .with_style(
                ProgressStyle::default_spinner()
                    // https://github.com/sindresorhus/cli-spinners
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", " "])
                    .template("{spinner:.dim.bold} {prefix}{wide_msg}")
                    .expect("Invalid spinner template"),
            )
            .with_prefix(format!("{:<4}   ", style(prefix).yellow()))
    }

    pub async fn trace_progress<R>(&self, mut reader: Lines<BufReader<R>>)
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        while let Some(line) = reader
            .next_line()
            .await
            .expect("Unable to read output from command.")
        {
            self.show_progress(line.as_str());
        }
    }

    pub fn show_progress(&self, progress: &str) {
        self.bar.set_message(format!(
            "{:<41}   {}",
            style(self.image_name.clone()).cyan(),
            style(truncate_str(progress, 80, "...")).dim()
        ));
    }

    pub fn finish(&self, output: std::io::Result<Output>, status: Option<&str>) -> CommandStatus {
        match output {
            Ok(output) => {
                if output.status.success() {
                    self.success(status);
                    CommandStatus::success(self.image_name.as_str())
                } else {
                    self.error(
                        format!("Command failed with code {}", output.status.code().unwrap())
                            .as_str(),
                    );
                    CommandStatus::error(
                        self.image_name.as_str(),
                        String::from_utf8_lossy(&output.stderr)
                            .replace('\n', " ")
                            .as_str(),
                    )
                }
            }
            Err(e) => {
                self.error(format!("Command failed: {}", e).as_str());
                CommandStatus::error(self.image_name.as_str(), e.to_string().as_str())
            }
        }
    }

    pub fn finish_no_output(&self, status: Option<&str>) -> CommandStatus {
        self.success(status);
        CommandStatus::success(self.image_name.as_str())
    }

    fn success(&self, status: Option<&str>) {
        self.bar.set_prefix(format!(
            "{:<4}   ",
            style(self.prefix.as_str()).green().bold()
        ));
        self.bar.finish_with_message(match status {
            Some(status) => format!(
                "{:<41}   {}",
                style(self.image_name.as_str()).cyan(),
                style(status).green()
            ),
            None => format!("{:<41}", style(self.image_name.as_str()).cyan()),
        });
    }

    fn error(&self, err: &str) {
        self.bar.set_prefix(format!(
            "{:<4}   ",
            style(self.prefix.as_str()).red().bold()
        ));
        self.bar.abandon_with_message(format!(
            "{:<41}   {}",
            style(self.image_name.as_str()).cyan(),
            style(err).red()
        ));
    }
}

// ------------------------------------------------------ stdout / stderr

pub fn stdout_reader(child: &mut Child) -> Lines<BufReader<ChildStdout>> {
    let stdout = child
        .stdout
        .take()
        .expect("Command did not have a handle to stdout.");
    let stdout_reader = BufReader::new(stdout).lines();
    stdout_reader
}

pub fn stderr_reader(child: &mut Child) -> Lines<BufReader<ChildStderr>> {
    let stderr = child
        .stderr
        .take()
        .expect("Command did not have a handle to stderr.");
    let stderr_reader = BufReader::new(stderr).lines();
    stderr_reader
}
