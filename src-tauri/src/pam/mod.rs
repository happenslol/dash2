pub mod converse;
mod env;
mod ffi;
pub mod session;

use std::cell::RefCell;

use thiserror::Error as ThisError;

use pam_sys::PamReturnCode;

#[derive(Debug, ThisError)]
pub enum PamError {
  #[error("{0}")]
  Error(String),
  #[error("{0}")]
  AuthError(String),
  #[error("abort error: {0}")]
  AbortError(String),
}

impl PamError {
  pub fn from_rc(prefix: &str, rc: PamReturnCode) -> PamError {
    match rc {
      PamReturnCode::ABORT => PamError::AbortError(format!("{}: {:?}", prefix, rc)),
      PamReturnCode::AUTH_ERR
      | PamReturnCode::MAXTRIES
      | PamReturnCode::CRED_EXPIRED
      | PamReturnCode::ACCT_EXPIRED
      | PamReturnCode::CRED_INSUFFICIENT
      | PamReturnCode::USER_UNKNOWN
      | PamReturnCode::PERM_DENIED
      | PamReturnCode::SERVICE_ERR => PamError::AuthError(format!("{}: {:?}", prefix, rc)),
      _ => PamError::Error(format!("{}: {:?}", prefix, rc)),
    }
  }
}

pub struct PasswordConv(RefCell<Option<String>>);

impl PasswordConv {
  pub fn new(password: String) -> Self {
    Self(RefCell::new(Some(password)))
  }
}

impl converse::Converse for PasswordConv {
  fn prompt_echo(&self, _msg: &str) -> Result<String, ()> {
    Ok(String::new())
  }

  fn prompt_blind(&self, _msg: &str) -> Result<String, ()> {
    Ok(self.0.borrow_mut().take().unwrap_or_default())
  }

  fn info(&self, msg: &str) -> Result<(), ()> {
    eprintln!("pam info: {msg}");
    Err(())
  }

  fn error(&self, msg: &str) -> Result<(), ()> {
    eprintln!("pam error: {msg}");
    Err(())
  }
}
