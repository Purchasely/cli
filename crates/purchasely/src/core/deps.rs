use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: u64,
    pub refuse_redirect: bool,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: String::new(),
            url: String::new(),
            headers: Vec::new(),
            body: None,
            timeout_ms: 120_000,
            refuse_redirect: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
#[derive(Clone, Debug, Default)]
pub struct NetError;
#[derive(Clone, Debug, Default)]
pub struct ExecOpts;
#[derive(Clone, Debug, Default)]
pub struct ExecResult;
#[derive(Clone, Debug, Default)]
pub enum ExecError { #[default] NotFound }
pub trait Keychain {}
pub trait Io {}

pub trait Host {
    fn fetch(&self, req: Request) -> Result<Response, NetError>;
    fn exec(&self, file: &str, args: &[String], opts: ExecOpts) -> Result<ExecResult, ExecError>;
    fn prompt(&self, question: &str, hidden: bool) -> String;
    fn read_stdin(&self) -> String;
    fn now(&self) -> i64;
    fn wall_now(&self) -> i64;
    fn pid_alive(&self, pid: u32) -> bool;
    fn sleep(&self, ms: i64);
    fn stdout(&self, s: &str);
    fn stderr(&self, s: &str);
    fn keychain(&self) -> Option<&dyn Keychain>;
    fn io(&self) -> Option<&dyn Io>;
}

pub struct Deps {
    pub host: Box<dyn Host>,
    pub env: BTreeMap<String, String>,
    pub is_tty: bool,
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub version: String,
    pub pid: u32,
    pub name: String,
    pub host_url: String,
    pub ext: Option<super::ext::Extension>,
    pub self_exe: PathBuf,
    pub platform: &'static str,
    pub stdout_tty: bool,
    pub columns: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TrivialHost;
    impl Host for TrivialHost {
        fn fetch(&self, _: Request) -> Result<Response, NetError> { Ok(Response::default()) }
        fn exec(&self, _: &str, _: &[String], _: ExecOpts) -> Result<ExecResult, ExecError> { Ok(ExecResult) }
        fn prompt(&self, _: &str, _: bool) -> String { String::new() }
        fn read_stdin(&self) -> String { String::new() }
        fn now(&self) -> i64 { 0 }
        fn wall_now(&self) -> i64 { 0 }
        fn pid_alive(&self, _: u32) -> bool { false }
        fn sleep(&self, _: i64) {}
        fn stdout(&self, _: &str) {}
        fn stderr(&self, _: &str) {}
        fn keychain(&self) -> Option<&dyn Keychain> { None }
        fn io(&self) -> Option<&dyn Io> { None }
    }

    #[test]
    fn builds_deps() {
        let deps = Deps {
            host: Box::new(TrivialHost), env: BTreeMap::new(), is_tty: false,
            config_dir: PathBuf::new(), state_dir: PathBuf::new(), cache_dir: PathBuf::new(),
            version: "0.7.2".into(), pid: 1, name: "purchasely".into(),
            host_url: "https://cli.purchasely.io".into(), ext: None,
            self_exe: PathBuf::new(), platform: "linux", stdout_tty: false, columns: None,
        };
        assert_eq!(deps.name, "purchasely");
        assert_eq!(deps.host.now(), 0);
    }
}
