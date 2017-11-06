#![feature(plugin, custom_derive, decl_macro)]
#![plugin(rocket_codegen)]
extern crate chrono;
extern crate config;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod error;
pub use error::{Error, ErrorKind, Result};

pub mod app;
pub use app::{App, Date, DayCommit, Participant, Time};

pub mod slack;

use slack::slash_command::Request;
use slack::Response;

lazy_static! {
    static ref APP: App = App::assure_new();
}

#[derive(Clone)]
enum Command {
    Init,
    Add,
    Remove,
    Status,
    Commit,
    Reset,
    Log,
    Push,
    Help,
}

impl From<String> for Command {
    fn from(s: String) -> Command {
        use Command::*;
        if s.starts_with("init") {
            Init
        } else if s.starts_with("add") {
            Add
        } else if s.starts_with("rm") {
            Remove
        } else if s.starts_with("status") {
            Status
        } else if s.starts_with("commit") {
            Commit
        } else if s.starts_with("reset") {
            Reset
        } else if s.starts_with("log") {
            Log
        } else if s.starts_with("push") {
            Push
        } else {
            Help
        }
    }
}

impl Command {
    pub fn into_str(self) -> String {
        self.into()
    }
}

impl From<Command> for String {
    fn from(c: Command) -> String {
        use Command::*;
        match c {
            Init => "init",
            Add => "add",
            Remove => "rm",
            Status => "status",
            Commit => "commit",
            Reset => "reset",
            Log => "log",
            Push => "push",
            Help => "help",
        }.to_owned()
    }
}

pub fn handle_command(mut data: Request) -> Result<serde_json::Value> {
    use Command::*;

    let app = &APP;

    if !app.verify(&data.token) {
        bail!(ErrorKind::InvalidToken);
    }

    let command: Command = data.text.clone().into();
    data.text = data.text.replace(&command.clone().into_str(), "");
    data.text = data.text.trim().to_owned();

    Ok(serde_json::to_value(match command {
        Init => init_command,
        Add => add_command,
        Remove => rm_command,
        Status => status_command,
        Commit => commit_command,
        Reset => reset_command,
        Log => log_command,
        Push => push_command,
        Help => help_command,
    }(app, &data)?)?)
}

fn init_command(app: &App, _data: &Request) -> Result<Response> {
    match init(app) {
        Err(Error(ErrorKind::AlreadyInitialized, _)) => Ok(already_initialized_message()),
        Ok(day_commit) => Ok(initialized_message(&day_commit)),
        Err(e) => Err(e),
    }
}

fn add_command(app: &App, data: &Request) -> Result<Response> {
    let text = data.text.clone();
    if text.is_empty() {
        return Ok(invalid_argument_message());
    }
    let list: Vec<String> = text.split(' ').map(|s| s.to_owned()).collect();
    if list.is_empty() {
        return Ok(invalid_argument_message());
    }
    match add(app, list) {
        Err(Error(ErrorKind::NotInitialized, _)) => Ok(not_initialized_message()),
        Ok(added) => Ok(added_message(added)),
        Err(e) => Err(e),
    }
}

fn rm_command(app: &App, data: &Request) -> Result<Response> {
    let text = data.text.clone();
    if text.is_empty() {
        return Ok(invalid_argument_message());
    }
    let list: Vec<String> = text.split(' ').map(|s| s.to_owned()).collect();
    if list.is_empty() {
        return Ok(invalid_argument_message());
    }
    match rm(app, list) {
        Err(Error(ErrorKind::NotInitialized, _)) => Ok(not_initialized_message()),
        Ok(_) => Ok(removed_message()),
        Err(e) => Err(e),
    }
}

fn status_command(app: &App, _data: &Request) -> Result<Response> {
    match status(app) {
        Err(Error(ErrorKind::NotInitialized, _)) => Ok(not_initialized_message()),
        Ok(day_commit) => Ok(status_message(day_commit)),
        Err(e) => Err(e),
    }
}

fn commit_command(app: &App, data: &Request) -> Result<Response> {
    let text = data.text.clone();
    if text.is_empty() {
        return Ok(invalid_argument_message());
    }
    match commit(app, text) {
        Err(Error(ErrorKind::NotInitialized, _)) => Ok(not_initialized_message()),
        Ok(day_commit) => Ok(committed_message(day_commit)),
        Err(e) => Err(e),
    }
}

fn reset_command(app: &App, _data: &Request) -> Result<Response> {
    match reset(app) {
        Err(Error(ErrorKind::NotInitialized, _)) => Ok(not_initialized_message()),
        Ok(()) => Ok(resetted_message()),
        Err(e) => Err(e),
    }
}

