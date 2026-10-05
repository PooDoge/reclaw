//! What the network layer writes to the log: enough to find a failed request afterwards (which host, which path, how many tries,
//! what the server said), and nothing that would be a credential. The logger is process-wide, so this is one test.
use std::fs;

use reclaw_log::{LogConfig, LogLevel, init};
use reclaw_net::{
    Net, NetError, Request,
    testing::{Reply, TestServer, local_config},
};

#[test]
fn requests_leave_a_trail_and_never_a_credential() {
    let dir = tempfile::tempdir().expect("tempdir");
    let logging = init(LogConfig { dir: Some(dir.path().join("logs")), level: LogLevel::Detailed, filter: None, stderr: false });
    let log = || fs::read_to_string(logging.file().expect("a log file")).expect("read the log");

    // A good answer: one line, naming host and path, and not the query string (that is where a credential would be).
    let server = TestServer::start(|_, _| Reply::ok("hello"));
    let net = Net::new(local_config()).expect("net");
    net.fetch(&Request::get(server.url("/catalog/index.json?access_token=hunter2hunter2&x=1"))).expect("fetched");
    let text = log();
    let line = text.lines().find(|l| l.contains("fetched")).unwrap_or_else(|| panic!("a fetch line in:\n{text}"));
    assert!(
        line.contains("/catalog/index.json") && line.contains("status=200") && line.contains("bytes=5") && line.contains("ms="),
        "{line}"
    );
    assert!(!text.contains("hunter2") && !text.contains("access_token"), "the query never reaches the log:\n{text}");

    // A flaky server: each failed try is a warning with the attempt count, so the log shows why a request took long.
    let flaky = TestServer::start(|_, n| if n < 2 { Reply::new(503, "busy") } else { Reply::ok("fine") });
    let mut config = local_config();
    config.attempts = 3;
    Net::new(config).expect("net").fetch(&Request::get(flaky.url("/flaky"))).expect("third time");
    let retries: Vec<String> = log().lines().filter(|l| l.contains("trying again")).map(str::to_string).collect();
    assert_eq!(retries.len(), 2, "{retries:?}");
    assert!(retries[0].contains("attempt=1") && retries[0].contains("of=3") && retries[0].contains("WARN"), "{}", retries[0]);

    // A refusal: the final error and the hint a person would act on.
    let missing = TestServer::start(|_, _| Reply::new(404, "nope"));
    let error = net.fetch(&Request::get(missing.url("/gone"))).expect_err("a 404");
    assert!(matches!(error, NetError::Status { status: 404, .. }));
    let text = log();
    let failure =
        text.lines().find(|l| l.contains("fetch failed") && l.contains("/gone")).unwrap_or_else(|| panic!("a failure line in:\n{text}"));
    assert!(failure.contains("WARN") && failure.contains("404"), "{failure}");

    // A long rate limit is remembered and logged once with how long and how many requests were left.
    let limited = TestServer::start(|_, _| Reply::new(429, "slow down").header("Retry-After", "120").header("X-RateLimit-Remaining", "0"));
    net.fetch(&Request::get(limited.url("/a"))).expect_err("limited");
    let text = log();
    let limit_line = text.lines().find(|l| l.contains("rate limit reached")).unwrap_or_else(|| panic!("a rate-limit line in:\n{text}"));
    assert!(limit_line.contains("wait_s=120") && limit_line.contains("remaining=0"), "{limit_line}");
}
