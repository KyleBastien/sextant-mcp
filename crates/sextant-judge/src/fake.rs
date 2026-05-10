//! Test-only judge that returns canned responses.
//!
//! Used by `sextant-rules` tests and the engine integration tests to
//! exercise the LLM rule path without hitting the network. Records every
//! request it receives so tests can assert on prompt rendering.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::{JudgeError, JudgeProvider, JudgeRequest, JudgeResult};

pub struct FakeJudge {
    name: &'static str,
    response: Mutex<Vec<JudgeResult>>,
    received: Mutex<Vec<RecordedRequest>>,
}

#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub user_prompt: String,
    pub model: String,
}

impl FakeJudge {
    pub fn new(name: &'static str, responses: Vec<JudgeResult>) -> Self {
        Self {
            name,
            response: Mutex::new(responses),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn always(name: &'static str, result: JudgeResult) -> Self {
        Self::new(name, vec![result])
    }

    pub fn received(&self) -> Vec<RecordedRequest> {
        self.received.lock().unwrap().clone()
    }
}

#[async_trait]
impl JudgeProvider for FakeJudge {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn judge(&self, req: JudgeRequest<'_>) -> Result<JudgeResult, JudgeError> {
        self.received.lock().unwrap().push(RecordedRequest {
            user_prompt: req.user_prompt.to_string(),
            model: req.model.to_string(),
        });
        let mut q = self.response.lock().unwrap();
        if q.is_empty() {
            return Err(JudgeError::Parse("no canned response left".into()));
        }
        // Re-use the last response if we run out. Tests that need a
        // distinct sequence can supply enough entries up front.
        if q.len() == 1 {
            Ok(q[0].clone())
        } else {
            Ok(q.remove(0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JudgeFinding, JudgeResult, JudgeSeverity};

    fn req(prompt: &str) -> JudgeRequest<'_> {
        JudgeRequest {
            system_prompt: None,
            user_prompt: prompt,
            model: "m",
            max_tokens: 8,
            temperature: 0.0,
        }
    }

    fn one() -> JudgeResult {
        JudgeResult {
            findings: vec![JudgeFinding {
                severity: JudgeSeverity::Info,
                message: "x".into(),
                line: None,
                end_line: None,
                patch: None,
            }],
            patch: None,
        }
    }

    #[tokio::test]
    async fn always_returns_same_response_on_repeat_calls() {
        let f = FakeJudge::always("p", one());
        f.judge(req("a")).await.unwrap();
        f.judge(req("b")).await.unwrap();
        assert_eq!(f.received().len(), 2);
    }

    #[tokio::test]
    async fn new_pops_responses_in_order() {
        let two = JudgeResult {
            findings: vec![],
            patch: None,
        };
        let f = FakeJudge::new("p", vec![one(), two.clone()]);
        let a = f.judge(req("x")).await.unwrap();
        let b = f.judge(req("y")).await.unwrap();
        assert_eq!(a, one());
        assert_eq!(b, two);
    }

    #[tokio::test]
    async fn empty_queue_produces_an_error() {
        let f = FakeJudge::new("p", vec![]);
        assert!(f.judge(req("x")).await.is_err());
    }
}