fn log_command(app: &App, _data: &Request) -> Result<Response> {
    match log(app) {
        Err(Error(ErrorKind::NotInitialized, _)) => Ok(not_initialized_message()),
        Ok(commits) => Ok(log_message(&commits)),
        Err(e) => Err(e),
    }
}

fn push_command(app: &App, _data: &Request) -> Result<Response> {
    match push(app) {
        Err(Error(ErrorKind::NotInitialized, _)) => Ok(not_initialized_message()),
        Ok(()) => Ok(push_message()),
        Err(e) => Err(e),
    }
}

fn help_command(_app: &App, _data: &Request) -> Result<Response> {
    Ok(help_message())
}

fn initialized_message(day_commit: &DayCommit) -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::InChannel,
        text: format!("{} 근무 시작!", day_commit.date),
        mrkdwn: false,
    })
}

fn already_initialized_message() -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::Ephemeral,
        text: "이미 근무가 시작되었습니다.\n오늘의 근무를 취소하려면 `glt reset`"
            .to_owned(),
        mrkdwn: true,
    })
}

fn added_message(added: Vec<String>) -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::InChannel,
        text: format!("{} 근무자가 추가되었습니다.", {
            let mut s = String::new();
            for p in added {
                s = s + &p + ", ";
            }
            s.pop();
            s.pop();
            s
        }),
        mrkdwn: false,
    })
}

fn invalid_argument_message() -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::Ephemeral,
        text: "잘못된 인자 사용\n도움말을 보려면 `glt help`".to_owned(),
        mrkdwn: true,
    })
}

fn not_initialized_message() -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::Ephemeral,
        text: "근무가 시작되지 않았습니다.\n근무를 시작하려면 `glt init`"
            .to_owned(),
        mrkdwn: true,
    })
}

fn removed_message() -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::Ephemeral,
        text: "근무자가 제거되었습니다.".to_owned(),
        mrkdwn: false,
    })
}

fn status_message(day_commit: DayCommit) -> Response {
    use slack::*;
    let mut m = AttachedMessage {
        response_type: ResponseType::Ephemeral,
        attachments: vec![],
    };
    let mut a = Attachment {
        title: day_commit.date.to_string(),
        text: "".to_owned(),
        pretext: "오늘의 근무 기록".to_owned(),
        fields: vec![],
        mrkdwn_in: vec![],
    };
    a.fields.push(AttachmentFields {
        title: "시작 시간".to_owned(),
        value: day_commit.start_time.to_string(),
    });
    a.fields.push(AttachmentFields {
        title: "근무자".to_owned(),
        value: {
            let mut content = String::new();
            for p in day_commit.participants {
                let line = format!("{} - {}\n", p.name, p.commit_time);
                content.push_str(&line);
            }
            content
        },
    });
    m.attachments.push(a);
    Response::AttachedMessage(m)
}

fn committed_message(day_commit: DayCommit) -> Response {
    use slack::*;
    let mut m = AttachedMessage {
        response_type: ResponseType::InChannel,
        attachments: vec![],
    };
    let mut a = Attachment {
        title: day_commit.date.to_string(),
        text: "".to_owned(),
        pretext: "오늘의 근무가 끝났습니다. 수고하셨습니다!".to_owned(),
        fields: vec![],
        mrkdwn_in: vec![],
    };
    a.fields.push(AttachmentFields {
        title: "근무 시간".to_owned(),
        value: {
            let start_time = &day_commit.start_time;
            let end_time = &day_commit.end_time.unwrap();
            let diff = end_time - start_time;
            format!(
                "{}:{} ~ {}:{} {}시간 {}분",
                start_time.0,
                start_time.1,
                end_time.0,
                end_time.1,
                diff.0,
                diff.1
            )
        },
    });
    a.fields.push(AttachmentFields {
        title: "근무 내용".to_owned(),
        value: day_commit.message.unwrap(),
    });
    a.fields.push(AttachmentFields {
        title: "근무자".to_owned(),
        value: {
            let mut content = String::new();
            for p in day_commit.participants {
                let line = format!("{} - {}\n", p.name, p.commit_time);
                content.push_str(&line);
            }
            content
        },
    });
    m.attachments.push(a);
    Response::AttachedMessage(m)
}

fn resetted_message() -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::Ephemeral,
        text: "근무 기록이 삭제되었습니다.".to_owned(),
        mrkdwn: false,
    })
}

