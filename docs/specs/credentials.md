# Access tokens

- last-verified: 2026-10-05
- owner-paths: reclaw-net/src/credentials.rs, reclaw-net/src/check.rs, reclaw-net/tests/credentials.rs, reclaw-net/tests/live.rs, reclaw-config/src/secrets.rs, reclaw-app/src/host/credentials.rs, reclaw-ui/src/credentials.rs, reclaw-ui/tests/ui/credentials.rs

A token raises GitHub's limit from 60 requests an hour to 5,000 and needs no permissions. GitLab's is optional (`read_api`, for private
projects and higher limits). Decisions and the Quiver comparison are in ADR 0014. **Nothing in the program sends a request to
`api.github.com` yet except the check below; the first consumer is release fetching (M2).**

## Where a token comes from

1. **Saved**, pasted in Settings, Network, kept in `secrets.toml` in the config folder: created `0600`, replaced through a private
   temporary file, tightened (and the user told) if found readable by others. A damaged file is kept as `secrets.toml.bad` and the program
   starts without tokens; one from a newer Reclaw or one that cannot be read is left alone and never written over.
2. **The environment**: `RECLAW_GITHUB_TOKEN` or `GITHUB_TOKEN`, `RECLAW_GITLAB_TOKEN` or `GITLAB_TOKEN`. Used only when nothing is saved;
   Settings says which variable ("From GITHUB_TOKEN"). Removing the saved token returns to it.

## What a token does

* Goes to **one host** (`api.github.com`, `gitlab.com`), over https, as `Authorization: Bearer`, marked sensitive. The client removes
  that header when a redirect changes host. Not sent to `github.com`, `raw.githubusercontent.com` or any download host.
* Separates the cache: what a token fetched is never served to a request without it (`anon` or a 12-hex fingerprint is part of the key).
* **Changing it** (`Net::set_token`) takes effect on the next request and forgets a rate-limit wait recorded anonymously.
* **A service that refuses it** (`401`) stops receiving it: the token is marked rejected, the request is repeated without it, the
  host is told once (a notice: "GitHub refused the token"; the status line reads "GitHub refused it"), and it stays saved until the
  person replaces or removes it. At start it is checked again, so a dead saved token is reported each start until dealt with.

## The Network section (both interfaces)

Per service, in order: **Status** (read-only: "No token (60 requests an hour)", "Saved", "From GITHUB_TOKEN", "4,987 of 5,000 left",
"GitHub refused it", "Could not check"), **Token** (a masked box; pasting sends nothing), **Save token**, **Check token**, **Create a
token on GitHub** (opens the service's page with a name filled in), **Remove the saved token**.

* **Save token** turns the box's text into a `Secret` and empties the box. The effect `SaveToken` prints as nothing; the Deck's
  `EndTextEntry` for a token box reports nothing as a setting's text; `SubmitToken` is how the Deck's Save row reaches the typed text.
* The host validates the text (not empty, no spaces or line breaks, printable ASCII, at most 512 characters; the message never repeats
  it), writes the file, uses the token, and **asks the service**:
  accepted → "GitHub token saved: 4,987 of 5,000 requests left, resets in 41 minutes" (plus the expiry date, and a warning if a classic
  token has permissions Reclaw does not need); **refused → not kept**, the previous state is restored and the person told; **cannot be
  asked** (offline, blocked, limited) → kept and said to be untested.
* **Check token** asks again and always answers. At start each saved token (and GitHub's anonymous allowance) is checked quietly: a
  notice appears only if the token is dead or has extra permissions.

## The check

GitHub: `GET /rate_limit` (costs nothing against the allowance; answers anonymously too): `resources.core` gives limit, remaining and
reset; the headers `x-oauth-scopes` (classic tokens) and `github-authentication-token-expiration` are read if present. GitLab:
`GET /api/v4/personal_access_tokens/self`: `active`, `revoked`, `expires_at` and the `ratelimit-*` headers.

## Speed

What makes GitHub content faster, in the order it matters: not asking (the platform index answers "which platforms" for hundreds of
projects with no API call); conditional requests (`ETag`; **a `304` costs nothing against the allowance: observed live**, 14,957 to 14,956
for one full answer and one `304`); one question for two askers (a per-address lock: the second reads what the first saved, tested with
eight threads and one request); a token (5,000 an hour, not 60; no hour-long wait after the limit); a wait that ends when the identity
changes. Not built: GraphQL batching (ADR 0014).

## Verified, and not

Verified live: the check's request and answer for GitHub (through the development sandbox's proxy, which substitutes its own
account: the allowance seen there is 15,000, not 5,000 or 60), the expiry header, `304` costing nothing. Verified with a local server: every
other rule above. **Not verified against the real services**: the `401` for a refused GitHub token (the proxy answers for its own
account whatever token is sent, so the live test reports itself inconclusive here), `X-OAuth-Scopes` on a real classic token, and
everything about GitLab tokens, including that `Authorization: Bearer` is accepted for a personal token (GitLab documents it; Quiver uses
`PRIVATE-TOKEN`). The anonymous 60-an-hour limit is from GitHub's documentation, not observed.
