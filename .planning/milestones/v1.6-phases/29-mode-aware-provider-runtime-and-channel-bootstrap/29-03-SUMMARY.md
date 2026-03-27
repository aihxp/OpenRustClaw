# Summary 29-03: Validate Channel Bootstrap During Onboarding

## Completed

- Telegram, Discord, and Slack setup now refresh the shipped channel probe report immediately after configuration
- onboarding records the resulting channel bootstrap outcome into durable setup state
- unsupported WhatsApp bootstrap is now reported honestly as manual work instead of being silently treated as configured

## Result

Channel onboarding no longer treats a token write as the same thing as channel readiness. The operator can now see whether the selected channel actually passed its first probe.
