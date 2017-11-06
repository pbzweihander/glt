#[derive(Deserialize)]
pub struct Settings {
    pub verification_token: String,
    pub api_token: String,
    pub data_path: String,
}

impl Settings {
    pub fn try_new() -> Result<Settings, ::config::ConfigError> {
        use std::env::args;
        use std::path::Path;
        use config::{Config, File};
        let mut settings = Config::new();
        settings.merge(if args().len() >= 2 {
            File::from(Path::new(&args().nth(1).unwrap()))
        } else {
            File::with_name("settings")
        })?;
        settings.try_into::<Settings>()
    }

    pub fn assure_new() -> Settings {
        use std::fs::read_dir;
        let mut s = match Settings::try_new() {
            Ok(s) => s,
            Err(e) => panic!("Settings file parse error!, {}", e),
        };
        if let Err(e) = read_dir(&s.data_path) {
            panic!("Invalid data folder. Check settings file!, {}", e);
        }
        if !s.data_path.ends_with("/") {
            s.data_path.push('/');
        }
        s
    }
}
