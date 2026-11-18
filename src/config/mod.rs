use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub database_url: String,
    pub redis_url: String,
}

impl Settings {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL")?,
        })
    }
}
