# 0014 Tokens have their own file, and a refused one is not sent

- status: accepted
- date: 2026-10-05
- spec: ../specs/credentials.md

## Context

GitHub allows 60 requests an hour to an anonymous client and 5,000 to one with a token; the catalog has hundreds of repositories. Quiver
lets the user enter a token (read from its source on 2026-10-05):

* It is a text box in Settings with Save, Create (opens GitHub's page with a name filled in), Clear and a how-to. No scope is needed
  for public releases.
* It is stored as plain text in `settings.json`, and never checked: a typo is discovered when downloads start failing.
* It goes to `api.github.com` as `Authorization: Bearer`; GitLab's goes as `PRIVATE-TOKEN`. Release requests are coalesced per
  address and credential, queued per provider (interactive ahead of background), cached with `ETag` per credential, and a rate limit
  starts a cool-down per credential from `Retry-After` / `x-ratelimit-reset` or an exponential 60 s.
* The platform index (published metadata) lets the catalog show what each project supports with no API call at all.
* A banner offers the token once the limit is hit, and can be snoozed or silenced.

Reclaw loads the same catalog and the same index, and today sends nothing to `api.github.com`: the catalog comes from
`raw.githubusercontent.com`, which the token does not affect. The token matters when releases are fetched (M2), which is also when
its value shows. This decision builds what that needs, now, while it can be tested against the live service.

## Decision

* **Its own file**, `secrets.toml` beside the settings, created readable by its owner only, replaced atomically, tightened if found
  loose. The settings file is what gets pasted into bug reports.
* **Sent to one host only** (`api.github.com`, `gitlab.com`), over https, as `Authorization: Bearer` for both: the HTTP client removes
  exactly that header when a redirect changes host, so a release asset's redirect to signed storage cannot carry it.
* **Checked when pasted** (`GET /rate_limit` for GitHub, free against the limit; `/personal_access_tokens/self` for GitLab). One the
  service refuses is not kept, so a typo does not become a mystery later. The answer is shown: "4,987 of 5,000 left".
* **A token refused later (expired, revoked) stops being sent**, and the request is repeated without it, because GitHub answers 401 even
  for public data: a dead token is worse than none. The token stays saved so the person can see what happened; they are told once.
* **Saved beats the environment** (`GITHUB_TOKEN`), which is the fallback and is labelled in Settings. Removing the saved one returns to it.
* Changing the token forgets a rate-limit wait: the new identity has its own allowance.
* Answers are cached per identity, and two askers for the same cached answer cost one request.
* A classic token with permissions gets a warning (`X-OAuth-Scopes`): Reclaw needs none.

## Rejected

* **Plain text in the settings file, as Quiver does.** The settings file is shared in bug reports, and this very session needed a rule
  that a token is never printed. Cheap to avoid.
* **The OS keyring.** On a Steam Deck in game mode there is often no unlocked Secret Service, so it fails exactly where the token matters;
  it adds a D-Bus dependency; and a token with no permissions can read only public data, so the value protected is small. A private file
  is proportionate. A keyring backend can replace the file later behind the same `Secrets` interface.
* **`PRIVATE-TOKEN` for GitLab, as Quiver does.** Proven there, but the client forwards a custom header across a redirect to another
  host. GitLab documents `Authorization: Bearer` for personal tokens; this is **not yet verified against a real GitLab token**.
* **Sending the token to `github.com` or `raw.githubusercontent.com`, as Quiver's icon fetch does.** It buys nothing for public
  content and sends a credential to hosts that redirect to storage.
* **Keeping a token the service refused at paste time.** The kind thing for the next hour, the confusing thing for the next week.
* **GraphQL batching** (one query for many repositories' latest release, one point of the GraphQL allowance instead of one REST request
  each): the strongest speed-up available, but GraphQL requires a token, and its cost model cannot be checked from the development
  sandbox. Proposed for M2 rather than built blind.

## Verified, and not

Verified against the live service (through the sandbox's gateway account, which re-authenticates `api.github.com`): the allowance
check, its fields, the expiry header, and that a `304 Not Modified` costs nothing against the allowance. **Not** verified live: the 401
for a refused token, `X-OAuth-Scopes` on a real classic token, and everything GitLab does with a token. Those rest on the local-server
tests and on the services' documentation.
