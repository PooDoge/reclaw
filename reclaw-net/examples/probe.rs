//! Contact every host the launcher depends on, from this machine, and say what happened to each.
//!
//!   cargo run -p reclaw-net --example probe              # the built-in list
//!   cargo run -p reclaw-net --example probe -- URL...    # your own addresses
//!
//! Run it where the problem is (on the Bazzite machine, say). It reads `HTTPS_PROXY`, `SSL_CERT_FILE`, `RECLAW_PROXY` and
//! `GITHUB_TOKEN` like the program does, so it shows the program's view of the network, not curl's.
use reclaw_net::{Net, NetConfig, diagnose::HOSTS};

fn main() {
    let (config, problems) = NetConfig::from_env(|k| std::env::var(k).ok());
    for problem in &problems {
        eprintln!("note: {problem}");
    }
    eprintln!("{config:?}\n");
    let net = match Net::new(config) {
        Ok(net) => net,
        Err(e) => {
            eprintln!("could not start the network layer: {e}");
            std::process::exit(2);
        }
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let targets: Vec<(String, String)> = if args.is_empty() {
        HOSTS.iter().map(|(u, what)| (u.to_string(), what.to_string())).collect()
    } else {
        args.into_iter().map(|u| (u, String::new())).collect()
    };
    let mut failures = 0;
    for (url, what) in targets {
        let probe = net.probe(&url);
        if probe.result.is_err() {
            failures += 1;
        }
        println!("{what}\n  {url}\n  {}\n", probe.summary());
        if let Ok(reached) = &probe.result
            && let Some(csp) = &reached.content_security_policy
        {
            println!("  (sends `content-security-policy: {csp}`, which only a browser acts on)\n");
        }
    }
    std::process::exit(if failures == 0 { 0 } else { 1 });
}
