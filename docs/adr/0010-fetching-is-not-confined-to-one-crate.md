# 0010 Fetching is not confined to one crate

- status: accepted
- date: 2026-10-05
- spec: ../specs/network.md

## Context

ADR 0003 made `reclaw-media` the only code that fetches, and AGENTS.md carried it as a hard rule. That was right while the only thing
fetched was pictures and READMEs written by strangers. The launcher now has to read a catalog, ask GitHub and GitLab for releases, download
installers of hundreds of megabytes and talk to mod sites. Those are first-party endpoints and large files, and `reclaw-media`'s shape
(a `MediaUrl` that refuses any port but 443, a body capped in memory, a picture cache) fits none of them. The rule would have meant either
bending the media crate into a general downloader or breaking the rule.

## Decision

Any crate that is not the UI may fetch, through one shared HTTP client (`reclaw-net`, ADR 0011), so that the user agent, proxy and
certificate settings, retries, conditional requests and resumable downloads exist once. What stays:

* **Components never fetch.** This is the existing rule about the outside world (an `Effect`, or a hook backed by a worker), not a new one.
* **One address policy for everything** (https, standard port, no credentials, nothing on the local network, rechecked on every
  redirect). It costs first-party fetches nothing, since every host they use is public https, and it means no code path can be sent
  to a private address by a redirect. Size caps and kind detection stay in `reclaw-media`, for what strangers write.
* **The toolkit's own fetching stays off** (ADR 0003): it has no limits at all.

## Rejected

* **Keep the rule and widen `reclaw-media`.** One crate would carry a picture cache and a resumable multi-gigabyte downloader and a
  catalog client, and its tests would stop being about one thing.
* **Keep the rule with a list of exceptions.** A rule with a growing list of exceptions says nothing, and the list would be the real policy.
* **No shared client; each crate builds its own `reqwest` client.** Five user agents, five places to forget the proxy variable or the extra
  certificate bundle, and five behaviours when GitHub asks to slow down.

## Consequences

AGENTS.md loses its rule 12 and ADR 0003's "only crate" sentence no longer binds (its safety reasoning for stranger-chosen addresses does).
A new network call needs no exemption and gets no choice: it goes through `Net`, and `Net` applies the policy.
