//! Owner: P1b
use purchasely::core::deps::{
    ExecError, ExecOpts, ExecResult, Host, Keychain, NetError, Request, Response,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayError(pub String);

/// Ordered, transcript-backed effects for one recorded execution step.
pub struct FakeHost {
    pub calls: RefCell<VecDeque<Value>>,
    pub execs: RefCell<VecDeque<Value>>,
    pub answers: RefCell<VecDeque<Value>>,
    pub probes: RefCell<VecDeque<bool>>,
    pub wall: RefCell<i64>,
    pub now: RefCell<i64>,
    pub stdout: RefCell<String>,
    pub stderr: RefCell<String>,
    pub prompts: RefCell<Vec<Value>>,
    pub keychain_enabled: bool,
    pub error: RefCell<Option<ReplayError>>,
}

impl FakeHost {
    pub fn from_step(step: &Value) -> Self {
        let arr = |key: &str| {
            step.get(key)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        Self {
            calls: RefCell::new(arr("calls").into()),
            execs: RefCell::new(arr("execs").into()),
            answers: RefCell::new(arr("answers").into()),
            probes: RefCell::new(
                arr("pid_probes")
                    .into_iter()
                    .filter_map(|v| v.as_bool())
                    .collect(),
            ),
            wall: RefCell::new(step["wall"].as_i64().unwrap_or(0)),
            now: RefCell::new(step["t_before"].as_i64().unwrap_or(0)),
            stdout: RefCell::new(String::new()),
            stderr: RefCell::new(String::new()),
            prompts: RefCell::new(Vec::new()),
            keychain_enabled: !step.get("keychain").is_some_and(Value::is_null),
            error: RefCell::new(None),
        }
    }
    fn fail(&self, msg: impl Into<String>) {
        *self.error.borrow_mut() = Some(ReplayError(msg.into()));
    }
    pub fn finish(&self) -> Result<(), ReplayError> {
        if let Some(e) = self.error.borrow().clone() {
            return Err(e);
        }
        if !self.calls.borrow().is_empty() {
            return Err(ReplayError(format!(
                "{} fetch call(s) were not consumed",
                self.calls.borrow().len()
            )));
        }
        if !self.execs.borrow().is_empty() {
            return Err(ReplayError(format!(
                "{} exec call(s) were not consumed",
                self.execs.borrow().len()
            )));
        }
        Ok(())
    }
}

impl Host for FakeHost {
    fn fetch(&self, req: Request) -> Result<Response, NetError> {
        let Some(expected) = self.calls.borrow_mut().pop_front() else {
            self.fail("unexpected fetch call");
            return Err(NetError);
        };
        if expected["method"].as_str().is_some_and(|v| v != req.method)
            || expected["url"].as_str().is_some_and(|v| v != req.url)
        {
            self.fail(format!("fetch differs: {} {}", req.method, req.url));
            return Err(NetError);
        }
        let reply = &expected["reply"];
        if let Some(err) = reply["error"].as_str() {
            self.fail(err);
            return Err(NetError);
        }
        Ok(Response {
            status: reply["status"].as_u64().unwrap_or(200) as u16,
            headers: pairs(&reply["headers"]),
            body: reply["text"]
                .as_str()
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
        })
    }
    fn exec(&self, file: &str, args: &[String], _: ExecOpts) -> Result<ExecResult, ExecError> {
        let Some(expected) = self.execs.borrow_mut().pop_front() else {
            self.fail("unexpected exec call");
            return Err(ExecError::NotFound);
        };
        let ef = expected["file"].as_str().unwrap_or_default();
        let ea: Vec<String> = serde_json::from_value(expected["args"].clone()).unwrap_or_default();
        if ef != file && ef != "<SELF>" || ea != args {
            self.fail(format!("exec differs: {file} {args:?}"));
            return Err(ExecError::NotFound);
        }
        if expected["error"].is_string() {
            return Err(ExecError::NotFound);
        }
        Ok(ExecResult)
    }
    fn prompt(&self, question: &str, hidden: bool) -> String {
        self.prompts
            .borrow_mut()
            .push(json!({"text":question,"hidden":hidden}));
        self.answers
            .borrow_mut()
            .pop_front()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    }
    fn read_stdin(&self) -> String {
        String::new()
    }
    fn now(&self) -> i64 {
        *self.now.borrow()
    }
    fn wall_now(&self) -> i64 {
        *self.wall.borrow()
    }
    fn pid_alive(&self, _: u32) -> bool {
        self.probes.borrow_mut().pop_front().unwrap_or(false)
    }
    fn sleep(&self, ms: i64) {
        *self.wall.borrow_mut() += ms;
        *self.now.borrow_mut() += ms;
    }
    fn stdout(&self, s: &str) {
        self.stdout.borrow_mut().push_str(s);
    }
    fn stderr(&self, s: &str) {
        self.stderr.borrow_mut().push_str(s);
    }
    fn keychain(&self) -> Option<&dyn Keychain> {
        None
    }
    fn io(&self) -> Option<&dyn purchasely::core::deps::Io> {
        None
    }
}
fn pairs(v: &Value) -> Vec<(String, String)> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| Some((p[0].as_str()?.into(), p[1].as_str()?.into())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extra_call_fails() {
        let h = FakeHost::from_step(&json!({}));
        h.fetch(Request::default()).unwrap_err();
        assert!(h.finish().unwrap_err().0.contains("unexpected fetch"));
    }
    #[test]
    fn missing_call_fails() {
        let h = FakeHost::from_step(&json!({"calls":[{}]}));
        assert!(h.finish().unwrap_err().0.contains("not consumed"));
    }
    #[test]
    fn keychain_failure_replays() {
        let h = FakeHost::from_step(&json!({"keychain":null}));
        assert!(h.keychain().is_none());
    }
    #[test]
    fn run_child_reruns() {
        let h = FakeHost::from_step(
            &json!({"execs":[{"file":"node","args":["child"],"run_child":true}]}),
        );
        assert!(h.exec("node", &["child".into()], ExecOpts).is_ok());
        h.finish().unwrap();
    }
    #[test]
    fn pid_alive_replays() {
        let h = FakeHost::from_step(&json!({"pid_probes":[true,false]}));
        assert!(h.pid_alive(1));
        assert!(!h.pid_alive(1));
    }
    #[test]
    fn sleep_advances_wall() {
        let h = FakeHost::from_step(&json!({"wall":100}));
        h.sleep(25);
        assert_eq!(h.wall_now(), 125);
    }
}
