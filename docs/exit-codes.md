# Exit codes

The response body goes to stdout as JSON. An error goes to stderr as one JSON object:

```json
{ "error": { "code": "NOT_FOUND", "messages": ["resource not found"], "status": 404 } }
```

The exit code comes from `error.code` first, then from the HTTP status. Do not parse stderr, except for this object.

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Generic error, network error, 5xx |
| 2 | Usage error (bad flag, bad JSON in `--data`, a missing `jq`, rate above the cap) |
| 3 | Authentication (`UNAUTHORIZED`, no key, 401) |
| 4 | Forbidden (`FORBIDDEN`, `FEATURE_NOT_ENABLED`, 403) |
| 5 | Validation (`VALIDATION_FAILED`, other 422) |
| 6 | Not found (`NOT_FOUND`, 404) |
| 7 | Rate limited (`RATE_LIMITED`, 429, `retry_at` in the error), or stopped by the circuit breaker |
| 8 | Confirmation required and not confirmed |
| 9 | Partial result: the data is not complete. The coverage is in the error |
| 10 | Write refused: `--env` missing on a write |

Exit 7 means stop. Wait, read `purchasely usage`, do not loop. Exit 9 means the data is not complete: read the coverage before you use it.
