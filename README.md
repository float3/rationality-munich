# rationality-munich.com

The pages behind [rationality-munich.com](https://rationality-munich.com): a hub
for the EA, ACX, LessWrong and AI safety groups in Munich.

Everything under `www/` is served as-is. Push to `master` and the server picks
the change up within five minutes — no NixOS rebuild.

| Path | Page |
| --- | --- |
| `www/index.html` | the hub: where each group announces its events |
| `www/privacy.html` | what the site and the mailing list store |
| `www/impressum.html` | legal notice |

Two parts of the site live elsewhere, because a server builds them:

- `/calendar` and `/calendar.ics` are generated hourly from the groups' event
  feeds by `rationality-calendar` in [float3/nixos](https://github.com/float3/nixos).
- `lists.rationality-munich.com` is a listmonk install for event invitations.

## Editing

Plain HTML, no build step. Each page carries its own CSS, and a dark mode via
`prefers-color-scheme`. Open the file in a browser to check a change.