fn log_message(commits: &[DayCommit]) -> Response {
    use slack::*;
    use std::collections::HashMap;
    let first_day = commits.first().unwrap();
    let total_hour: Vec<f32> = commits
        .into_iter()
        .filter(|c| c.end_time.is_some())
        .map(|c| (&c.end_time.clone().unwrap() - &c.start_time).into())
        .collect();
    let total_hour: f32 = total_hour.into_iter().sum();
    let total_hour: Time = total_hour.into();
    let mut participants_record: HashMap<String, (u32, f32)> = HashMap::new();

    let mut m = AttachedMessage {
        response_type: ResponseType::InChannel,
        attachments: vec![],
    };
    let mut a = Attachment {
        title: format!("{}년 {}월", first_day.date.0, first_day.date.1),
        text: format!(
            "총 {}일, {}의 근무 기록이 있습니다.",
            commits.len(),
            total_hour
        ),
        pretext: "이 달의 근무 기록".to_owned(),
        fields: vec![],
        mrkdwn_in: vec!["fields".to_owned()],
    };
    for day_commit in commits {
        a.fields.push(AttachmentFields {
            title: format!("{}일", day_commit.date.2),
            value: {
                let mut s = String::new();
                if let Some(ref end_time) = day_commit.end_time {
                    let start_time = &day_commit.start_time;
                    let diff = end_time - start_time;
                    s = s
                        + &format!(
                            "{}:{} ~ {}:{} {}시간 {}분",
                            start_time.0,
                            start_time.1,
                            end_time.0,
                            end_time.1,
                            diff.0,
                            diff.1
                        );
                } else {
                    s = s + &format!("{} 시작", day_commit.start_time);
                }
                if let Some(ref msg) = day_commit.message {
                    s = s + "\n" + msg;
                }
                if !day_commit.participants.is_empty() {
                    s += "\n";
                    for p in &day_commit.participants {
                        s = s + &p.name + ", ";

                        let entry = participants_record
                            .entry(p.name.clone())
                            .or_insert((0u32, 0f32));
                        entry.0 += 1;
                        if let Some(ref end_time) = day_commit.end_time {
                            let d: f32 = (end_time - &p.commit_time).into();
                            entry.1 += d;
                        }
                    }
                    s.pop();
                    s.pop();
                }
                s
            },
        });
    }
    if !participants_record.is_empty() {
        a.fields.push(AttachmentFields {
            title: "총계".to_owned(),
            value: {
                let mut s = format!(
                    "총 {}일, {}의 근무 시간 중",
                    commits.len(),
                    total_hour
                );
                for (k, v) in participants_record {
                    let t: Time = v.1.into();
                    s = s + &format!("\n{} - {}일, {}", k, v.0, t);
                }
                s
            },
        });
    }
    m.attachments.push(a);
    Response::AttachedMessage(m)
}

fn push_message() -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::Ephemeral,
        text: "이 달의 근무가 끝났습니다. 수고하셨습니다!".to_owned(),
        mrkdwn: false,
    })
}

fn help_message() -> Response {
    use slack::*;
    Response::Message(Message {
        response_type: ResponseType::Ephemeral,
        text: "/glt init # 그 날의 근무 시작
/glt add <name> # 온 사람 이름 추가
/glt rm <name> # 온 사람 이름 제거
/glt status # 그 날의 근무 기록 보기
/glt commit <message> # 그 날의 근무 끝, 기록 추가
/glt reset # 그 날의 근무 취소, 기록 버리기
/glt log # 그 달의 근무 기록 보기
/glt push # 그 달의 근무 기록 저장 및 새 달로 넘어감"
            .to_owned(),
        mrkdwn: false,
    })
}

fn init(app: &App) -> Result<DayCommit> {
    use chrono::prelude::*;
    app.create_working_file(Local::today().into(), Local::now().time().into())
}

fn add(app: &App, participants: Vec<String>) -> Result<Vec<String>> {
    let now: Time = chrono::Local::now().time().into();
    let mut added: Vec<String> = vec![];

    app.edit_working_commit(|mut day_commit| {
        let cloned_commit = day_commit.clone();

        for p in participants {
            let pp = Participant {
                commit_time: now.clone(),
                name: p,
            };
            if !cloned_commit.participants.contains(&pp) {
                added.push(pp.name.clone());
                day_commit.participants.push(pp);
            }
        }
        day_commit
    })?;
    Ok(added)
}

fn rm(app: &App, participants: Vec<String>) -> Result<()> {
    app.edit_working_commit(|mut day_commit| {
        for p in participants {
            day_commit.participants.retain(|dp| dp.name != p);
        }
        day_commit
    }).map(|_| ())
}

fn status(app: &App) -> Result<DayCommit> {
    app.get_working_commit()
}

fn commit(app: &App, message: String) -> Result<DayCommit> {
    use chrono::prelude::*;
    app.commit_a_day(Local::now().time().into(), message)
}

fn reset(app: &App) -> Result<()> {
    app.remove_working_commit()
}

fn log(app: &App) -> Result<Vec<DayCommit>> {
    let commits = app.get_working_directory_commit()?;
    if commits.is_empty() {
        bail!(ErrorKind::NotInitialized);
    }
    Ok(commits)
}

fn push(app: &App) -> Result<()> {
    app.push_a_month()
}
