use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum ServerError{
    EnvNotFound(Box<dyn Error>),

}

impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ServerError::EnvNotFound(e) => write!(f, "Environment variable not found: {}", e),
        }
    }
}

impl Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}