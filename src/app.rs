use super::{ErrorKind, Result};
use std::fs::File;
use std::path::PathBuf;
use std::io::{Read, Write};
use std::ops::Sub;
use toml;
use chrono::Date as cDate;
use chrono::{Datelike, Local, Timelike};

#[derive(Deserialize, Serialize, Clone)]
pub struct DayCommit {
    pub date: Date,
    pub start_time: Time,
    pub end_time: Option<Time>,
    pub message: Option<String>,
    pub participants: Vec<Participant>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl From<Date> for cDate<Local> {
    fn from(d: Date) -> cDate<Local> {
        use chrono::prelude::*;
        Local.ymd(d.year, d.month, d.day)
    }
}

impl From<cDate<Local>> for Date {
    fn from(d: cDate<Local>) -> Date {
        Date {
            year: d.year(),
            month: d.month(),
            day: d.day(),
        }
    }
}

impl ::std::fmt::Display for Date {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}년 {}월 {}일", self.year, self.month, self.day)
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
}

impl From<Time> for ::chrono::NaiveTime {
    fn from(t: Time) -> ::chrono::NaiveTime {
        ::chrono::NaiveTime::from_hms(t.hour, t.minute, 0)
    }
}

impl From<::chrono::NaiveTime> for Time {
    fn from(t: ::chrono::NaiveTime) -> Time {
        Time {
            hour: t.hour(),
            minute: t.minute(),
        }
    }
}

impl From<Time> for f32 {
    fn from(t: Time) -> f32 {
        (t.hour as f32) + ((t.minute as f32) / 60f32)
    }
}

impl From<f32> for Time {
    fn from(f: f32) -> Time {
        Time {
            hour: f as u32,
            minute: ((f * 60f32) as u32) % 60,
        }
    }
}

impl Sub for Time {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Time {
            hour: self.hour - rhs.hour,
            minute: self.minute - rhs.minute,
        }
    }
}

impl<'a> Sub for &'a Time {
    type Output = Time;
    fn sub(self, rhs: Self) -> Self::Output {
        Time {
            hour: self.hour - rhs.hour,
            minute: self.minute - rhs.minute,
        }
    }
}

impl ::std::fmt::Display for Time {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}시 {}분", self.hour, self.minute)
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Participant {
    pub commit_time: Time,
    pub name: String,
}

impl PartialEq<Participant> for Participant {
    fn eq(&self, other: &Participant) -> bool {
        self.name.eq(&other.name)
    }
}

#[derive(Deserialize)]
pub struct App {
    pub verification_token: String,
    pub api_token: String,
    pub data_path: String,
}

impl App {
    pub fn try_new() -> Result<App> {
        use std::env::args;
        use std::path::Path;
        use config::{Config, File};
        let mut app = Config::new();
        app.merge(if args().len() >= 2 {
            File::from(Path::new(&args().nth(1).unwrap()))
        } else {
            File::with_name("settings")
        })?;
        app.try_into::<App>()
            .map_err(|e| ErrorKind::Config(e).into())
    }

    pub fn assure_new() -> App {
        use std::fs::read_dir;
        let mut app = match App::try_new() {
            Ok(s) => s,
            Err(e) => panic!("Settings file parse error!, {}", e),
        };
        if let Err(e) = read_dir(&app.data_path) {
            panic!("Invalid data folder. Check settings file!, {}", e);
        }
        if !app.data_path.ends_with('/') {
            app.data_path.push('/');
        }
        app
    }

    pub fn verify(&self, token: &str) -> bool {
        token == self.verification_token
    }

    fn get_commit_from_file(file: &mut File) -> Result<DayCommit> {
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        toml::from_str(&content).map_err(|e| ErrorKind::Toml(e).into())
    }

    fn get_commit_from_path(path: PathBuf) -> Result<DayCommit> {
        let mut file = File::open(path)?;
        App::get_commit_from_file(&mut file)
    }

