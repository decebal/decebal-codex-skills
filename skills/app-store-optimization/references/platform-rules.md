# Platform Metadata Rules

Last verified: 2026-08-28. Recheck official sources before final compliance
claim.

## Apple App Store

Official references:

- [App information](https://developer.apple.com/help/app-store-connect/reference/app-information/app-information)
- [Platform version information](https://developer.apple.com/help/app-store-connect/reference/app-information/platform-version-information)

| Field | Limit | Unit | Runtime behavior |
|---|---:|---|---|
| Name | 30 | characters | required by `aso-lint` |
| Subtitle | 30 | characters | optional input, checked when present |
| Promotional text | 170 | characters | optional input, checked when present |
| Description | 4,000 | characters | required by `aso-lint` |
| Keywords | 100 | UTF-8 bytes | required; each comma-separated keyword must exceed two characters |
| What's new | 4,000 | characters | optional input, checked when present |

Apple says app/company names should not be duplicated in keyword list and other
apps' or companies' names are not allowed. `aso-lint` flags direct duplicates
available in input; human review remains required.

## Google Play

Official reference:

- [Create and set up your app](https://support.google.com/googleplay/android-developer/answer/9859152)

| Field | Limit | Unit | Runtime behavior |
|---|---:|---|---|
| App name | 30 | characters | required by `aso-lint` |
| Short description | 80 | characters | required by `aso-lint` |
| Full description | 4,000 | characters | required by `aso-lint` |

Google warns repetitive or irrelevant keyword use can cause suspension. Runtime
checks limits only; relevance and repetition need evidence-led human review.
