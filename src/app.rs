use super::{ErrorKind, Result};
use std::fs::{create_dir_all, File, OpenOptions};
use std::path::PathBuf;
use std::ops::Sub;
use serde_json;
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
pub struct Date(pub i32, pub u32, pub u32);

impl From<Date> for cDate<Local> {
    fn from(d: Date) -> cDate<Local> {
        use chrono::prelude::*;
        Local.ymd(d.0, d.1, d.2)
    }
}

impl From<cDate<Local>> for Date {
    fn from(d: cDate<Local>) -> Date {
        Date(d.year(), d.month(), d.day())
    }
}

impl ::std::fmt::Display for Date {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}년 {}월 {}일", self.0, self.1, self.2)
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Time(pub u32, pub u32);

pub struct TimeDiff(pub i32, pub i32);

impl From<Time> for ::chrono::NaiveTime {
    fn from(t: Time) -> ::chrono::NaiveTime {
        ::chrono::NaiveTime::from_hms(t.0, t.1, 0)
    }
}

impl From<::chrono::NaiveTime> for Time {
    fn from(t: ::chrono::NaiveTime) -> Time {
        Time(t.hour(), t.minute())
    }
}

impl From<Time> for f32 {
    fn from(t: Time) -> f32 {
        (t.0 as f32) + ((t.1 as f32) / 60f32)
    }
}

impl From<f32> for Time {
    fn from(f: f32) -> Time {
        Time(f as u32, ((f * 60f32) as u32) % 60)
    }
}

impl<'a> From<&'a Time> for f32 {
    fn from(t: &'a Time) -> f32 {
        (t.0 as f32) + ((t.1 as f32) / 60f32)
    }
}

impl From<TimeDiff> for f32 {
    fn from(t: TimeDiff) -> f32 {
        (t.0 as f32) + ((t.1 as f32) / 60f32)
    }
}

impl From<f32> for TimeDiff {
    fn from(f: f32) -> TimeDiff {
        TimeDiff(f as i32, ((f * 60f32) as i32) % 60)
    }
}

impl Sub for Time {
    type Output = TimeDiff;
    fn sub(self, rhs: Self) -> Self::Output {
        let self_f: f32 = self.into();
        let rhs_f: f32 = rhs.into();
        let diff: f32 = self_f - rhs_f;
        diff.into()
    }
}

impl<'a> Sub for &'a Time {
    type Output = TimeDiff;
    fn sub(self, rhs: Self) -> Self::Output {
        let self_f: f32 = self.into();
        let rhs_f: f32 = rhs.into();
        let diff: f32 = self_f - rhs_f;
        diff.into()
    }
}

impl ::std::fmt::Display for Time {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}시 {}분", self.0, self.1)
    }
}

impl ::std::fmt::Display for TimeDiff {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}시간 {}분", self.0, self.1)
    }
}

impl Time {
    pub fn to_short_str(&self) -> String {
        format!("{}:{}", self.0, self.1)
    }
}

impl TimeDiff {
    pub fn to_short_str(&self) -> String {
        format!("{}:{}", self.0, self.1)
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

    fn get_commit_from_file(file: &File) -> Result<DayCommit> {
        serde_json::from_reader(file).map_err(|e| ErrorKind::Json(e).into())
    }

    fn get_commit_from_path(path: PathBuf) -> Result<DayCommit> {
        let file = File::open(path)?;
        App::get_commit_from_file(&file)
    }

    pub fn create_working_file(&self, date: Date, time: Time) -> Result<DayCommit> {
        let mut path = PathBuf::from(&self.data_path);
        path.push("working.json");

        if path.exists() {
            bail!(ErrorKind::AlreadyInitialized);
        }

        let file = File::create(path)?;

        let day_commit = DayCommit {
            date,
            start_time: time,
            end_time: None,
            message: None,
            participants: vec![],
        };

        serde_json::to_writer_pretty(file, &day_commit)?;

        Ok(day_commit)
    }

    pub fn get_working_file(&self, option: &mut OpenOptions) -> Result<::std::fs::File> {
        let mut path = PathBuf::from(&self.data_path);
        path.push("working.json");

        if !path.exists() {
            bail!(ErrorKind::NotInitialized);
        }

        option.open(&path).map_err(|e| ErrorKind::Io(e).into())
    }

    pub fn edit_working_commit<F>(&self, f: F) -> Result<DayCommit>
    where
        F: FnOnce(DayCommit) -> DayCommit,
    {
        let mut day_commit: DayCommit = self.get_working_commit()?;

        day_commit = f(day_commit);

        let file: File = self.get_working_file(OpenOptions::new().write(true).truncate(true))?;
        serde_json::to_writer_pretty(file, &day_commit)?;

        Ok(day_commit)
    }

    pub fn get_working_commit(&self) -> Result<DayCommit> {
        let file = self.get_working_file(OpenOptions::new().read(true))?;
        App::get_commit_from_file(&file)
    }

    pub fn remove_working_commit(&self) -> Result<()> {
        use std::fs::remove_file;

        let mut path = PathBuf::from(&self.data_path);
        path.push("working.json");

        if !path.exists() {
            bail!(ErrorKind::NotInitialized);
        }
        remove_file(path).map_err(|e| ErrorKind::Io(e).into())
    }

    pub fn commit_a_day(&self, end_time: Time, message: String) -> Result<DayCommit> {
        let mut day_commit: DayCommit = self.get_working_commit()?;

        day_commit.end_time = Some(end_time);
        day_commit.message = Some(message);

        let mut path = PathBuf::from(&self.data_path);
        path.push("working");
        create_dir_all(&path)?;
        path.push(day_commit.date.2.to_string());
        path.set_extension("json");

        let mut i: usize = 1;
        while path.exists() {
            path.pop();
            path.push(day_commit.date.2.to_string() + "_" + &i.to_string());
            path.set_extension("json");
            i += 1;
        }

        let commit_file = File::create(&path)?;
        serde_json::to_writer_pretty(commit_file, &day_commit)?;

        self.remove_working_commit()?;

        Ok(day_commit)
    }

    pub fn get_working_directory_commit(&self) -> Result<Vec<DayCommit>> {
        let dir = self.get_working_directory_entries()?;
        Ok(
            dir.into_iter()
                .filter_map(|f| {
                    File::open(f.path())
                        .ok()
                        .and_then(|f| App::get_commit_from_file(&f).ok())
                })
                .collect(),
        )
    }

    pub fn get_working_directory_entries(&self) -> Result<Vec<::std::fs::DirEntry>> {
        use std::fs::read_dir;

        let mut path = PathBuf::from(&self.data_path);
        path.push("working");
        if !path.exists() {
            bail!(ErrorKind::NotInitialized);
        }

        Ok(read_dir(path)?.filter_map(|d| d.ok()).collect())
    }

    pub fn push_a_month(&self) -> Result<()> {
        use std::fs::{copy, remove_file};

        let dir = self.get_working_directory_entries()?;
        let first_day: DayCommit = App::get_commit_from_path(dir[0].path())?;

        let mut path = PathBuf::from(&self.data_path);
        path.push(first_day.date.0.to_string());
        path.push(first_day.date.1.to_string());
        create_dir_all(&path)?;

        for d in dir {
            let origin = d.path();
            let mut target = path.clone();
            target.push(origin.file_name().unwrap());
            copy(&origin, target)?;
            remove_file(origin)?;
        }

        Ok(())
    }
}