    pub fn create_working_file(&self, date: Date, time: Time) -> Result<DayCommit> {
        let mut path = PathBuf::from(&self.data_path);
        path.push("working.toml");

        if path.exists() {
            bail!(ErrorKind::AlreadyInitialized);
        }

        let mut file = File::create(path)?;

        let day_commit = DayCommit {
            date,
            start_time: time,
            end_time: None,
            message: None,
            participants: vec![],
        };

        file.write_all(toml::to_string_pretty(&day_commit)?.as_bytes())?;

        Ok(day_commit)
    }

    pub fn get_working_file(&self) -> Result<::std::fs::File> {
        use std::fs::OpenOptions;

        let mut path = PathBuf::from(&self.data_path);
        path.push("working.toml");

        if !path.exists() {
            bail!(ErrorKind::NotInitialized);
        }

        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| ErrorKind::Io(e).into())
    }

    pub fn edit_working_commit<F>(&self, f: F) -> Result<DayCommit>
    where
        F: FnOnce(DayCommit) -> DayCommit,
    {
        let mut file = self.get_working_file()?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut day_commit: DayCommit = toml::from_str(&content)?;

        day_commit = f(day_commit);

        file.write_all(toml::to_string_pretty(&day_commit)?.as_bytes())?;

        Ok(day_commit)
    }

    pub fn get_working_commit(&self) -> Result<DayCommit> {
        let mut file = self.get_working_file()?;
        App::get_commit_from_file(&mut file)
    }

    pub fn remove_working_commit(&self) -> Result<()> {
        use std::fs::remove_file;

        let mut path = PathBuf::from(&self.data_path);
        path.push("working.toml");

        if !path.exists() {
            bail!(ErrorKind::NotInitialized);
        }
        remove_file(path).map_err(|e| ErrorKind::Io(e).into())
    }

    pub fn commit_a_day(&self, end_time: Time, message: String) -> Result<DayCommit> {
        use std::fs::create_dir_all;

        let mut origin = self.get_working_file()?;

        let mut content = String::new();
        origin.read_to_string(&mut content)?;

        let mut day_commit: DayCommit = toml::from_str(&content)?;

        day_commit.end_time = Some(end_time);
        day_commit.message = Some(message);

        let mut path = PathBuf::from(&self.data_path);
        path.push("working");
        path.push(day_commit.date.day.to_string());
        path.set_extension("toml");
        create_dir_all(&path)?;

        let mut commit_file = File::create(&path)?;
        commit_file.write_all(toml::to_string_pretty(&day_commit)?.as_bytes())?;

        self.remove_working_commit()?;

        Ok(day_commit)
    }

    pub fn get_working_directory_commit(&self) -> Result<Vec<DayCommit>> {
        let dir = self.get_working_directory_entries()?;
        Ok(
            dir.filter_map(|f| {
                f.ok()
                    .and_then(|f| File::open(f.path()).ok())
                    .and_then(|mut f| App::get_commit_from_file(&mut f).ok())
            }).collect(),
        )
    }

    pub fn get_working_directory_entries(&self) -> Result<::std::fs::ReadDir> {
        use std::fs::read_dir;

        let mut path = PathBuf::from(&self.data_path);
        path.push("working");

        read_dir(path).map_err(|e| ErrorKind::Io(e).into())
    }

    pub fn push_a_month(&self) -> Result<()> {
        use std::fs::{create_dir_all, copy, remove_file};

        let mut dir = self.get_working_directory_entries()?;
        let first_day: DayCommit = App::get_commit_from_path(dir.next().unwrap()?.path())?;

        let mut path = PathBuf::from(&self.data_path);
        path.push(first_day.date.year.to_string());
        path.push(first_day.date.month.to_string());
        create_dir_all(&path)?;

        for d in dir {
            let origin = d?.path();
            let mut target = path.clone();
            target.push(origin.file_name().unwrap());
            copy(&origin, target)?;
            remove_file(origin)?;
        }

        Ok(())
    }
}