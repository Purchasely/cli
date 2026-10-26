<!-- Generated from the command tree. Do not edit. Run: npx vitest run test/docs.test.ts -t "commands.md is generated" -u -->

# purchasely command reference

```text
Usage: purchasely [options] [command]

Purchasely command line tool

Options:
  -V, --version                  output the version number
  -h, --help                     display help for command

Commands:
  auth                           Manage the Management API key
  usage [options]                Requests of the last hour, rejections, limiter and breaker state
  purge [options]                Delete the cached client data
  search [options] <task>        Find a command for a task
  reference [options]            Print the full command reference of this version (Markdown, man,
                                 HTML or JSON)
  ai                             Install the Purchasely AI plugin in your coding agents
  update [options]               Check for a new version and install it (managed install) or print
                                 the upgrade command
  games [game]                   Play a game in the terminal
  completion <shell>             Print a shell completion script (bash, zsh or fish)
  api [options] <method> <path>  Send a raw Management API request
  dashboard [options]            Analytics dashboards. Start with: dashboard summary --app <app_id>
                                 --output table
  ab-tests                       Management API: ab-tests
  answers                        Management API: answers
  applications|apps              Management API: applications
  audiences                      Management API: audiences
  campaigns                      Management API: campaigns
  category-tags                  Management API: category-tags
  client-errors                  Management API: client-errors
  custom-event-properties        Management API: custom-event-properties
  custom-events                  Management API: custom-events
  custom-properties              Management API: custom-properties
  flows                          Management API: flows
  web-activations                Management API: web-activations
  fonts                          Management API: fonts
  placements                     Management API: placements
  plans                          Management API: plans
  distributions                  Management API: distributions
  platforms                      Management API: platforms
  products                       Management API: products
  one-time-purchases             Management API: one-time-purchases
  subscriptions                  Management API: subscriptions
  screens                        Management API: screens
  screen-versions                Management API: screen-versions
  surveys                        Management API: surveys
  translations                   Management API: translations
  users                          Management API: users
  web-flows                      Management API: web-flows
  webhook-deliveries             Management API: webhook-deliveries
  webhooks                       Management API: webhooks
  help [command]                 display help for command

Conventions:
  JSON goes to stdout. An error goes to stderr as one JSON object.
  A command is <resource> <action>. The resource is the lowercase tag of the spec (ab-tests, custom-events).
  The action is list, get, create, update, delete, or a verb such as start or make-default. apps is an alias of applications.
  --app <id> is the application id (--app-id is an alias). A command that exists with and without an application
  (placements list) calls the global endpoint when you omit --app.
  The other path parameters are flags: --id, --plan-id, --store.
  The query parameters are flags in kebab case. An array takes --fields id,name or a repeated flag.
  --data '<json>', --data @file.json or --data - (stdin) is the body.
  --dry-run prints { method, url, headers, body } and sends nothing. It needs no key.
  --env is mandatory on every write (POST, PUT, PATCH, DELETE), also with --dry-run. Exit 10 without it.
  A risky change returns CONFIRMATION_REQUIRED with its impact and a confirm_token (valid 5 minutes, tied to the exact request).
  A JSON output over 200 KB is not printed. It goes to a file in the cache directory and the CLI prints {"file": "...", "bytes": 123}.
  --out <file> chooses the file. --out - forces stdout.

Examples:
  purchasely placements list --app app_xxx --fields id,vendor_id | jq -r '.placements[].id'
  purchasely screens list --app app_xxx
  purchasely placements get --app app_xxx --id <placement>
  purchasely placements update --app app_xxx --id plc_xxx --data @placement.json --env prod --dry-run
  purchasely reference --format man
```

## ab-tests

Management API: ab-tests

### `purchasely ab-tests`

```text
Usage: purchasely ab-tests [options] [command]

Management API: ab-tests

Options:
  -h, --help                                 display help for command

Commands:
  list [options]                             List AB tests for an application (excludes
                                             campaign-owned draft AB tests)
  create [options]                           Create an AB test (draft) on an application
  get [options]                              Show an AB test
  update [options]                           Update an AB test
  delete [options]                           Permanently delete a DRAFT AB test
  ab-test-variants-detailed-stats [options]  Per-dimension conversion + revenue stats for the
                                             variants of one AB test
  start [options]                            Start a DRAFT AB test, wires it into its placements
  terminate [options]                        Terminate a RUNNING AB test, unwires it from its
                                             placements
  help [command]                             display help for command
```

### `purchasely ab-tests ab-test-variants-detailed-stats`

```text
Usage: purchasely ab-tests ab-test-variants-detailed-stats [options]

Per-dimension conversion + revenue stats for the variants of one AB test

Options:
  --output <format>             json (default) or table
  --env <env>                   prod or staging. Mandatory for writes
  --dry-run                     print the request without sending it
  --yes                         accept the confirmation prompt of a risky change
  --reason <text>               why you run this command (sent in the User-Agent)
  --rate <n>                    requests per second, lower than the default
  --i-asked-backend             allow a rate above the hard cap
  --out <file>                  write the JSON output to a file, or - for stdout
  --app <app_id>                application public id (required)
  --id <value>                  Public ID of the resource. [string, required]
  --ab-test-variant-id <value>  Public ID of the resource. [string, required]
  --dimension <value>           Group the variant's conversions and revenue by this dimension:
                                country = store country (ISO code); plan = subscription plan;
                                platform = store platform. Required: there is no ungrouped result.
                                For a variant total, add the counts and revenue of one dimension's
                                groups; unique_viewers_count is the variant's total repeated in
                                every group (read it once) and the per-user averages are ratios, not
                                sums. [enum: country|plan|platform, required]
  --fields <a,b>                Field selection for token economy: an explicit list of field names
                                to include in the response, or ["*"] for every field. Omit for this
                                endpoint's own default field set. [array of string] comma separated
                                or repeated
  -h, --help                    display help for command

Per-dimension conversion + revenue stats for the variants of one AB test. Without ab_test_variant_id (MCP only), returns every variant in one call under `variants`, in position order: prefer this to compare variants. With it, returns one variant.

Required scope: experimentation.ab_tests:view
Request:
  GET /management/v1/applications/{application_id}/ab_tests/{id}/ab_test_variants/{ab_test_variant_id}/detailed_stats

Example:
  purchasely ab-tests ab-test-variants-detailed-stats --app app_xxx --id <id> --ab-test-variant-id <ab-test-variant-id> --dimension <dimension>
```

### `purchasely ab-tests create`

```text
Usage: purchasely ab-tests create [options]

Create an AB test (draft) on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create an AB test (draft) on an application.

Required scope: experimentation.ab_tests:create
Request:
  POST /management/v1/applications/{application_id}/ab_tests

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  ab_test_variants (array of object, required) Variants: [{ vendor_id, weight, position, config, presentable_id }, ...]. At least 2, positions 0..n-1, and weights (1-99 each) summing to 100 are required even when saving a draft.
  audience_id (string) Audience public ID (aud_… or caud_… or rtaud_…) to target; omit to run against all users.
  description (string) Free-text description of the test.
  name (string, required) Display name shown in console.
  placement_ids (array of string) Placement public IDs (plac_…) this test runs on.
  test_type (string) USER_INTERFACE or PRICING. Defaults to USER_INTERFACE.
  vendor_id (string, required) Stable machine identifier used in SDK calls.

Example:
  purchasely ab-tests create --app app_xxx --data @body.json --env prod
```

### `purchasely ab-tests delete`

```text
Usage: purchasely ab-tests delete [options]

Permanently delete a DRAFT AB test

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Permanently delete a DRAFT AB test. REST-only, requires confirmation.

Required scope: experimentation.ab_tests:delete
Request:
  DELETE /management/v1/applications/{application_id}/ab_tests/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Confirmation flow: a risky change first returns the impact and a confirm_token (query).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely ab-tests delete --app app_xxx --id <id> --env prod
```

### `purchasely ab-tests get`

```text
Usage: purchasely ab-tests get [options]

Show an AB test

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show an AB test.

Required scope: experimentation.ab_tests:view
Request:
  GET /management/v1/applications/{application_id}/ab_tests/{id}

Example:
  purchasely ab-tests get --app app_xxx --id <id>
```

### `purchasely ab-tests list`

```text
Usage: purchasely ab-tests list [options]

List AB tests for an application (excludes campaign-owned draft AB tests)

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --status <value>    Filter by status: DRAFT, RUNNING, COMPLETED, or CANCELLED. [string]
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List AB tests for an application (excludes campaign-owned draft AB tests).

Required scope: experimentation.ab_tests:view
Request:
  GET /management/v1/applications/{application_id}/ab_tests  (with --app)
  GET /management/v1/ab_tests  (without --app)

Example:
  purchasely ab-tests list --app app_xxx
```

### `purchasely ab-tests start`

```text
Usage: purchasely ab-tests start [options]

Start a DRAFT AB test, wires it into its placements

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Start a DRAFT AB test, wires it into its placements. Requires >= 2 variants with positions 0..n-1 and weights summing to 100.

Required scope: experimentation.ab_tests:edit
Request:
  PUT /management/v1/applications/{application_id}/ab_tests/{id}/start

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely ab-tests start --app app_xxx --id <id> --env prod
```

### `purchasely ab-tests terminate`

```text
Usage: purchasely ab-tests terminate [options]

Terminate a RUNNING AB test, unwires it from its placements

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Terminate a RUNNING AB test, unwires it from its placements.

Required scope: experimentation.ab_tests:edit
Request:
  PUT /management/v1/applications/{application_id}/ab_tests/{id}/terminate

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely ab-tests terminate --app app_xxx --id <id> --env prod
```

### `purchasely ab-tests update`

```text
Usage: purchasely ab-tests update [options]

Update an AB test

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update an AB test. Most fields apply only while DRAFT (see individual field descriptions). category_tag_ids/placement_ids, when provided, fully replace the existing set (send the complete list, not a diff); omit either to leave the existing set untouched. ab_test_variants, when provided, is a full-resend of every variant.

Required scope: experimentation.ab_tests:edit
Request:
  PATCH /management/v1/applications/{application_id}/ab_tests/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  ab_test_variants (array of object) Variants to create/update/remove. DRAFT only. An entry with `id` (a variant's public_id) updates that variant; an existing variant whose id is NOT included is removed; an entry with no `id` creates a new variant.
  audience_id (string) Audience public ID (aud_… or caud_… or rtaud_…) to target. DRAFT only.
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate. Omit to leave existing category tags untouched.
  description (string) Free-text description of the test.
  name (string) Display name shown in console.
  placement_ids (array of string) Placement public IDs (plac_…) this test runs on. DRAFT only. Omit to leave existing placements untouched.
  test_type (string) USER_INTERFACE or PRICING. DRAFT only.
  vendor_id (string) Stable machine identifier used in SDK calls. DRAFT only.

Example:
  purchasely ab-tests update --app app_xxx --id <id> --data @body.json --env prod
```

## ai

Install the Purchasely AI plugin in your coding agents

### `purchasely ai`

```text
Usage: purchasely ai [options] [command]

Install the Purchasely AI plugin in your coding agents

Options:
  -h, --help           display help for command

Commands:
  install [options]    Install the plugin in every coding agent found on this machine
  update [options]     Update the plugin in the agents that ai install registered
  status [options]     Show the agents, the plugin version and how each one follows releases
  uninstall [options]  Remove the plugin from the agents that ai install registered
  help [command]       display help for command
```

### `purchasely ai install`

```text
Usage: purchasely ai install [options]

Install the plugin in every coding agent found on this machine

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --agent <name>     claude, codex, copilot, gemini, cursor, opencode or all (default: all)
  -h, --help         display help for command

Agents: claude, codex, copilot, gemini, cursor, opencode (default all). With --dry-run it runs read-only probes and prints the plan. PURCHASELY_OFFLINE=1 blocks it.
```

### `purchasely ai status`

```text
Usage: purchasely ai status [options]

Show the agents, the plugin version and how each one follows releases

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --agent <name>     claude, codex, copilot, gemini, cursor, opencode or all (default: all)
  -h, --help         display help for command

Per agent: detected, managed by ai install, installed by hand (true, false or unknown), plugin version, how it follows releases, last error. It changes nothing.
```

### `purchasely ai uninstall`

```text
Usage: purchasely ai uninstall [options]

Remove the plugin from the agents that ai install registered

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --agent <name>     claude, codex, copilot, gemini, cursor, opencode or all (default: all)
  -h, --help         display help for command

A marketplace is never removed.
```

### `purchasely ai update`

```text
Usage: purchasely ai update [options]

Update the plugin in the agents that ai install registered

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --agent <name>     claude, codex, copilot, gemini, cursor, opencode or all (default: all)
  -h, --help         display help for command

A copy agent (Cursor, OpenCode) updates only when a newer release exists.
```

## answers

Management API: answers

### `purchasely answers`

```text
Usage: purchasely answers [options] [command]

Management API: answers

Options:
  -h, --help        display help for command

Commands:
  list [options]    List answer options for a survey
  create [options]  Add a new answer option to an existing survey
  help [command]    display help for command
```

### `purchasely answers create`

```text
Usage: purchasely answers create [options]

Add a new answer option to an existing survey

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Add a new answer option to an existing survey.

Required scope: setup.surveys:edit
Request:
  POST /management/v1/applications/{application_id}/answers

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  survey_id (string, required) Survey public_id (survey_...) this answer belongs to.
  vendor_id (string, required) Stable machine identifier for this answer option (unique within the survey).

Example:
  purchasely answers create --app app_xxx --data @body.json --env prod
```

### `purchasely answers list`

```text
Usage: purchasely answers list [options]

List answer options for a survey

Options:
  --output <format>    json (default) or table
  --env <env>          prod or staging. Mandatory for writes
  --dry-run            print the request without sending it
  --yes                accept the confirmation prompt of a risky change
  --reason <text>      why you run this command (sent in the User-Agent)
  --rate <n>           requests per second, lower than the default
  --i-asked-backend    allow a rate above the hard cap
  --out <file>         write the JSON output to a file, or - for stdout
  --app <app_id>       application public id (optional: omit for all apps)
  --survey-id <value>  Survey public_id (survey_...) to list answers for. [string, required]
  --fields <a,b>       Field selection for token economy: an explicit list of field names to include
                       in the response, or ["*"] for every field. Omit for this endpoint's own
                       default field set. [array of string] comma separated or repeated
  --page <value>       Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>   Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help           display help for command

List answer options for a survey.

Required scope: setup.surveys:view
Request:
  GET /management/v1/applications/{application_id}/answers  (with --app)
  GET /management/v1/answers  (without --app)

Example:
  purchasely answers list --app app_xxx --survey-id <survey-id>
```

## api

Send a raw Management API request

### `purchasely api`

```text
Usage: purchasely api [options] <method> <path>

Send a raw Management API request

Arguments:
  method                   GET, POST, PUT, PATCH, DELETE
  path                     path under /management/v1, such as /applications/app_xxx/placements

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --query <k=v>            query parameter (repeatable) (default: [])
  --data <json>            request body: JSON, @file.json or - for stdin
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

The path is relative to /management/v1, or starts with it. The CLI sends it only to the Management API of --env.
A write (POST, PUT, PATCH, DELETE) needs --env prod or --env staging, also with --dry-run.
A risky change returns CONFIRMATION_REQUIRED: add --yes, answer the prompt, or resend with --confirm-token <token>.

Examples:
  purchasely api GET /applications/app_xxx/placements --query limit=10
  purchasely api PATCH /applications/app_xxx/placements/plc_xxx --data @placement.json --env prod --dry-run
```

## applications

Management API: applications

### `purchasely applications`

```text
Usage: purchasely applications|apps [options] [command]

Management API: applications

Options:
  -h, --help                       display help for command

Commands:
  list [options]                   List your applications
  get [options]                    Show an application
  update [options]                 Update an application's settings and/or platform credentials
  colors-palette [options]         Show an application's colors palette
  update-colors-palette [options]  Replace an application's colors palette
  help [command]                   display help for command
```

### `purchasely applications colors-palette`

```text
Usage: purchasely applications colors-palette [options]

Show an application's colors palette

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show an application's colors palette.

Required scope: setup.apps:view
Request:
  GET /management/v1/applications/{application_id}/colors_palette

Example:
  purchasely applications colors-palette --app app_xxx
```

### `purchasely applications get`

```text
Usage: purchasely applications get [options]

Show an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show an application.

Required scope: setup.apps:view
Request:
  GET /management/v1/applications/{application_id}

Example:
  purchasely applications get --app app_xxx
```

### `purchasely applications list`

```text
Usage: purchasely applications list [options]

List your applications

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List your applications.

Required scope: setup.apps:view
Request:
  GET /management/v1/applications

Example:
  purchasely applications list
```

### `purchasely applications update`

```text
Usage: purchasely applications update [options]

Update an application's settings and/or platform credentials

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update an application's settings and/or platform credentials.

Required scope: setup.apps:edit
Request:
  PATCH /management/v1/applications/{application_id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  client_webhook_url (string) URL Purchasely calls for client webhook notifications.
  default_dashboard_currency (string) Default currency for console dashboards (ISO 4217, e.g. EUR).
  default_integration_currency (string) Default currency used for SDK/API integration values (ISO 4217, e.g. EUR).
  default_screen_id (string) Public ID of the screen (pres_…) to use as this app's default.
  icon_url (string) URL of the app's icon.
  name (string) Display name shown in console.

Example:
  purchasely applications update --app app_xxx --data @body.json --env prod
```

### `purchasely applications update-colors-palette`

```text
Usage: purchasely applications update-colors-palette [options]

Replace an application's colors palette

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Replace an application's colors palette.

Required scope: setup.apps:edit
Request:
  PATCH /management/v1/applications/{application_id}/update_colors_palette

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  colors_palette (object, required) Full colors palette object; replaces the app's current one entirely.

Example:
  purchasely applications update-colors-palette --app app_xxx --data @body.json --env prod
```

## audiences

Management API: audiences

### `purchasely audiences`

```text
Usage: purchasely audiences [options] [command]

Management API: audiences

Options:
  -h, --help         display help for command

Commands:
  list [options]     List audiences for an application
  create [options]   Create a targeting audience on an application
  update [options]   Update an audience's name, vendor_id, targeting definition, and/or category
                     tags
  discard [options]  Discard (soft-delete) an audience
  help [command]     display help for command
```

### `purchasely audiences create`

```text
Usage: purchasely audiences create [options]

Create a targeting audience on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a targeting audience on an application.

Required scope: targeting.audiences:create
Request:
  POST /management/v1/applications/{application_id}/audiences

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate.
  definition (object, required) Targeting rule definition ({operator, criterias}).
  name (string, required) Display name shown in console.
  vendor_id (string) Stable machine identifier used in SDK calls.

Example:
  purchasely audiences create --app app_xxx --data @body.json --env prod
```

### `purchasely audiences discard`

```text
Usage: purchasely audiences discard [options]

Discard (soft-delete) an audience

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --data <json>            request body: JSON, @file.json or - for stdin
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Discard (soft-delete) an audience. Requires confirm_token on the second call if the audience is still targeted by live placements.

Required scope: targeting.audiences:delete
Request:
  POST /management/v1/applications/{application_id}/audiences/{id}/discard

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  confirm_token (string) Confirmation token from a prior call that returned CONFIRMATION_REQUIRED. Resend with identical params to proceed.
  only_if_unused (boolean) Safe cleanup: refuse any incoming references and never remove associations. Defaults to false.

Confirmation flow: a risky change first returns the impact and a confirm_token (body).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely audiences discard --app app_xxx --id <id> --data @body.json --env prod
```

### `purchasely audiences list`

```text
Usage: purchasely audiences list [options]

List audiences for an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List audiences for an application.

Required scope: targeting.audiences:view
Request:
  GET /management/v1/applications/{application_id}/audiences  (with --app)
  GET /management/v1/audiences  (without --app)

Example:
  purchasely audiences list --app app_xxx
```

### `purchasely audiences update`

```text
Usage: purchasely audiences update [options]

Update an audience's name, vendor_id, targeting definition, and/or category tags

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update an audience's name, vendor_id, targeting definition, and/or category tags.

Required scope: targeting.audiences:edit
Request:
  PATCH /management/v1/applications/{application_id}/audiences/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate. Replaces the existing set entirely.
  definition (object) Targeting rule definition ({operator, criterias}). Replaces the existing definition entirely.
  name (string) Display name shown in console.
  vendor_id (string) Stable machine identifier used in SDK calls.

Example:
  purchasely audiences update --app app_xxx --id <id> --data @body.json --env prod
```

## auth

Manage the Management API key

### `purchasely auth`

```text
Usage: purchasely auth [options] [command]

Manage the Management API key

Options:
  -h, --help        display help for command

Commands:
  login [options]   Store the Management API key
  status [options]  Show where the credential comes from
  logout [options]  Delete the stored credential
  help [command]    display help for command
```

### `purchasely auth login`

```text
Usage: purchasely auth login [options]

Store the Management API key

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --api-key <key>    the key (visible in the shell history: prefer the prompt or stdin)
  -h, --help         display help for command

The key comes from the hidden prompt, from --api-key or from stdin. It is not checked against the API: no read endpoint works for every Management scope.
```

### `purchasely auth logout`

```text
Usage: purchasely auth logout [options]

Delete the stored credential

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  -h, --help         display help for command

Deletes the key stored in the keychain or in the file. A key in the environment is not touched.
```

### `purchasely auth status`

```text
Usage: purchasely auth status [options]

Show where the credential comes from

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  -h, --help         display help for command

Prints the source of the credential (env, keychain or file), its last 4 characters and the identity. Exit 3 when there is no key.
```

## campaigns

Management API: campaigns

### `purchasely campaigns`

```text
Usage: purchasely campaigns [options] [command]

Management API: campaigns

Options:
  -h, --help             display help for command

Commands:
  list [options]         List campaigns for an application
  create [options]       Create a campaign on an application
  index-stats [options]  Conversion stats for campaigns matching the given filters
  positions [options]    Bulk-reorder running campaigns by public_id
  get [options]          Show a campaign
  update [options]       Update a campaign
  delete [options]       Discard (soft-delete) a campaign
  help [command]         display help for command
```

### `purchasely campaigns create`

```text
Usage: purchasely campaigns create [options]

Create a campaign on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a campaign on an application.

Required scope: targeting.campaigns:create
Request:
  POST /management/v1/applications/{application_id}/campaigns

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate. Omit to leave existing category tags untouched.
  config (object) Campaign configuration. Drafts may be incomplete. Launching requires what, who and when; when must include frequency_cap, impression_cap and exposure_window. Reference persisted IDs, not vendor IDs.
  ends_at (string) ISO8601 end datetime.
  name (string, required) Display name shown in console.
  placement_ids (array of string) Placement public IDs (plac_…) this campaign targets. For Management/MCP drafts, saves the selection for the console editor; delivery starts only when running. Pass IDs returned by placements_create/list. Omit to leave existing placements untouched; [] clears them.
  starts_at (string) ISO8601 start datetime. Required by the model when status is running.
  status (enum: draft|running, required) Create a draft or running campaign. A deactivated campaign cannot be created.
  vendor_id (string, required) Stable machine identifier used in SDK calls.

Example:
  purchasely campaigns create --app app_xxx --data @body.json --env prod
```

### `purchasely campaigns delete`

```text
Usage: purchasely campaigns delete [options]

Discard (soft-delete) a campaign

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Discard (soft-delete) a campaign. REST-only, requires confirmation.

Required scope: targeting.campaigns:delete
Request:
  DELETE /management/v1/applications/{application_id}/campaigns/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Confirmation flow: a risky change first returns the impact and a confirm_token (query).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely campaigns delete --app app_xxx --id <id> --env prod
```

### `purchasely campaigns get`

```text
Usage: purchasely campaigns get [options]

Show a campaign

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a campaign.

Required scope: targeting.campaigns:view
Request:
  GET /management/v1/applications/{application_id}/campaigns/{id}

Example:
  purchasely campaigns get --app app_xxx --id <id>
```

### `purchasely campaigns index-stats`

```text
Usage: purchasely campaigns index-stats [options]

Conversion stats for campaigns matching the given filters

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (required)
  --status <value>          Filter by status: draft, running, or deactivated. [string]
  --search <value>          Filter by campaign name or vendor_id (case-insensitive substring match).
                            [string]
  --category-tag-ids <a,b>  Filter by category tag public IDs. [array of string] comma separated or
                            repeated
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  -h, --help                display help for command

Conversion stats for campaigns matching the given filters.

Required scope: targeting.campaigns:view
Request:
  GET /management/v1/applications/{application_id}/campaigns/index_stats

Example:
  purchasely campaigns index-stats --app app_xxx
```

### `purchasely campaigns list`

```text
Usage: purchasely campaigns list [options]

List campaigns for an application

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (optional: omit for all apps)
  --status <value>          Filter by status: draft, running, or deactivated. [string]
  --search <value>          Filter by campaign name or vendor_id (case-insensitive substring match).
                            [string]
  --category-tag-ids <a,b>  Filter by category tag public IDs. [array of string] comma separated or
                            repeated
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  --page <value>            Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>        Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help                display help for command

List campaigns for an application.

Required scope: targeting.campaigns:view
Request:
  GET /management/v1/applications/{application_id}/campaigns  (with --app)
  GET /management/v1/campaigns  (without --app)

Example:
  purchasely campaigns list --app app_xxx
```

### `purchasely campaigns positions`

```text
Usage: purchasely campaigns positions [options]

Bulk-reorder running campaigns by public_id

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Bulk-reorder running campaigns by public_id.

Required scope: targeting.campaigns:edit
Request:
  PUT /management/v1/applications/{application_id}/campaigns/positions

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  campaigns (array of object, required) Ordered list of { id: <campaign public_id>, position: <integer> }. Only RUNNING campaigns owned by this app are reordered; other entries are silently ignored.

Example:
  purchasely campaigns positions --app app_xxx --data @body.json --env prod
```

### `purchasely campaigns update`

```text
Usage: purchasely campaigns update [options]

Update a campaign

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a campaign. Resends the full campaign object, category_tag_ids/placement_ids, if provided, fully replace the existing set; omitted config/tags/placements are preserved. Running campaigns cannot change vendor_id, content or audience, and need complete timing. Deactivation must be requested separately from other field changes; running campaigns cannot return to draft.

Required scope: targeting.campaigns:edit
Request:
  PATCH /management/v1/applications/{application_id}/campaigns/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate. Omit to leave existing category tags untouched.
  config (object) Campaign configuration. Drafts may be incomplete. Launching requires what, who and when; when must include frequency_cap, impression_cap and exposure_window. Reference persisted IDs, not vendor IDs.
  ends_at (string) ISO8601 end datetime.
  name (string, required) Display name shown in console.
  placement_ids (array of string) Placement public IDs (plac_…) this campaign targets. For Management/MCP drafts, saves the selection for the console editor; delivery starts only when running. Pass IDs returned by placements_create/list. Omit to leave existing placements untouched; [] clears them.
  starts_at (string) ISO8601 start datetime. Required by the model when status is running.
  status (enum: draft|running|deactivated, required) draft, running, or deactivated.
  vendor_id (string, required) Stable machine identifier used in SDK calls.

Example:
  purchasely campaigns update --app app_xxx --id <id> --data @body.json --env prod
```

## category-tags

Management API: category-tags

### `purchasely category-tags`

```text
Usage: purchasely category-tags [options] [command]

Management API: category-tags

Options:
  -h, --help        display help for command

Commands:
  list [options]    List category tags for an application
  create [options]  Create a category tag on an application
  get [options]     Show a single category tag for an application
  delete [options]  Permanently delete a category tag from an application
  help [command]    display help for command
```

### `purchasely category-tags create`

```text
Usage: purchasely category-tags create [options]

Create a category tag on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a category tag on an application.

Required scope: experiences.category_tags:create
Request:
  POST /management/v1/applications/{application_id}/category_tags

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  name (string, required) Display name of the category tag.
  vendor_id (string, required) Stable machine identifier (unique within the app).

Example:
  purchasely category-tags create --app app_xxx --data @body.json --env prod
```

### `purchasely category-tags delete`

```text
Usage: purchasely category-tags delete [options]

Permanently delete a category tag from an application

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Permanently delete a category tag from an application. REST + client MCP only, requires confirmation.

Required scope: experiences.category_tags:delete
Request:
  DELETE /management/v1/applications/{application_id}/category_tags/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Confirmation flow: a risky change first returns the impact and a confirm_token (query).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely category-tags delete --app app_xxx --id <id> --env prod
```

### `purchasely category-tags get`

```text
Usage: purchasely category-tags get [options]

Show a single category tag for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a single category tag for an application.

Required scope: experiences.category_tags:view
Request:
  GET /management/v1/applications/{application_id}/category_tags/{id}

Example:
  purchasely category-tags get --app app_xxx --id <id>
```

### `purchasely category-tags list`

```text
Usage: purchasely category-tags list [options]

List category tags for an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --search <value>    Case-insensitive search on the category tag's name. [string]
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List category tags for an application.

Required scope: experiences.category_tags:view
Request:
  GET /management/v1/applications/{application_id}/category_tags  (with --app)
  GET /management/v1/category_tags  (without --app)

Example:
  purchasely category-tags list --app app_xxx
```

## client-errors

Management API: client-errors

### `purchasely client-errors`

```text
Usage: purchasely client-errors [options] [command]

Management API: client-errors

Options:
  -h, --help                   display help for command

Commands:
  list [options]               List active, actionable client integration errors for an application
                               (or across every application you can access)
  update-collection [options]  Bulk-update client errors (mark read/unread, or resolved)
  help [command]               display help for command
```

### `purchasely client-errors list`

```text
Usage: purchasely client-errors list [options]

List active, actionable client integration errors for an application (or across every application
you can access)

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List active, actionable client integration errors for an application (or across every application you can access).

Required scope: lookups.client_errors:view
Request:
  GET /management/v1/applications/{application_id}/client_errors  (with --app)
  GET /management/v1/client_errors  (without --app)

Example:
  purchasely client-errors list --app app_xxx
```

### `purchasely client-errors update-collection`

```text
Usage: purchasely client-errors update-collection [options]

Bulk-update client errors (mark read/unread, or resolved)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Bulk-update client errors (mark read/unread, or resolved). Scoped to the applications you can access.

Required scope: lookups.client_errors:edit
Request:
  PUT /management/v1/applications/{application_id}/client_errors/update_collection

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  client_errors (array of object, required) Array of {id, read, status}. id is required; read (boolean) and status ('resolved') are optional.

Example:
  purchasely client-errors update-collection --app app_xxx --data @body.json --env prod
```

## completion

Print a shell completion script (bash, zsh or fish)

### `purchasely completion`

```text
Usage: purchasely completion [options] <shell>

Print a shell completion script (bash, zsh or fish)

Arguments:
  shell       the shell of the script (choices: "bash", "zsh", "fish")

Options:
  -h, --help  display help for command

It prints the script and installs nothing. Put one line in the startup file of the shell:
  bash  source <(purchasely completion bash)     in ~/.bashrc
  zsh   source <(purchasely completion zsh)      in ~/.zshrc, after compinit
  fish  purchasely completion fish | source      in ~/.config/fish/config.fish
The script holds the commands, the subcommands and the long options of this version. It makes no request.
```

## custom-event-properties

Management API: custom-event-properties

### `purchasely custom-event-properties`

```text
Usage: purchasely custom-event-properties [options] [command]

Management API: custom-event-properties

Options:
  -h, --help        display help for command

Commands:
  list [options]    List the custom event properties of an application
  create [options]  Declare a custom event property on an application
  get [options]     Show a single custom event property of an application
  update [options]  Update a custom event property (partial, omitted fields are left untouched)
  delete [options]  Permanently delete a custom event property from an application (and so from
                    every custom event)
  help [command]    display help for command
```

### `purchasely custom-event-properties create`

```text
Usage: purchasely custom-event-properties create [options]

Declare a custom event property on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Declare a custom event property on an application. It is available to every custom event of the app. data_type must be one of: int, bool, float, string, date, array_of_strings.

Required scope: setup.custom_events:create
Request:
  POST /management/v1/applications/{application_id}/custom_event_properties

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  data_type (string, required) One of: int, bool, float, string, date, array_of_strings.
  name (string, required) Property name, as the SDK sends it (unique within the app).
  position (integer) Display order within the application (default 0).

Example:
  purchasely custom-event-properties create --app app_xxx --data @body.json --env prod
```

### `purchasely custom-event-properties delete`

```text
Usage: purchasely custom-event-properties delete [options]

Permanently delete a custom event property from an application (and so from every custom event)

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Permanently delete a custom event property from an application (and so from every custom event). Requires confirmation.

Required scope: setup.custom_events:edit
Request:
  DELETE /management/v1/applications/{application_id}/custom_event_properties/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Confirmation flow: a risky change first returns the impact and a confirm_token (query).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely custom-event-properties delete --app app_xxx --id <id> --env prod
```

### `purchasely custom-event-properties get`

```text
Usage: purchasely custom-event-properties get [options]

Show a single custom event property of an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a single custom event property of an application.

Required scope: setup.custom_events:view
Request:
  GET /management/v1/applications/{application_id}/custom_event_properties/{id}

Example:
  purchasely custom-event-properties get --app app_xxx --id <id>
```

### `purchasely custom-event-properties list`

```text
Usage: purchasely custom-event-properties list [options]

List the custom event properties of an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --search <value>    Case-insensitive search on the property's name. [string]
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List the custom event properties of an application. Every property is available to every custom event of the app.

Required scope: setup.custom_events:view
Request:
  GET /management/v1/applications/{application_id}/custom_event_properties  (with --app)
  GET /management/v1/custom_event_properties  (without --app)

Example:
  purchasely custom-event-properties list --app app_xxx
```

### `purchasely custom-event-properties update`

```text
Usage: purchasely custom-event-properties update [options]

Update a custom event property (partial, omitted fields are left untouched)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a custom event property (partial, omitted fields are left untouched). data_type must be one of: int, bool, float, string, date, array_of_strings.

Required scope: setup.custom_events:edit
Request:
  PATCH /management/v1/applications/{application_id}/custom_event_properties/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  data_type (string) One of: int, bool, float, string, date, array_of_strings.
  name (string) Property name, as the SDK sends it (unique within the app).
  position (integer) Display order within the application.

Example:
  purchasely custom-event-properties update --app app_xxx --id <id> --data @body.json --env prod
```

## custom-events

Management API: custom-events

### `purchasely custom-events`

```text
Usage: purchasely custom-events [options] [command]

Management API: custom-events

Options:
  -h, --help        display help for command

Commands:
  list [options]    List custom events (event-schema definitions) for an application
  create [options]  Create a custom event (event-schema definition) on an application
  get [options]     Show a single custom event for an application
  update [options]  Update a custom event (partial, omitted fields are left untouched, except
                    category_tag_ids, which fully replaces the existing set when sent)
  help [command]    display help for command
```

### `purchasely custom-events create`

```text
Usage: purchasely custom-events create [options]

Create a custom event (event-schema definition) on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a custom event (event-schema definition) on an application.

Required scope: setup.custom_events:create
Request:
  POST /management/v1/applications/{application_id}/custom_events

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  active (boolean) Whether the event is active.
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate with this event.
  color (string) Display color (hex) shown in console.
  custom_event_properties (array of object) Properties this event carries, fully replaces the existing set. Each entry is either { id } (an existing property public_id of the application) or { name, data_type, position? } (links the application's property with that name, or declares it; data_type is required and must match). A property is shared by every event of the application: it is renamed, retyped or deleted through the custom event properties endpoints. data_type must be one of: int, bool, float, string, date, array_of_strings.
  name (string, required) Display name shown in console.
  tracked (boolean) Whether the event is tracked.
  vendor_id (string, required) Stable machine identifier used in SDK calls (unique within the app).

Example:
  purchasely custom-events create --app app_xxx --data @body.json --env prod
```

### `purchasely custom-events get`

```text
Usage: purchasely custom-events get [options]

Show a single custom event for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a single custom event for an application.

Required scope: setup.custom_events:view
Request:
  GET /management/v1/applications/{application_id}/custom_events/{id}

Example:
  purchasely custom-events get --app app_xxx --id <id>
```

### `purchasely custom-events list`

```text
Usage: purchasely custom-events list [options]

List custom events (event-schema definitions) for an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --search <value>    Case-insensitive search on the custom event's name. [string]
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List custom events (event-schema definitions) for an application.

Required scope: setup.custom_events:view
Request:
  GET /management/v1/applications/{application_id}/custom_events  (with --app)
  GET /management/v1/custom_events  (without --app)

Example:
  purchasely custom-events list --app app_xxx
```

### `purchasely custom-events update`

```text
Usage: purchasely custom-events update [options]

Update a custom event (partial, omitted fields are left untouched, except category_tag_ids, which
fully replaces the existing set when sent)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a custom event (partial, omitted fields are left untouched, except category_tag_ids, which fully replaces the existing set when sent).

Required scope: setup.custom_events:edit
Request:
  PATCH /management/v1/applications/{application_id}/custom_events/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  active (boolean) Whether the event is active.
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate with this event, fully replaces the existing set.
  color (string) Display color (hex) shown in console.
  custom_event_properties (array of object) Properties this event carries, fully replaces the existing set. Each entry is either { id } (an existing property public_id of the application) or { name, data_type, position? } (links the application's property with that name, or declares it; data_type is required and must match). A property is shared by every event of the application: it is renamed, retyped or deleted through the custom event properties endpoints. data_type must be one of: int, bool, float, string, date, array_of_strings.
  name (string) Display name shown in console.
  tracked (boolean) Whether the event is tracked.
  vendor_id (string) Stable machine identifier used in SDK calls (unique within the app).

Example:
  purchasely custom-events update --app app_xxx --id <id> --data @body.json --env prod
```

## custom-properties

Management API: custom-properties

### `purchasely custom-properties`

```text
Usage: purchasely custom-properties [options] [command]

Management API: custom-properties

Options:
  -h, --help        display help for command

Commands:
  list [options]    List custom properties (user-attribute schema definitions) for an application,
                    including global builtin properties available to every app
  create [options]  Create a custom property (user-attribute schema definition) on an application
  get [options]     Show a single custom property for an application, or a global builtin property
  update [options]  Update a custom property (partial, omitted fields are left untouched)
  help [command]    display help for command
```

### `purchasely custom-properties create`

```text
Usage: purchasely custom-properties create [options]

Create a custom property (user-attribute schema definition) on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a custom property (user-attribute schema definition) on an application. data_type must be one of: bool, date, float, int, array_string, string, version.

Required scope: setup.user_attributes:create
Request:
  POST /management/v1/applications/{application_id}/custom_properties

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  category (string) Use Insight Attributes for survey answer storage. Other values are ignored, matching the console.
  data_type (string, required) One of: bool, date, float, int, array_string, string, version.
  key (string, required) SDK key used to set this property's value. Alphanumeric, must not start with a digit.
  name (string, required) Display name shown in console.

Example:
  purchasely custom-properties create --app app_xxx --data @body.json --env prod
```

### `purchasely custom-properties get`

```text
Usage: purchasely custom-properties get [options]

Show a single custom property for an application, or a global builtin property

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a single custom property for an application, or a global builtin property.

Required scope: setup.user_attributes:view
Request:
  GET /management/v1/applications/{application_id}/custom_properties/{id}

Example:
  purchasely custom-properties get --app app_xxx --id <id>
```

### `purchasely custom-properties list`

```text
Usage: purchasely custom-properties list [options]

List custom properties (user-attribute schema definitions) for an application, including global
builtin properties available to every app

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List custom properties (user-attribute schema definitions) for an application, including global builtin properties available to every app.

Required scope: setup.user_attributes:view
Request:
  GET /management/v1/applications/{application_id}/custom_properties  (with --app)
  GET /management/v1/custom_properties  (without --app)

Example:
  purchasely custom-properties list --app app_xxx
```

### `purchasely custom-properties update`

```text
Usage: purchasely custom-properties update [options]

Update a custom property (partial, omitted fields are left untouched)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a custom property (partial, omitted fields are left untouched). data_type must be one of: bool, date, float, int, array_string, string, version. Builtin properties cannot be updated.

Required scope: setup.user_attributes:edit
Request:
  PATCH /management/v1/applications/{application_id}/custom_properties/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  data_type (string) One of: bool, date, float, int, array_string, string, version.
  key (string) SDK key used to set this property's value. Alphanumeric, must not start with a digit.
  name (string) Display name shown in console.

Example:
  purchasely custom-properties update --app app_xxx --id <id> --data @body.json --env prod
```

## dashboard

Analytics dashboards. Start with: dashboard summary --app <app_id> --output table

### `purchasely dashboard`

```text
Usage: purchasely dashboard [options] [command]

Analytics dashboards. Start with: dashboard summary --app <app_id> --output table

Options:
  --output <format>                      json (default) or table
  --env <env>                            prod or staging. Mandatory for writes
  --dry-run                              print the request without sending it
  --yes                                  accept the confirmation prompt of a risky change
  --reason <text>                        why you run this command (sent in the User-Agent)
  --rate <n>                             requests per second, lower than the default
  --i-asked-backend                      allow a rate above the hard cap
  --out <file>                           write the JSON output to a file, or - for stdout
  --app <app_id>                         application public id. Without it, a menu shows the apps of
                                         the key
  -h, --help                             display help for command

Commands:
  summary [options]                      Analytics dashboard of an app: MRR, subscriptions, viewers,
                                         conversions with trend charts
  composition-live [options]             Subscription composition (by platform/plan/product/state)
                                         for an application, or across every application you can
                                         access
  events [options]                       Event counts per day for an application, or across every
                                         application you can access
  flow-paths [options]                   Flow-paths funnel for a single application
  funnel [options]                       Conversion funnel (views -> purchases) for an application,
                                         or across every application you can access
  header-live [options]                  Live header cards (active-subscriptions statuses, live
                                         users, inactive-subscriptions causes) for an application,
                                         or across every application you can access
  header-today [options]                 Today's header card for an application, or across every
                                         application you can access
  mrr-evolution-live [options]           Live MRR evolution (by day and by month) for an
                                         application, or across every application you can access
  one-time-purchases-revenues [options]  Daily one-time-purchase revenue by country/plan, for an
                                         application or across every application you can access
  one-time-purchases-stats [options]     Live one-time-purchase stats (by store/plan, refunds) for
                                         an application, or across every application you can access
  subscriptions-revenues [options]       Daily subscription revenue by country/plan, for an
                                         application or across every application you can access
  subscriptions-stats [options]          Live subscriptions revenue/churn/refund stats for an
                                         application, or across every application you can access
  trials-live [options]                  Trial/intro-offer cohorts and balances for an application,
                                         or across every application you can access
  conversion [options]                   Paywall conversion: paywall views and the purchases they
                                         led to, from 2024-01-01 onwards
  mau-reports [options]                  Monthly active users: app starts and distinct users who
                                         started the app, per calendar month, over the rolling 12
                                         months ending today (always monthly; takes no date_range,
                                         periodicity or filter)
  paid-subscription-movements [options]  Paid subscription movements: for each period, how many
                                         PAYING subscriptions (full price or paid offer) entered the
                                         paid base (total_activations_count: direct purchases,
                                         trial/intro/promo conversions, reactivations) and how many
                                         left it (total_churns_count: voluntary churn,
                                         involuntary/billing-failure churn, refunds), plus
                                         net_movement_count = total_activations_count -
                                         total_churns_count
  paid-subscription-retention [options]  Paid-subscription retention cohort matrix for a single
                                         application
  paywall-latency [options]              Paywall-serving latency percentiles (p50/p95/p99) for an
                                         application, or aggregated across every application you can
                                         access
  sdk-evolution [options]                SDK-version adoption series over time for a single
                                         application
  sdk-versions [options]                 Current SDK-version breakdown for a single application
  subscription-activations [options]     Subscription activations: how many subscriptions started in
                                         each period
  subscription-evolution [options]       Active subscription base: how many subscriptions were
                                         active in each period, i.e
  subscription-reactivations [options]   Subscription reactivations: how many subscriptions came
                                         back to life in each period after having lapsed,
                                         re-subscribed after expiry/cancellation, recovered from a
                                         billing retry, or unpaused (one count per subscription per
                                         day; family-shared subscriptions excluded)
  help [command]                         display help for command

Without a subcommand, an interactive terminal shows a menu of the dashboards, then runs the chosen one for the last 30 days.
The menu leaves out the dashboards that need another flag (funnel, flow-paths): run them by name.
With --output json, or without a terminal, it prints the list of all the dashboards as JSON, with their required flags, and sends no request.

Example:
  purchasely dashboard --output json | jq -r '.[].command'
```

### `purchasely dashboard composition-live`

```text
Usage: purchasely dashboard composition-live [options]

Subscription composition (by platform/plan/product/state) for an application, or across every
application you can access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Subscription composition (by platform/plan/product/state) for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/composition_live  (with --app)
  GET /management/v1/dashboard/composition_live  (without --app)

Example:
  purchasely dashboard composition-live --app app_xxx --date-range <date-range>
```

### `purchasely dashboard conversion`

```text
Usage: purchasely dashboard conversion [options]

Paywall conversion: paywall views and the purchases they led to, from 2024-01-01 onwards

Options:
  --output <format>                    json (default) or table
  --env <env>                          prod or staging. Mandatory for writes
  --dry-run                            print the request without sending it
  --yes                                accept the confirmation prompt of a risky change
  --reason <text>                      why you run this command (sent in the User-Agent)
  --rate <n>                           requests per second, lower than the default
  --i-asked-backend                    allow a rate above the hard cap
  --out <file>                         write the JSON output to a file, or - for stdout
  --app <app_id>                       application public id (required)
  --date-range <a,b>                   Two-element [start_date, end_date] window of ISO8601 dates
                                       (YYYY-MM-DD); both ends inclusive (the end date's events are
                                       counted). Every time-series tool in this namespace needs it.
                                       [array of string, required] comma separated or repeated
  --periodicity <value>                Bucket granularity. Defaults to MONTH. Use DAY for ranges up
                                       to ~60 days, WEEK up to ~6 months, MONTH beyond. Each row's
                                       `period` is the bucket's start date. [enum:
                                       DAY|WEEK|MONTH|YEAR]
  --countries <a,b>                    Filter by store country ISO codes. [array of string] comma
                                       separated or repeated
  --platform-names <a,b>               Filter by platform name. [array of enum:
                                       AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or
                                       repeated
  --audiences <a,b>                    Filter by audience public_id. [array of string] comma
                                       separated or repeated
  --campaigns <a,b>                    Filter by campaign public_id. [array of string] comma
                                       separated or repeated
  --placements <a,b>                   Filter by placement public_id. [array of string] comma
                                       separated or repeated
  --screens <a,b>                      Filter by screen (paywall) public_id. [array of string] comma
                                       separated or repeated
  --ab-test-variants <a,b>             Filter by A/B test variant public_id. [array of string] comma
                                       separated or repeated
  --offer-types <a,b>                  Filter by offer type. [array of enum:
                                       NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE]
                                       comma separated or repeated
  --unique-users <value>               Restrict to unique users ("true"/"false"). [string]
  --survey <value>                     Filter by survey identifier. [string]
  --primary-metric-group-by <value>    Split the paywall views / unique viewers (primary metric) by
                                       one dimension. Omit for a single, ungrouped series. Values
                                       beyond the 10 largest groups are collapsed into one "Other"
                                       row. platform = store platform (ios, android, …); screen =
                                       paywall screen the subscription was purchased from; placement
                                       = placement the paywall was shown at; audience = audience the
                                       user matched; ab_test_variant = A/B test variant; country =
                                       store country (ISO code). [enum:
                                       platform|screen|placement|audience|ab_test_variant|country]
  --secondary-metric-group-by <value>  Split the purchases / conversions (secondary metric) by one
                                       dimension. Omit for a single, ungrouped series. Values beyond
                                       the 10 largest groups are collapsed into one "Other" row.
                                       platform = store platform (ios, android, …); screen = paywall
                                       screen the subscription was purchased from; placement =
                                       placement the paywall was shown at; audience = audience the
                                       user matched; plan = subscription plan; ab_test_variant = A/B
                                       test variant; country = store country (ISO code). [enum:
                                       platform|screen|placement|audience|plan|ab_test_variant|country]
  --fields <a,b>                       Field selection for token economy: an explicit list of field
                                       names to include in the response, or ["*"] for every field.
                                       Omit for this endpoint's own default field set. [array of
                                       string] comma separated or repeated
  -h, --help                           display help for command

Paywall conversion: paywall views and the purchases they led to, from 2024-01-01 onwards. Returns one object with five keys: primary_metric_by_period, [{period, group_by, group_by_public_id, group_by_name, presentation_views_count, unique_viewers_count}] (views and distinct viewers per period, split by primary_metric_group_by); secondary_metric_by_period, [{period, group_by…, purchase_tapped_count, conversions_to_offer_count, conversions_to_regular_count, total_conversions_count, conversions_to_offer_ratio, conversions_to_regular_ratio, total_conversions_ratio, click_through_rate_purchase_tapped_ratio}] (counts split by secondary_metric_group_by; each ratio divides the group's count by the unique viewers of the whole period, not of the group); primary_metric_table and secondary_metric_table, the same counts totalled over the whole range per dimension value, with *_by_presentation_views_ratio and *_by_unique_viewers_ratio; kpi, [{presentation_views_count, unique_viewers_count, paywalls_viewed_per_user_average, conversions_to_offer_ratio, conversions_to_regular_ratio, total_conversions_ratio}] for the whole range. conversions_to_offer = free-trial/intro/promotional-offer starts; conversions_to_regular = those offers converting to paid plus direct full-price purchases; total_conversions = offer starts plus direct purchases. Units: counts, and ratios of a count to a viewer count (a viewer can start several subscriptions, so a ratio can exceed 1; not a percentage). date_range is mandatory here. Recommended periodicity: WEEK or MONTH. Single application only.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/conversion

Example:
  purchasely dashboard conversion --app app_xxx --date-range <date-range>
```

### `purchasely dashboard events`

```text
Usage: purchasely dashboard events [options]

Event counts per day for an application, or across every application you can access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --periodicity <value>       Bucket granularity: DAY, WEEK, MONTH or YEAR (default MONTH). [enum:
                              DAY|WEEK|MONTH|YEAR]
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Event counts per day for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/events  (with --app)
  GET /management/v1/dashboard/events  (without --app)

Example:
  purchasely dashboard events --app app_xxx --date-range <date-range>
```

### `purchasely dashboard flow-paths`

```text
Usage: purchasely dashboard flow-paths [options]

Flow-paths funnel for a single application

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (required)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --flow-id <value>           Restrict to a single Flow, by public_id. [string, required]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Flow-paths funnel for a single application.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/flow_paths

Example:
  purchasely dashboard flow-paths --app app_xxx --date-range <date-range> --flow-id <flow-id>
```

### `purchasely dashboard funnel`

```text
Usage: purchasely dashboard funnel [options]

Conversion funnel (views -> purchases) for an application, or across every application you can
access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --dimension <value>         Group funnel steps by this dimension: country = store country (ISO
                              code); placement = placement the paywall was shown at; plan =
                              subscription plan; platform = store platform (ios, android, …); screen
                              = paywall screen. One result group per dimension value. [enum:
                              country|placement|plan|platform|screen, required]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Conversion funnel (views -> purchases) for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/funnel  (with --app)
  GET /management/v1/dashboard/funnel  (without --app)

Example:
  purchasely dashboard funnel --app app_xxx --date-range <date-range> --dimension <dimension>
```

### `purchasely dashboard header-live`

```text
Usage: purchasely dashboard header-live [options]

Live header cards (active-subscriptions statuses, live users, inactive-subscriptions causes) for an
application, or across every application you can access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --category <value>          Restrict to one header card (perf optimization); omit to receive all
                              cards. active_subscriptions_statuses = active subscriptions split by
                              status; active_subscribers = live users (the most expensive card);
                              inactive_subscriptions_causes = inactive subscriptions split by cause.
                              [enum:
                              active_subscriptions_statuses|active_subscribers|inactive_subscriptions_causes]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Live header cards (active-subscriptions statuses, live users, inactive-subscriptions causes) for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/header_live  (with --app)
  GET /management/v1/dashboard/header_live  (without --app)

Example:
  purchasely dashboard header-live --app app_xxx --date-range <date-range>
```

### `purchasely dashboard header-today`

```text
Usage: purchasely dashboard header-today [options]

Today's header card for an application, or across every application you can access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Today's header card for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/header_today  (with --app)
  GET /management/v1/dashboard/header_today  (without --app)

Example:
  purchasely dashboard header-today --app app_xxx --date-range <date-range>
```

### `purchasely dashboard mau-reports`

```text
Usage: purchasely dashboard mau-reports [options]

Monthly active users: app starts and distinct users who started the app, per calendar month, over
the rolling 12 months ending today (always monthly; takes no date_range, periodicity or filter)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (optional: omit for all apps)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Monthly active users: app starts and distinct users who started the app, per calendar month, over the rolling 12 months ending today (always monthly; takes no date_range, periodicity or filter). For one application returns {data: {app_starts: [{period: "YYYY-MM", value}], unique_users: [{period, value}]}}; without application_id returns {data: {applications: [{mobile_application_id, mobile_application_name, client_name, chart_data: {app_starts, unique_users}}]}} with one entry per application you can access (never summed across apps). Unit: count of app starts / of unique users per month (unique_users is a per-month distinct count, do not sum it across months).

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/mau_reports  (with --app)
  GET /management/v1/dashboard_v2/mau_reports  (without --app)

Example:
  purchasely dashboard mau-reports --app app_xxx
```

### `purchasely dashboard mrr-evolution-live`

```text
Usage: purchasely dashboard mrr-evolution-live [options]

Live MRR evolution (by day and by month) for an application, or across every application you can
access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --periodicity <value>       Bucket granularity: DAY, WEEK, MONTH or YEAR (default MONTH). [enum:
                              DAY|WEEK|MONTH|YEAR]
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Live MRR evolution (by day and by month) for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/mrr_evolution_live  (with --app)
  GET /management/v1/dashboard/mrr_evolution_live  (without --app)

Example:
  purchasely dashboard mrr-evolution-live --app app_xxx --date-range <date-range>
```

### `purchasely dashboard one-time-purchases-revenues`

```text
Usage: purchasely dashboard one-time-purchases-revenues [options]

Daily one-time-purchase revenue by country/plan, for an application or across every application you
can access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --periodicity <value>       Bucket granularity: DAY, WEEK, MONTH or YEAR (default MONTH). [enum:
                              DAY|WEEK|MONTH|YEAR]
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Daily one-time-purchase revenue by country/plan, for an application or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/one_time_purchases_revenues  (with --app)
  GET /management/v1/dashboard/one_time_purchases_revenues  (without --app)

Example:
  purchasely dashboard one-time-purchases-revenues --app app_xxx --date-range <date-range>
```

### `purchasely dashboard one-time-purchases-stats`

```text
Usage: purchasely dashboard one-time-purchases-stats [options]

Live one-time-purchase stats (by store/plan, refunds) for an application, or across every
application you can access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --periodicity <value>       Bucket granularity: DAY, WEEK, MONTH or YEAR (default MONTH). [enum:
                              DAY|WEEK|MONTH|YEAR]
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --product-types <a,b>       Filter by product type (CONSUMABLE or NON_CONSUMABLE). [array of enum:
                              CONSUMABLE|NON_CONSUMABLE|NON_RENEWING_SUBSCRIPTION|RENEWING_SUBSCRIPTION]
                              comma separated or repeated
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Live one-time-purchase stats (by store/plan, refunds) for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/one_time_purchases_stats  (with --app)
  GET /management/v1/dashboard/one_time_purchases_stats  (without --app)

Example:
  purchasely dashboard one-time-purchases-stats --app app_xxx --date-range <date-range>
```

### `purchasely dashboard paid-subscription-movements`

```text
Usage: purchasely dashboard paid-subscription-movements [options]

Paid subscription movements: for each period, how many PAYING subscriptions (full price or paid
offer) entered the paid base (total_activations_count: direct purchases, trial/intro/promo
conversions, reactivations) and how many left it (total_churns_count: voluntary churn,
involuntary/billing-failure churn, refunds), plus net_movement_count = total_activations_count -
total_churns_count

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (required)
  --date-range <a,b>        Two-element [start_date, end_date] window of ISO8601 dates (YYYY-MM-DD);
                            both ends inclusive (the end date's events are counted). Every
                            time-series tool in this namespace needs it. [array of string, required]
                            comma separated or repeated
  --periodicity <value>     Bucket granularity. Defaults to MONTH. Use DAY for ranges up to ~60
                            days, WEEK up to ~6 months, MONTH beyond. Each row's `period` is the
                            bucket's start date. [enum: DAY|WEEK|MONTH|YEAR]
  --countries <a,b>         Filter by store country ISO codes. [array of string] comma separated or
                            repeated
  --platform-names <a,b>    Filter by platform name. [array of enum:
                            AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --audiences <a,b>         Filter by audience public_id. [array of string] comma separated or
                            repeated
  --campaigns <a,b>         Filter by campaign public_id. [array of string] comma separated or
                            repeated
  --placements <a,b>        Filter by placement public_id. [array of string] comma separated or
                            repeated
  --screens <a,b>           Filter by screen (paywall) public_id. [array of string] comma separated
                            or repeated
  --ab-test-variants <a,b>  Filter by A/B test variant public_id. [array of string] comma separated
                            or repeated
  --offer-types <a,b>       Filter by offer type. [array of enum:
                            NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                            separated or repeated
  --unique-users <value>    Restrict to unique users ("true"/"false"). [string]
  --survey <value>          Filter by survey identifier. [string]
  --group-by <value>        Split the movements by one dimension. Omit for a single, ungrouped
                            series. Values beyond the 10 largest groups are collapsed into one
                            "Other" row. platform = store platform (ios, android, …); screen =
                            paywall screen the subscription was purchased from; placement =
                            placement the paywall was shown at; audience = audience the user
                            matched; ab_test_variant = A/B test variant; plan = subscription plan;
                            country = store country (ISO code); offer_types = offer type (NONE,
                            FREE_TRIAL, INTRO_OFFER, PROMO_CODE, PROMOTIONAL_OFFER); payment_types =
                            payment type (FULL_PRICE, PAID_OFFER, FREE_OFFER); period_types = plan
                            billing period (P1W, P1M, P1Y, …); campaign = campaign; event_types =
                            movement type, entries: ACTIVATION, TRIAL_CONVERTED,
                            INTRO_OFFER_STARTED/CONVERTED, PROMO_CODE_STARTED/CONVERTED,
                            PROMOTIONAL_OFFER_STARTED/CONVERTED, REACTIVATION; exits:
                            VOLUNTARY_CHURN, INVOLUNTARY_CHURN, REFUND (each row then carries only
                            the count that applies). [enum:
                            platform|screen|placement|audience|ab_test_variant|plan|country|offer_types|payment_types|period_types|campaign|event_types]
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  -h, --help                display help for command

Paid subscription movements: for each period, how many PAYING subscriptions (full price or paid offer) entered the paid base (total_activations_count: direct purchases, trial/intro/promo conversions, reactivations) and how many left it (total_churns_count: voluntary churn, involuntary/billing-failure churn, refunds), plus net_movement_count = total_activations_count - total_churns_count. Always restricted to the paid segment (no `segment` parameter), this is the paid counterpart of subscription_activations. Returns an array of {period, group_by, group_by_public_id, group_by_name, total_activations_count, total_churns_count, net_movement_count} rows; unit: count of subscriptions (a flow, summable across periods). Recommended periodicity: MONTH. Single application only.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/paid_subscription_movements

Example:
  purchasely dashboard paid-subscription-movements --app app_xxx --date-range <date-range>
```

### `purchasely dashboard paid-subscription-retention`

```text
Usage: purchasely dashboard paid-subscription-retention [options]

Paid-subscription retention cohort matrix for a single application

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (required)
  --date-range <a,b>        Two-element [start_date, end_date] window of ISO8601 dates (YYYY-MM-DD);
                            both ends inclusive (the end date's events are counted). Every
                            time-series tool in this namespace needs it. [array of string, required]
                            comma separated or repeated
  --periodicity <value>     Bucket granularity. Defaults to MONTH. Use DAY for ranges up to ~60
                            days, WEEK up to ~6 months, MONTH beyond. Each row's `period` is the
                            bucket's start date. [enum: DAY|WEEK|MONTH|YEAR]
  --countries <a,b>         Filter by store country ISO codes. [array of string] comma separated or
                            repeated
  --platform-names <a,b>    Filter by platform name. [array of enum:
                            AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --audiences <a,b>         Filter by audience public_id. [array of string] comma separated or
                            repeated
  --campaigns <a,b>         Filter by campaign public_id. [array of string] comma separated or
                            repeated
  --placements <a,b>        Filter by placement public_id. [array of string] comma separated or
                            repeated
  --screens <a,b>           Filter by screen (paywall) public_id. [array of string] comma separated
                            or repeated
  --ab-test-variants <a,b>  Filter by A/B test variant public_id. [array of string] comma separated
                            or repeated
  --offer-types <a,b>       Filter by offer type. [array of enum:
                            NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                            separated or repeated
  --unique-users <value>    Restrict to unique users ("true"/"false"). [string]
  --survey <value>          Filter by survey identifier. [string]
  --segment <value>         Restrict to a revenue segment. all = every subscription (default); paid
                            = paying, at full price or on a paid (discounted) offer; full_price =
                            paying the regular price; discounted = paying a reduced
                            introductory/promotional price; free = on a free trial or free offer.
                            [enum: all|paid|full_price|discounted|free]
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  -h, --help                display help for command

Paid-subscription retention cohort matrix for a single application.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/paid_subscription_retention

Example:
  purchasely dashboard paid-subscription-retention --app app_xxx --date-range <date-range>
```

### `purchasely dashboard paywall-latency`

```text
Usage: purchasely dashboard paywall-latency [options]

Paywall-serving latency percentiles (p50/p95/p99) for an application, or aggregated across every
application you can access

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (optional: omit for all apps)
  --period <value>   Lookback window in days, clamped to [7, 90] (default 30). [string]
  --countries <a,b>  Filter by store country ISO codes. [array of string] comma separated or
                     repeated
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Paywall-serving latency percentiles (p50/p95/p99) for an application, or aggregated across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/paywall_latency  (with --app)
  GET /management/v1/dashboard_v2/paywall_latency  (without --app)

Example:
  purchasely dashboard paywall-latency --app app_xxx
```

### `purchasely dashboard sdk-evolution`

```text
Usage: purchasely dashboard sdk-evolution [options]

SDK-version adoption series over time for a single application

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (required)
  --date-range <a,b>        Two-element [start_date, end_date] window of ISO8601 dates (YYYY-MM-DD);
                            both ends inclusive (the end date's events are counted). Every
                            time-series tool in this namespace needs it. [array of string, required]
                            comma separated or repeated
  --periodicity <value>     Bucket granularity. Defaults to MONTH. Use DAY for ranges up to ~60
                            days, WEEK up to ~6 months, MONTH beyond. Each row's `period` is the
                            bucket's start date. [enum: DAY|WEEK|MONTH|YEAR]
  --countries <a,b>         Filter by store country ISO codes. [array of string] comma separated or
                            repeated
  --platform-names <a,b>    Filter by platform name. [array of enum:
                            AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --audiences <a,b>         Filter by audience public_id. [array of string] comma separated or
                            repeated
  --campaigns <a,b>         Filter by campaign public_id. [array of string] comma separated or
                            repeated
  --placements <a,b>        Filter by placement public_id. [array of string] comma separated or
                            repeated
  --screens <a,b>           Filter by screen (paywall) public_id. [array of string] comma separated
                            or repeated
  --ab-test-variants <a,b>  Filter by A/B test variant public_id. [array of string] comma separated
                            or repeated
  --offer-types <a,b>       Filter by offer type. [array of enum:
                            NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                            separated or repeated
  --unique-users <value>    Restrict to unique users ("true"/"false"). [string]
  --survey <value>          Filter by survey identifier. [string]
  --group-by <value>        Split the series by one dimension. Omit for a single, ungrouped series.
                            Values beyond the 10 largest groups are collapsed into one "Other" row.
                            sdk_version = SDK version; sdk_running_mode = SDK running mode. [enum:
                            sdk_version|sdk_running_mode]
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  -h, --help                display help for command

SDK-version adoption series over time for a single application.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/sdk_evolution

Example:
  purchasely dashboard sdk-evolution --app app_xxx --date-range <date-range>
```

### `purchasely dashboard sdk-versions`

```text
Usage: purchasely dashboard sdk-versions [options]

Current SDK-version breakdown for a single application

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (required)
  --date-range <a,b>        Two-element [start_date, end_date] window of ISO8601 dates (YYYY-MM-DD);
                            both ends inclusive (the end date's events are counted). Every
                            time-series tool in this namespace needs it. [array of string, required]
                            comma separated or repeated
  --countries <a,b>         Filter by store country ISO codes. [array of string] comma separated or
                            repeated
  --platform-names <a,b>    Filter by platform name. [array of enum:
                            AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --audiences <a,b>         Filter by audience public_id. [array of string] comma separated or
                            repeated
  --campaigns <a,b>         Filter by campaign public_id. [array of string] comma separated or
                            repeated
  --placements <a,b>        Filter by placement public_id. [array of string] comma separated or
                            repeated
  --screens <a,b>           Filter by screen (paywall) public_id. [array of string] comma separated
                            or repeated
  --ab-test-variants <a,b>  Filter by A/B test variant public_id. [array of string] comma separated
                            or repeated
  --offer-types <a,b>       Filter by offer type. [array of enum:
                            NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                            separated or repeated
  --unique-users <value>    Restrict to unique users ("true"/"false"). [string]
  --survey <value>          Filter by survey identifier. [string]
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  -h, --help                display help for command

Current SDK-version breakdown for a single application.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/sdk_versions

Example:
  purchasely dashboard sdk-versions --app app_xxx --date-range <date-range>
```

### `purchasely dashboard subscription-activations`

```text
Usage: purchasely dashboard subscription-activations [options]

Subscription activations: how many subscriptions started in each period

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (required)
  --date-range <a,b>        Two-element [start_date, end_date] window of ISO8601 dates (YYYY-MM-DD);
                            both ends inclusive (the end date's events are counted). Every
                            time-series tool in this namespace needs it. [array of string, required]
                            comma separated or repeated
  --periodicity <value>     Bucket granularity. Defaults to MONTH. Use DAY for ranges up to ~60
                            days, WEEK up to ~6 months, MONTH beyond. Each row's `period` is the
                            bucket's start date. [enum: DAY|WEEK|MONTH|YEAR]
  --countries <a,b>         Filter by store country ISO codes. [array of string] comma separated or
                            repeated
  --platform-names <a,b>    Filter by platform name. [array of enum:
                            AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --audiences <a,b>         Filter by audience public_id. [array of string] comma separated or
                            repeated
  --campaigns <a,b>         Filter by campaign public_id. [array of string] comma separated or
                            repeated
  --placements <a,b>        Filter by placement public_id. [array of string] comma separated or
                            repeated
  --screens <a,b>           Filter by screen (paywall) public_id. [array of string] comma separated
                            or repeated
  --ab-test-variants <a,b>  Filter by A/B test variant public_id. [array of string] comma separated
                            or repeated
  --offer-types <a,b>       Filter by offer type. [array of enum:
                            NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                            separated or repeated
  --unique-users <value>    Restrict to unique users ("true"/"false"). [string]
  --survey <value>          Filter by survey identifier. [string]
  --group-by <value>        Split the series by one dimension. Omit for a single, ungrouped series.
                            Values beyond the 10 largest groups are collapsed into one "Other" row.
                            platform = store platform (ios, android, …); screen = paywall screen the
                            subscription was purchased from; placement = placement the paywall was
                            shown at; audience = audience the user matched; ab_test_variant = A/B
                            test variant; plan = subscription plan; country = store country (ISO
                            code); offer_types = offer type (NONE, FREE_TRIAL, INTRO_OFFER,
                            PROMO_CODE, PROMOTIONAL_OFFER); payment_types = payment type
                            (FULL_PRICE, PAID_OFFER, FREE_OFFER); period_types = plan billing period
                            (P1W, P1M, P1Y, …); campaign = campaign; event_types = activation event
                            (SUBSCRIPTION_STARTED, TRIAL_STARTED, TRIAL_CONVERTED,
                            INTRO_OFFER_STARTED/CONVERTED, PROMO_CODE_STARTED/CONVERTED,
                            PROMOTIONAL_OFFER_STARTED/CONVERTED). [enum:
                            platform|screen|placement|audience|ab_test_variant|plan|country|offer_types|payment_types|period_types|campaign|event_types]
  --segment <value>         Restrict to a revenue segment. all = every subscription (default); paid
                            = paying, at full price or on a paid (discounted) offer; full_price =
                            paying the regular price; discounted = paying a reduced
                            introductory/promotional price; free = on a free trial or free offer.
                            [enum: all|paid|full_price|discounted|free]
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  -h, --help                display help for command

Subscription activations: how many subscriptions started in each period. Each subscription is counted once, on its first activation event inside the chosen segment (direct purchase, free-trial/intro/promo start, or trial/intro/promo conversion to paid, for segment=paid, only the first PAYING activation counts); reactivations, transfers and family-shared subscriptions are excluded. Returns an array of {period, group_by, group_by_public_id, group_by_name, total_activations_count} rows, one per period (and per group_by value when set); unit: count of subscriptions (a flow, summable across periods). Recommended periodicity: MONTH, WEEK for ranges under ~6 months. Single application only.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/subscription_activations

Example:
  purchasely dashboard subscription-activations --app app_xxx --date-range <date-range>
```

### `purchasely dashboard subscription-evolution`

```text
Usage: purchasely dashboard subscription-evolution [options]

Active subscription base: how many subscriptions were active in each period, i.e

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (required)
  --date-range <a,b>        Two-element [start_date, end_date] window of ISO8601 dates (YYYY-MM-DD);
                            both ends inclusive (the end date's events are counted). Every
                            time-series tool in this namespace needs it. [array of string, required]
                            comma separated or repeated
  --periodicity <value>     Bucket granularity. Defaults to MONTH. Use DAY for ranges up to ~60
                            days, WEEK up to ~6 months, MONTH beyond. Each row's `period` is the
                            bucket's start date. [enum: DAY|WEEK|MONTH|YEAR]
  --countries <a,b>         Filter by store country ISO codes. [array of string] comma separated or
                            repeated
  --platform-names <a,b>    Filter by platform name. [array of enum:
                            AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --audiences <a,b>         Filter by audience public_id. [array of string] comma separated or
                            repeated
  --campaigns <a,b>         Filter by campaign public_id. [array of string] comma separated or
                            repeated
  --placements <a,b>        Filter by placement public_id. [array of string] comma separated or
                            repeated
  --screens <a,b>           Filter by screen (paywall) public_id. [array of string] comma separated
                            or repeated
  --ab-test-variants <a,b>  Filter by A/B test variant public_id. [array of string] comma separated
                            or repeated
  --offer-types <a,b>       Filter by offer type. [array of enum:
                            NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                            separated or repeated
  --unique-users <value>    Restrict to unique users ("true"/"false"). [string]
  --survey <value>          Filter by survey identifier. [string]
  --group-by <value>        Split the active base by one dimension. Omit for a single, ungrouped
                            series. Values beyond the 10 largest groups are collapsed into one
                            "Other" row. platform = store platform (ios, android, …); screen =
                            paywall screen the subscription was purchased from; placement =
                            placement the paywall was shown at; audience = audience the user
                            matched; ab_test_variant = A/B test variant; plan = subscription plan;
                            country = store country (ISO code); offer_types = offer type (NONE,
                            FREE_TRIAL, INTRO_OFFER, PROMO_CODE, PROMOTIONAL_OFFER); payment_types =
                            payment type (FULL_PRICE, PAID_OFFER, FREE_OFFER); period_types = plan
                            billing period (P1W, P1M, P1Y, …); subscription_status = subscription
                            status (AUTO_RENEWING, AUTO_RENEWING_CANCELED, IN_GRACE_PERIOD);
                            campaign = campaign. [enum:
                            platform|screen|placement|audience|ab_test_variant|plan|country|offer_types|payment_types|period_types|subscription_status|campaign]
  --segment <value>         Restrict to a revenue segment. all = every subscription (default); paid
                            = paying, at full price or on a paid (discounted) offer; full_price =
                            paying the regular price; discounted = paying a reduced
                            introductory/promotional price; free = on a free trial or free offer.
                            [enum: all|paid|full_price|discounted|free]
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  -h, --help                display help for command

Active subscription base: how many subscriptions were active in each period, i.e. activated in or before that period and not deactivated before it (auto-renewing, cancelled-but-not-expired and grace-period subscriptions all count as active; family-shared subscriptions and transfers are excluded). Returns an array of {period, group_by, group_by_public_id, group_by_name, count} rows, one per period (and per group_by value when set); unit: count of active subscriptions, a stock, not a flow: never sum it across periods, read the last period for the current base. Recommended periodicity: MONTH (WEEK for a close look at the last few months). Single application only.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/subscription_evolution

Example:
  purchasely dashboard subscription-evolution --app app_xxx --date-range <date-range>
```

### `purchasely dashboard subscription-reactivations`

```text
Usage: purchasely dashboard subscription-reactivations [options]

Subscription reactivations: how many subscriptions came back to life in each period after having
lapsed, re-subscribed after expiry/cancellation, recovered from a billing retry, or unpaused (one
count per subscription per day; family-shared subscriptions excluded)

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (required)
  --date-range <a,b>        Two-element [start_date, end_date] window of ISO8601 dates (YYYY-MM-DD);
                            both ends inclusive (the end date's events are counted). Every
                            time-series tool in this namespace needs it. [array of string, required]
                            comma separated or repeated
  --periodicity <value>     Bucket granularity. Defaults to MONTH. Use DAY for ranges up to ~60
                            days, WEEK up to ~6 months, MONTH beyond. Each row's `period` is the
                            bucket's start date. [enum: DAY|WEEK|MONTH|YEAR]
  --countries <a,b>         Filter by store country ISO codes. [array of string] comma separated or
                            repeated
  --platform-names <a,b>    Filter by platform name. [array of enum:
                            AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --audiences <a,b>         Filter by audience public_id. [array of string] comma separated or
                            repeated
  --campaigns <a,b>         Filter by campaign public_id. [array of string] comma separated or
                            repeated
  --placements <a,b>        Filter by placement public_id. [array of string] comma separated or
                            repeated
  --screens <a,b>           Filter by screen (paywall) public_id. [array of string] comma separated
                            or repeated
  --ab-test-variants <a,b>  Filter by A/B test variant public_id. [array of string] comma separated
                            or repeated
  --offer-types <a,b>       Filter by offer type. [array of enum:
                            NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                            separated or repeated
  --unique-users <value>    Restrict to unique users ("true"/"false"). [string]
  --survey <value>          Filter by survey identifier. [string]
  --group-by <value>        Split the series by one dimension. Omit for a single, ungrouped series.
                            Values beyond the 10 largest groups are collapsed into one "Other" row.
                            platform = store platform (ios, android, …); screen = paywall screen the
                            subscription was purchased from; placement = placement the paywall was
                            shown at; audience = audience the user matched; ab_test_variant = A/B
                            test variant; plan = subscription plan; country = store country (ISO
                            code); offer_types = offer type (NONE, FREE_TRIAL, INTRO_OFFER,
                            PROMO_CODE, PROMOTIONAL_OFFER); payment_types = payment type
                            (FULL_PRICE, PAID_OFFER, FREE_OFFER); period_types = plan billing period
                            (P1W, P1M, P1Y, …); campaign = campaign; event_types = reactivation
                            cause (SUBSCRIPTION_REACTIVATED,
                            SUBSCRIPTION_RECOVERED_FROM_BILLING_RETRY, SUBSCRIPTION_UNPAUSED), or
                            the offer conversion it revealed (TRIAL_CONVERTED,
                            INTRO_OFFER_CONVERTED, PROMO_CODE_CONVERTED,
                            PROMOTIONAL_OFFER_CONVERTED). [enum:
                            platform|screen|placement|audience|ab_test_variant|plan|country|offer_types|payment_types|period_types|campaign|event_types]
  --segment <value>         Restrict to a revenue segment. all = every subscription (default); paid
                            = paying, at full price or on a paid (discounted) offer; full_price =
                            paying the regular price; discounted = paying a reduced
                            introductory/promotional price; free = on a free trial or free offer.
                            [enum: all|paid|full_price|discounted|free]
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  -h, --help                display help for command

Subscription reactivations: how many subscriptions came back to life in each period after having lapsed, re-subscribed after expiry/cancellation, recovered from a billing retry, or unpaused (one count per subscription per day; family-shared subscriptions excluded). Returns an array of {period, group_by, group_by_public_id, group_by_name, total_reactivations_count} rows, one per period (and per group_by value when set); unit: count of subscriptions (a flow, summable across periods). Recommended periodicity: MONTH. Single application only.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard_v2/subscription_reactivations

Example:
  purchasely dashboard subscription-reactivations --app app_xxx --date-range <date-range>
```

### `purchasely dashboard subscriptions-revenues`

```text
Usage: purchasely dashboard subscriptions-revenues [options]

Daily subscription revenue by country/plan, for an application or across every application you can
access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --periodicity <value>       Bucket granularity: DAY, WEEK, MONTH or YEAR (default MONTH). [enum:
                              DAY|WEEK|MONTH|YEAR]
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Daily subscription revenue by country/plan, for an application or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/subscriptions_revenues  (with --app)
  GET /management/v1/dashboard/subscriptions_revenues  (without --app)

Example:
  purchasely dashboard subscriptions-revenues --app app_xxx --date-range <date-range>
```

### `purchasely dashboard subscriptions-stats`

```text
Usage: purchasely dashboard subscriptions-stats [options]

Live subscriptions revenue/churn/refund stats for an application, or across every application you
can access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --periodicity <value>       Bucket granularity: DAY, WEEK or MONTH (default MONTH). YEAR is not
                              supported by this tool. [enum: DAY|WEEK|MONTH]
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Live subscriptions revenue/churn/refund stats for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/subscriptions_stats  (with --app)
  GET /management/v1/dashboard/subscriptions_stats  (without --app)

Example:
  purchasely dashboard subscriptions-stats --app app_xxx --date-range <date-range>
```

### `purchasely dashboard summary`

```text
Usage: purchasely dashboard summary [options]

Analytics dashboard of an app: MRR, subscriptions, viewers, conversions with trend charts

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --days <n>         number of days up to today, from 2 to 730 (default: "30")
  -h, --help         display help for command

Shows the MRR, the active subscriptions, the paywall unique viewers and the conversions of one app.
The default output is JSON. --output table draws one line per metric with a sparkline.
MRR is today's value, in the dashboard currency of the app. Active subscriptions is the current base.
Viewers and conversions are the total of the range.
It sends 3 requests (4 when --days is above 60).

Example:
  purchasely dashboard summary --app app_xxx --output table
  purchasely dashboard summary --app app_xxx --days 90 --output table
```

### `purchasely dashboard trials-live`

```text
Usage: purchasely dashboard trials-live [options]

Trial/intro-offer cohorts and balances for an application, or across every application you can
access

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (optional: omit for all apps)
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --periodicity <value>       Bucket granularity: DAY, WEEK, MONTH or YEAR (default MONTH). [enum:
                              DAY|WEEK|MONTH|YEAR]
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --survey <value>            Filter by survey identifier. [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Trial/intro-offer cohorts and balances for an application, or across every application you can access.

Required scope: analytics.dashboards:view
Request:
  GET /management/v1/applications/{application_id}/dashboard/trials_live  (with --app)
  GET /management/v1/dashboard/trials_live  (without --app)

Example:
  purchasely dashboard trials-live --app app_xxx --date-range <date-range>
```

## distributions

Management API: distributions

### `purchasely distributions`

```text
Usage: purchasely distributions [options] [command]

Management API: distributions

Options:
  -h, --help        display help for command

Commands:
  list [options]    List every store distribution (SKU) configured for a plan
  create [options]  Add a store distribution (the store SKU / store_product_id) to a plan
  get [options]     Show a plan's distribution for one store
                    (app_store/play_store/amazon/huawei/stripe)
  update [options]  Update a store's distribution on a plan (its store_product_id and, for
                    play_store, the Google Play base plan / SwG settings)
  delete [options]  Remove a store's distribution from a plan
  help [command]    display help for command
```

### `purchasely distributions create`

```text
Usage: purchasely distributions create [options]

Add a store distribution (the store SKU / store_product_id) to a plan

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --plan-id <value>  Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Add a store distribution (the store SKU / store_product_id) to a plan. The app must already have that store's platform configured. For stripe, pass store_product_ids (an array of Stripe price IDs). store_product_id is frozen once a purchase has been made.

Required scope: setup.plans:edit
Request:
  POST /management/v1/applications/{application_id}/plans/{plan_id}/distributions

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  google_base_plan_backwards_compatible (boolean) play_store subscriptions only, true keeps Google Play Billing v4 backwards compatibility, false is v5-only. REQUIRED for a Play Store renewing subscription (a DB default of true would otherwise apply silently), ask the user, do not guess. Ignored for one-time purchases; forced to false for non-renewing subs.
  google_base_plan_id (string) play_store subscriptions only, the Google Play base plan id (Billing v5): the part after ":" in a combined "<subscription_id>:<base_plan_id>" SKU. OMIT it when you already included ":<base_plan_id>" in store_product_id.
  is_swg (boolean) play_store only, mark this as a Subscribe with Google distribution.
  only_if_new (boolean) When true, refuse replacing any existing has-one store distribution or re-adding a linked Stripe price. New Stripe prices remain additive. Use for safe catalog imports.
  store (string, required) Which store this SKU is for: app_store (iOS), play_store (Android), amazon, huawei, or stripe.
  store_product_id (string) The store SKU / product identifier for app_store/play_store/amazon/huawei. Frozen once a purchase has been made against it. For a play_store subscription the Google Play SKU is "<subscription_id>:<base_plan_id>" (":" separator), pass the combined value here directly (do this when the user gives you a SKU containing ":"), OR pass the subscription_id here and the base plan id separately in google_base_plan_id. Never do both, or the base plan id is duplicated.
  store_product_ids (array of string) stripe only, the Stripe price IDs. A plan can have many stripe distributions; pass the full set (stripe is has_many, not one-per-plan).

Example:
  purchasely distributions create --app app_xxx --plan-id <plan-id> --data @body.json --env prod
```

### `purchasely distributions delete`

```text
Usage: purchasely distributions delete [options]

Remove a store's distribution from a plan

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --plan-id <value>  Public ID of the resource. [string, required]
  --store <value>    Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Remove a store's distribution from a plan. For stripe this clears the plan's entire stripe set. A distribution that already has purchases cannot be removed.

Required scope: setup.plans:edit
Request:
  DELETE /management/v1/applications/{application_id}/plans/{plan_id}/distributions/{store}

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely distributions delete --app app_xxx --plan-id <plan-id> --store <store> --env prod
```

### `purchasely distributions get`

```text
Usage: purchasely distributions get [options]

Show a plan's distribution for one store (app_store/play_store/amazon/huawei/stripe)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --plan-id <value>  Public ID of the resource. [string, required]
  --store <value>    Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a plan's distribution for one store (app_store/play_store/amazon/huawei/stripe). Stripe returns an array (a plan can have many stripe distributions).

Required scope: setup.plans:view
Request:
  GET /management/v1/applications/{application_id}/plans/{plan_id}/distributions/{store}

Example:
  purchasely distributions get --app app_xxx --plan-id <plan-id> --store <store>
```

### `purchasely distributions list`

```text
Usage: purchasely distributions list [options]

List every store distribution (SKU) configured for a plan

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --plan-id <value>  Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

List every store distribution (SKU) configured for a plan.

Required scope: setup.plans:view
Request:
  GET /management/v1/applications/{application_id}/plans/{plan_id}/distributions

Example:
  purchasely distributions list --app app_xxx --plan-id <plan-id>
```

### `purchasely distributions update`

```text
Usage: purchasely distributions update [options]

Update a store's distribution on a plan (its store_product_id and, for play_store, the Google Play
base plan / SwG settings)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --plan-id <value>  Public ID of the resource. [string, required]
  --store <value>    Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a store's distribution on a plan (its store_product_id and, for play_store, the Google Play base plan / SwG settings). For stripe, store_product_ids REPLACES the whole stripe set. store_product_id is frozen once a purchase has been made.

Required scope: setup.plans:edit
Request:
  PATCH /management/v1/applications/{application_id}/plans/{plan_id}/distributions/{store}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  google_base_plan_backwards_compatible (boolean) play_store subscriptions only, true keeps Google Play Billing v4 backwards compatibility, false is v5-only. REQUIRED for a Play Store renewing subscription (a DB default of true would otherwise apply silently), ask the user, do not guess. Ignored for one-time purchases; forced to false for non-renewing subs.
  google_base_plan_id (string) play_store subscriptions only, the Google Play base plan id (Billing v5): the part after ":" in a combined "<subscription_id>:<base_plan_id>" SKU. OMIT it when you already included ":<base_plan_id>" in store_product_id.
  is_swg (boolean) play_store only, mark this as a Subscribe with Google distribution.
  store_product_id (string) The store SKU / product identifier for app_store/play_store/amazon/huawei. Frozen once a purchase has been made against it. For a play_store subscription the Google Play SKU is "<subscription_id>:<base_plan_id>" (":" separator), pass the combined value here directly (do this when the user gives you a SKU containing ":"), OR pass the subscription_id here and the base plan id separately in google_base_plan_id. Never do both, or the base plan id is duplicated.
  store_product_ids (array of string) stripe only, the Stripe price IDs. REPLACES the plan's entire stripe distribution set: send the full desired set; any existing price id you omit is removed.

Example:
  purchasely distributions update --app app_xxx --plan-id <plan-id> --store <store> --data @body.json --env prod
```

## flows

Management API: flows

### `purchasely flows`

```text
Usage: purchasely flows [options] [command]

Management API: flows

Options:
  -h, --help        display help for command

Commands:
  list [options]    List the in-app flows of an application, newest first
  create [options]  Create an in-app flow: a graph of steps, each displaying a screen, that the SDK
                    opens through the deeplink ply/flows/<vendor_id>
  get [options]     Show an in-app flow with its published and draft content
  update [options]  Update an in-app flow (partial: omitted fields are left untouched)
  delete [options]  Discard (soft-delete) an in-app flow
  help [command]    display help for command
```

### `purchasely flows create`

```text
Usage: purchasely flows create [options]

Create an in-app flow: a graph of steps, each displaying a screen, that the SDK opens through the
deeplink ply/flows/<vendor_id>

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create an in-app flow: a graph of steps, each displaying a screen, that the SDK opens through the deeplink ply/flows/<vendor_id>. Send the same content as draft_content to publish it right away.

Required scope: experiences.flows:create
Request:
  POST /management/v1/applications/{application_id}/flows

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  app_flow (boolean) Defaults to true.
  content (object) Published content. Send the same value as draft_content to publish on creation.
  default_presentation_store_public_id (string) Screen public id (pres_…) shown when a step cannot be resolved.
  draft_content (object) Draft content edited in the console before publishing.
  name (string, required) Display name shown in console.
  vendor_id (string, required) Stable machine identifier used by the SDK deeplink ply/flows/<vendor_id>; unique within the app.
  web_flow (boolean) Must be false: web2app flows are not available on this API.

Example:
  purchasely flows create --app app_xxx --data @body.json --env prod
```

### `purchasely flows delete`

```text
Usage: purchasely flows delete [options]

Discard (soft-delete) an in-app flow

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Discard (soft-delete) an in-app flow. Refused while a draft or running A/B test uses it.

Required scope: experiences.flows:delete
Request:
  DELETE /management/v1/applications/{application_id}/flows/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely flows delete --app app_xxx --id <id> --env prod
```

### `purchasely flows get`

```text
Usage: purchasely flows get [options]

Show an in-app flow with its published and draft content

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show an in-app flow with its published and draft content.

Required scope: experiences.flows:view
Request:
  GET /management/v1/applications/{application_id}/flows/{id}

Example:
  purchasely flows get --app app_xxx --id <id>
```

### `purchasely flows list`

```text
Usage: purchasely flows list [options]

List the in-app flows of an application, newest first

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (required)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List the in-app flows of an application, newest first. Web2app flows are not listed here.

Required scope: experiences.flows:view
Request:
  GET /management/v1/applications/{application_id}/flows

Example:
  purchasely flows list --app app_xxx
```

### `purchasely flows update`

```text
Usage: purchasely flows update [options]

Update an in-app flow (partial: omitted fields are left untouched)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update an in-app flow (partial: omitted fields are left untouched). Sending content publishes it; draft_content only changes the draft edited in the console.

Required scope: experiences.flows:edit
Request:
  PATCH /management/v1/applications/{application_id}/flows/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  app_flow (boolean) Defaults to true.
  content (object) Published content. Send the same value as draft_content to publish.
  default_presentation_store_public_id (string) Screen public id (pres_…) shown when a step cannot be resolved.
  draft_content (object) Draft content edited in the console before publishing.
  name (string) Display name shown in console.
  vendor_id (string) Stable machine identifier used by the SDK deeplink ply/flows/<vendor_id>; unique within the app.
  web_flow (boolean) Must be false: web2app flows are not available on this API.

Example:
  purchasely flows update --app app_xxx --id <id> --data @body.json --env prod
```

## fonts

Management API: fonts

### `purchasely fonts`

```text
Usage: purchasely fonts [options] [command]

Management API: fonts

Options:
  -h, --help        display help for command

Commands:
  list [options]    List custom fonts for an application
  create [options]  Create a custom font on an application
  get [options]     Show a single custom font for an application
  update [options]  Update a custom font on an application
  help [command]    display help for command
```

### `purchasely fonts create`

```text
Usage: purchasely fonts create [options]

Create a custom font on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a custom font on an application.

Required scope: setup.fonts:create
Request:
  POST /management/v1/applications/{application_id}/fonts

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  android_name (string) Android-specific font family name.
  ios_name (string) iOS-specific PostScript font name.
  name (string, required) Font family display name (unique within the app).
  url (string) URL the custom font file is hosted at.

Example:
  purchasely fonts create --app app_xxx --data @body.json --env prod
```

### `purchasely fonts get`

```text
Usage: purchasely fonts get [options]

Show a single custom font for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a single custom font for an application.

Required scope: setup.fonts:view
Request:
  GET /management/v1/applications/{application_id}/fonts/{id}

Example:
  purchasely fonts get --app app_xxx --id <id>
```

### `purchasely fonts list`

```text
Usage: purchasely fonts list [options]

List custom fonts for an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --search <value>    Case-insensitive search on the font's name. [string]
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List custom fonts for an application.

Required scope: setup.fonts:view
Request:
  GET /management/v1/applications/{application_id}/fonts  (with --app)
  GET /management/v1/fonts  (without --app)

Example:
  purchasely fonts list --app app_xxx
```

### `purchasely fonts update`

```text
Usage: purchasely fonts update [options]

Update a custom font on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a custom font on an application.

Required scope: setup.fonts:edit
Request:
  PATCH /management/v1/applications/{application_id}/fonts/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  android_name (string) Android-specific font family name.
  ios_name (string) iOS-specific PostScript font name.
  name (string) Font family display name (unique within the app).
  url (string) URL the custom font file is hosted at.

Example:
  purchasely fonts update --app app_xxx --id <id> --data @body.json --env prod
```

## games

Play a game in the terminal

### `purchasely games`

```text
Usage: purchasely games [options] [game]

Play a game in the terminal

Arguments:
  game        The name of the game. Without a name, a menu shows the games

Options:
  -h, --help  display help for command

Games: 2048, invaders, minesweeper, pong, snake, tetris.
Keys: arrows or WASD move, space acts, q, Esc or Ctrl-C quits.
It needs an interactive terminal. It uses no key and sends no request.
Example:
  purchasely games snake
```

## one-time-purchases

Management API: one-time-purchases

### `purchasely one-time-purchases`

```text
Usage: purchasely one-time-purchases [options] [command]

Management API: one-time-purchases

Options:
  -h, --help      display help for command

Commands:
  list [options]  List purchasely one-time purchases for an application (or across every application
                  you can access)
  get [options]   Show a purchasely one-time purchase, including its webhook delivery history
  help [command]  display help for command
```

### `purchasely one-time-purchases get`

```text
Usage: purchasely one-time-purchases get [options]

Show a purchasely one-time purchase, including its webhook delivery history

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a purchasely one-time purchase, including its webhook delivery history.

Required scope: lookups.one_time_purchases:view
Request:
  GET /management/v1/applications/{application_id}/purchasely_one_time_purchases/{id}

Example:
  purchasely one-time-purchases get --app app_xxx --id <id>
```

### `purchasely one-time-purchases list`

```text
Usage: purchasely one-time-purchases list [options]

List purchasely one-time purchases for an application (or across every application you can access)

Options:
  --output <format>         json (default) or table
  --env <env>               prod or staging. Mandatory for writes
  --dry-run                 print the request without sending it
  --yes                     accept the confirmation prompt of a risky change
  --reason <text>           why you run this command (sent in the User-Agent)
  --rate <n>                requests per second, lower than the default
  --i-asked-backend         allow a rate above the hard cap
  --out <file>              write the JSON output to a file, or - for stdout
  --app <app_id>            application public id (optional: omit for all apps)
  --countries <a,b>         Filter by store country ISO codes. [array of string] comma separated or
                            repeated
  --otp-plans <a,b>         Filter by (one-time-purchase) plan public_id. [array of string] comma
                            separated or repeated
  --purchase-state <a,b>    Filter by purchase state. [array of string] comma separated or repeated
  --transaction-id <value>  Filter by exact original_purchase_id (transaction id). [string]
  --fields <a,b>            Field selection for token economy: an explicit list of field names to
                            include in the response, or ["*"] for every field. Omit for this
                            endpoint's own default field set. [array of string] comma separated or
                            repeated
  --page <value>            Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>        Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help                display help for command

List purchasely one-time purchases for an application (or across every application you can access).

Required scope: lookups.one_time_purchases:view
Request:
  GET /management/v1/applications/{application_id}/purchasely_one_time_purchases  (with --app)
  GET /management/v1/purchasely_one_time_purchases  (without --app)

Example:
  purchasely one-time-purchases list --app app_xxx
```

## placements

Management API: placements

### `purchasely placements`

```text
Usage: purchasely placements [options] [command]

Management API: placements

Options:
  -h, --help                 display help for command

Commands:
  list [options]             List placements for an application
  create [options]           Create a placement on an application
  get [options]              Show a placement
  update [options]           Update a placement (partial, omitted fields are left untouched, except
                             category_tag_ids, which fully replaces the existing set when sent)
  delete [options]           Discard (soft-delete) a placement
  audiences [options]        Replace the ordered audience-targeting list for a placement
  delete-audience [options]  Remove a single audience-targeting rule from a placement, by the
                             audience's public_id
  duplicate [options]        Duplicate a placement's audience-targeting configuration, either into a
                             NEW placement (name + vendor_id) or by copying it onto an EXISTING
                             placement (target_id)
  make-default [options]     Set a placement as the application's default placement (shown when no
                             other placement matches)
  help [command]             display help for command
```

### `purchasely placements audiences`

```text
Usage: purchasely placements audiences [options]

Replace the ordered audience-targeting list for a placement

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Replace the ordered audience-targeting list for a placement. Must include every currently-targeted audience (even if unchanged), an outdated/stale list is rejected with VALIDATION_FAILED. Every audience and presentable referenced must belong to this application, or the call is rejected.

Required scope: targeting.placements:edit
Request:
  PUT /management/v1/applications/{application_id}/placements/{id}/audiences

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  audiences (array of object, required) Ordered list of { id: <audience public_id>, priority: <integer>, presentable_id: <optional> }. Fully replaces the existing targeting list, every currently-targeted audience must be present (even if unchanged) or the call is rejected as stale; new entries may be added. Every audience/presentable must belong to this placement's mobile application.

Example:
  purchasely placements audiences --app app_xxx --id <id> --data @body.json --env prod
```

### `purchasely placements create`

```text
Usage: purchasely placements create [options]

Create a placement on an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a placement on an application. The first placement created for an app automatically becomes its default.

Required scope: targeting.placements:create
Request:
  POST /management/v1/applications/{application_id}/placements

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate.
  default_presentable_id (string) Screen/flow (pres_.../flow_...) shown when no targeting audience matches.
  description (string) Free-text description of the placement.
  name (string, required) Display name shown in console.
  vendor_id (string, required) Stable machine identifier used in SDK calls.

Example:
  purchasely placements create --app app_xxx --data @body.json --env prod
```

### `purchasely placements delete`

```text
Usage: purchasely placements delete [options]

Discard (soft-delete) a placement

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --only-if-unused         Safe cleanup: refuse any incoming references and never remove
                           associations. Defaults to false. [boolean]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Discard (soft-delete) a placement. Requires confirm_token on retry if the placement is still associated with running campaigns/audiences. Blocked outright (non-overridable) if the placement has draft/running AB tests or is the app's default placement.

Required scope: targeting.placements:delete
Request:
  DELETE /management/v1/applications/{application_id}/placements/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Confirmation flow: a risky change first returns the impact and a confirm_token (query).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely placements delete --app app_xxx --id <id> --env prod
```

### `purchasely placements delete-audience`

```text
Usage: purchasely placements delete-audience [options]

Remove a single audience-targeting rule from a placement, by the audience's public_id

Options:
  --output <format>      json (default) or table
  --env <env>            prod or staging. Mandatory for writes
  --dry-run              print the request without sending it
  --yes                  accept the confirmation prompt of a risky change
  --reason <text>        why you run this command (sent in the User-Agent)
  --rate <n>             requests per second, lower than the default
  --i-asked-backend      allow a rate above the hard cap
  --out <file>           write the JSON output to a file, or - for stdout
  --app <app_id>         application public id (required)
  --id <value>           Public ID of the resource. [string, required]
  --audience-id <value>  Public ID of the resource. [string, required]
  --fields <a,b>         Field selection for token economy: an explicit list of field names to
                         include in the response, or ["*"] for every field. Omit for this endpoint's
                         own default field set. [array of string] comma separated or repeated
  -h, --help             display help for command

Remove a single audience-targeting rule from a placement, by the audience's public_id.

Required scope: targeting.placements:edit
Request:
  DELETE /management/v1/applications/{application_id}/placements/{id}/audiences/{audience_id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely placements delete-audience --app app_xxx --id <id> --audience-id <audience-id> --env prod
```

### `purchasely placements duplicate`

```text
Usage: purchasely placements duplicate [options]

Duplicate a placement's audience-targeting configuration, either into a NEW placement (name +
vendor_id) or by copying it onto an EXISTING placement (target_id)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Duplicate a placement's audience-targeting configuration, either into a NEW placement (name + vendor_id) or by copying it onto an EXISTING placement (target_id). Exactly one of target_id or (name and vendor_id) must be given.

Required scope: targeting.placements:create
Request:
  POST /management/v1/applications/{application_id}/placements/{id}/duplicate

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  name (string) Display name for the new placement. Required when target_id is omitted.
  target_id (string) Existing placement public_id to copy audiences onto. Mutually exclusive with name/vendor_id.
  vendor_id (string) Stable machine identifier for the new placement. Required when target_id is omitted.

Example:
  purchasely placements duplicate --app app_xxx --id <id> --data @body.json --env prod
```

### `purchasely placements get`

```text
Usage: purchasely placements get [options]

Show a placement

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a placement.

Required scope: targeting.placements:view
Request:
  GET /management/v1/applications/{application_id}/placements/{id}

Example:
  purchasely placements get --app app_xxx --id <id>
```

### `purchasely placements list`

```text
Usage: purchasely placements list [options]

List placements for an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List placements for an application.

Required scope: targeting.placements:view
Request:
  GET /management/v1/applications/{application_id}/placements  (with --app)
  GET /management/v1/placements  (without --app)

Example:
  purchasely placements list --app app_xxx
```

### `purchasely placements make-default`

```text
Usage: purchasely placements make-default [options]

Set a placement as the application's default placement (shown when no other placement matches)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Set a placement as the application's default placement (shown when no other placement matches).

Required scope: targeting.placements:edit
Request:
  PUT /management/v1/applications/{application_id}/placements/{id}/make_default

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely placements make-default --app app_xxx --id <id> --env prod
```

### `purchasely placements update`

```text
Usage: purchasely placements update [options]

Update a placement (partial, omitted fields are left untouched, except category_tag_ids, which fully
replaces the existing set when sent)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a placement (partial, omitted fields are left untouched, except category_tag_ids, which fully replaces the existing set when sent).

Required scope: targeting.placements:edit
Request:
  PATCH /management/v1/applications/{application_id}/placements/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  category_tag_ids (array of string) Category tag public IDs (cat_…) to associate. Fully replaces the existing set when sent.
  default_presentable_id (string) Screen/flow (pres_.../flow_...) shown when no targeting audience matches.
  description (string) Free-text description of the placement.
  name (string) Display name shown in console.
  vendor_id (string) Stable machine identifier used in SDK calls.

Example:
  purchasely placements update --app app_xxx --id <id> --data @body.json --env prod
```

## plans

Management API: plans

### `purchasely plans`

```text
Usage: purchasely plans [options] [command]

Management API: plans

Options:
  -h, --help        display help for command

Commands:
  list [options]    List every plan configured for a product
  create [options]  Create a plan for a product
  get [options]     Show a plan
  update [options]  Update a plan's attributes and/or promo offers
  delete [options]  Permanently delete a plan
  help [command]    display help for command
```

### `purchasely plans create`

```text
Usage: purchasely plans create [options]

Create a plan for a product

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --data <json>            request body: JSON, @file.json or - for stdin
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Create a plan for a product. product_type/period_type/level define the plan and cannot be changed once it has purchases, collect them from the user, never guess. Store distributions (SKUs) are managed separately, via the distributions endpoints.

Required scope: setup.plans:create
Request:
  POST /management/v1/applications/{application_id}/plans

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  commitment_billing_type (string) iOS App Store commitment billing mode for a subscription (optional). "up_front" = charged the full period at purchase ("1 Year Upfront", the default behavior). "monthly" = "Monthly with 12-Month Commitment", charged monthly with a 12-month commitment; requires a matching SKU in App Store Connect (SDK v6.0+, iOS 26.4+). Worth surfacing for yearly subscription plans; leave blank otherwise.
  confirm_token (string) Confirmation token from a prior confirmation_required response; required to actually create the plan.
  default_screen_id (string) Public ID of the screen (pres_…) to use as this plan's default.
  internal_notes (string) Internal-only notes, never shown to end users.
  level (integer, required) Ranking level (integer) used to compare plans for upgrade/downgrade detection. Ask the user which level this plan should have; do not guess or default it.
  name (string, required) Display name shown in console.
  period_type (string) ISO-8601-ish billing period, e.g. P1M, P1Y. REQUIRED for subscription plans (RENEWING_SUBSCRIPTION / NON_RENEWING_SUBSCRIPTION); leave blank for CONSUMABLE / NON_CONSUMABLE. Effectively permanent once the plan has purchases, ask the user.
  product_id (string, required) Public ID of the product (prod_…) this plan belongs to.
  product_type (string, required) One of CONSUMABLE, NON_CONSUMABLE, NON_RENEWING_SUBSCRIPTION, RENEWING_SUBSCRIPTION. PERMANENT, cannot be changed once the plan has purchases. Ask the user; never guess.
  promo_offers (array of object) Promo offers for this plan.
  vendor_id (string, required) Unique vendor identifier for this plan, scoped to the mobile application.

Confirmation flow: a risky change first returns the impact and a confirm_token (body).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely plans create --app app_xxx --data @body.json --env prod
```

### `purchasely plans delete`

```text
Usage: purchasely plans delete [options]

Permanently delete a plan

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Permanently delete a plan. REST-only, requires confirmation.

Required scope: setup.plans:delete
Request:
  DELETE /management/v1/applications/{application_id}/plans/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Confirmation flow: a risky change first returns the impact and a confirm_token (query).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely plans delete --app app_xxx --id <id> --env prod
```

### `purchasely plans get`

```text
Usage: purchasely plans get [options]

Show a plan

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a plan.

Required scope: setup.plans:view
Request:
  GET /management/v1/applications/{application_id}/plans/{id}

Example:
  purchasely plans get --app app_xxx --id <id>
```

### `purchasely plans list`

```text
Usage: purchasely plans list [options]

List every plan configured for a product

Options:
  --output <format>     json (default) or table
  --env <env>           prod or staging. Mandatory for writes
  --dry-run             print the request without sending it
  --yes                 accept the confirmation prompt of a risky change
  --reason <text>       why you run this command (sent in the User-Agent)
  --rate <n>            requests per second, lower than the default
  --i-asked-backend     allow a rate above the hard cap
  --out <file>          write the JSON output to a file, or - for stdout
  --app <app_id>        application public id (required)
  --product-id <value>  Public ID of the product whose plans to list. [string, required]
  --fields <a,b>        Field selection for token economy: an explicit list of field names to
                        include in the response, or ["*"] for every field. Omit for this endpoint's
                        own default field set. [array of string] comma separated or repeated
  -h, --help            display help for command

List every plan configured for a product.

Required scope: setup.plans:view
Request:
  GET /management/v1/applications/{application_id}/plans

Example:
  purchasely plans list --app app_xxx --product-id <product-id>
```

### `purchasely plans update`

```text
Usage: purchasely plans update [options]

Update a plan's attributes and/or promo offers

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a plan's attributes and/or promo offers. Store distributions (SKUs) are managed separately, via the distributions endpoints.

Required scope: setup.plans:edit
Request:
  PATCH /management/v1/applications/{application_id}/plans/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  commitment_billing_type (string) iOS App Store commitment billing mode for a subscription (optional). "up_front" = charged the full period at purchase ("1 Year Upfront", the default behavior). "monthly" = "Monthly with 12-Month Commitment", charged monthly with a 12-month commitment; requires a matching SKU in App Store Connect (SDK v6.0+, iOS 26.4+). Worth surfacing for yearly subscription plans; leave blank otherwise.
  default_screen_id (string) Public ID of the screen (pres_…) to use as this plan's default.
  internal_notes (string) Internal-only notes, never shown to end users.
  level (integer) Ranking level used to compare plans (upgrade/downgrade detection).
  name (string) Display name shown in console.
  period_type (string) ISO-8601-ish duration, e.g. P1M, P1Y (required for subscriptions).
  product_type (string) One of CONSUMABLE, NON_CONSUMABLE, NON_RENEWING_SUBSCRIPTION, RENEWING_SUBSCRIPTION.
  promo_offers (array of object) Promo offers for this plan. Each item accepts id (public_id) + _destroy to edit/remove an existing one.
  vendor_id (string) Unique vendor identifier for this plan, scoped to the mobile application.

Example:
  purchasely plans update --app app_xxx --id <id> --data @body.json --env prod
```

## platforms

Management API: platforms

### `purchasely platforms`

```text
Usage: purchasely platforms [options] [command]

Management API: platforms

Options:
  -h, --help        display help for command

Commands:
  list [options]    List the store platforms (app_store/play_store/amazon/huawei) configured for an
                    application
  create [options]  Configure a store platform (app_store/play_store/amazon/huawei) for an
                    application
  get [options]     Show one store platform configured for an application
  update [options]  Update a store platform's settings and/or credentials for an application
  help [command]    display help for command
```

### `purchasely platforms create`

```text
Usage: purchasely platforms create [options]

Configure a store platform (app_store/play_store/amazon/huawei) for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Configure a store platform (app_store/play_store/amazon/huawei) for an application. Provide the store's credentials as flat fields; they are write-only and never returned. Stripe is connected out-of-band and cannot be configured here.

Required scope: setup.platforms:edit
Request:
  POST /management/v1/applications/{application_id}/platforms

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  app_id (string) app_store only, the numeric Apple App Store ID (Apple's "Apple ID" for the app).
  app_package_id (string, required) Store package / bundle identifier (e.g. com.acme.app). Cannot be changed once a purchase has been made.
  app_scheme (string) Used to preview paywalls. EITHER a bare URL-scheme name WITHOUT "://" (e.g. "myapp", not "myapp://") OR a full http/https universal-link prefix (e.g. "https://acme.com/app"). A scheme written with a trailing "://" is rejected.
  app_store_connect_api_key_id (string) app_store only, the App Store Connect API Key ID (e.g. ABC123DEFG). Create the key in App Store Connect > Users and Access > Integrations > App Store Connect API. Used for Apple catalogue discovery (SKUs) and Retention Messaging. Distinct from the legacy In-App Purchase key (private_key_name / private_key_value). Write-only, never returned.
  app_store_connect_api_key_value (string) app_store only, the full contents of the App Store Connect API private key (.p8 file, including the BEGIN/END PRIVATE KEY lines), paired with app_store_connect_api_key_id and issuer_id. Used for Apple catalogue discovery (SKUs) and Retention Messaging. Distinct from the legacy In-App Purchase key (private_key_name / private_key_value). Write-only, never returned.
  client_id (string) huawei only, the Huawei AppGallery OAuth client id.
  issuer_id (string) app_store only, the App Store Connect API Issuer ID (a UUID). Find it in App Store Connect > Users and Access > Integrations > App Store Connect API, at the top of the Active keys list. Write-only, never returned. Setup: https://docs.purchasely.com/docs/app-store-configuration
  private_key_name (string) app_store only, legacy in-app-purchase private key name. Write-only.
  private_key_value (string) app_store only, legacy in-app-purchase private key value. Write-only.
  public_key (string) huawei only, the AppGallery public key. Write-only.
  shared_app_secret (string) app_store: App-Specific Shared Secret. play_store: the Google service-account JSON credentials (as a JSON string). amazon/huawei: the store shared secret. Write-only, never returned. Setup: https://docs.purchasely.com/docs/app-store-configuration (iOS), https://docs.purchasely.com/docs/play-store-configuration (Android).
  store (string, required) Which store to configure: app_store (iOS), play_store (Android), amazon, or huawei. Stripe is NOT configurable here (connected out-of-band via Stripe Connect).

Example:
  purchasely platforms create --app app_xxx --data @body.json --env prod
```

### `purchasely platforms get`

```text
Usage: purchasely platforms get [options]

Show one store platform configured for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --store <value>    Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show one store platform configured for an application.

Required scope: setup.platforms:view
Request:
  GET /management/v1/applications/{application_id}/platforms/{store}

Example:
  purchasely platforms get --app app_xxx --store <store>
```

### `purchasely platforms list`

```text
Usage: purchasely platforms list [options]

List the store platforms (app_store/play_store/amazon/huawei) configured for an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (required)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List the store platforms (app_store/play_store/amazon/huawei) configured for an application.

Required scope: setup.platforms:view
Request:
  GET /management/v1/applications/{application_id}/platforms

Example:
  purchasely platforms list --app app_xxx
```

### `purchasely platforms update`

```text
Usage: purchasely platforms update [options]

Update a store platform's settings and/or credentials for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --store <value>    Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a store platform's settings and/or credentials for an application. Credential fields are write-only and never returned.

Required scope: setup.platforms:edit
Request:
  PATCH /management/v1/applications/{application_id}/platforms/{store}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  app_id (string) app_store only, the numeric Apple App Store ID (Apple's "Apple ID" for the app).
  app_package_id (string) Store package / bundle identifier. Cannot be changed once a purchase has been made.
  app_scheme (string) Used to preview paywalls. EITHER a bare URL-scheme name WITHOUT "://" (e.g. "myapp", not "myapp://") OR a full http/https universal-link prefix (e.g. "https://acme.com/app"). A scheme written with a trailing "://" is rejected.
  app_store_connect_api_key_id (string) app_store only, the App Store Connect API Key ID (e.g. ABC123DEFG). Create the key in App Store Connect > Users and Access > Integrations > App Store Connect API. Used for Apple catalogue discovery (SKUs) and Retention Messaging. Distinct from the legacy In-App Purchase key (private_key_name / private_key_value). Write-only, never returned.
  app_store_connect_api_key_value (string) app_store only, the full contents of the App Store Connect API private key (.p8 file, including the BEGIN/END PRIVATE KEY lines), paired with app_store_connect_api_key_id and issuer_id. Used for Apple catalogue discovery (SKUs) and Retention Messaging. Distinct from the legacy In-App Purchase key (private_key_name / private_key_value). Write-only, never returned.
  client_id (string) huawei only, the Huawei AppGallery OAuth client id.
  issuer_id (string) app_store only, the App Store Connect API Issuer ID (a UUID). Find it in App Store Connect > Users and Access > Integrations > App Store Connect API, at the top of the Active keys list. Write-only, never returned. Setup: https://docs.purchasely.com/docs/app-store-configuration
  private_key_name (string) app_store only, legacy in-app-purchase private key name. Write-only.
  private_key_value (string) app_store only, legacy in-app-purchase private key value. Write-only.
  public_key (string) huawei only, the AppGallery public key. Write-only.
  shared_app_secret (string) app_store: App-Specific Shared Secret. play_store: the Google service-account JSON credentials (as a JSON string). amazon/huawei: the store shared secret. Write-only, never returned. Setup: https://docs.purchasely.com/docs/app-store-configuration (iOS), https://docs.purchasely.com/docs/play-store-configuration (Android).

Example:
  purchasely platforms update --app app_xxx --store <store> --data @body.json --env prod
```

## products

Management API: products

### `purchasely products`

```text
Usage: purchasely products [options] [command]

Management API: products

Options:
  -h, --help        display help for command

Commands:
  list [options]    List products for an application
  create [options]  Create a product for an application
  get [options]     Show a product
  update [options]  Update a product's name, description, icon, and/or default screen
  delete [options]  Permanently delete a product
  help [command]    display help for command
```

### `purchasely products create`

```text
Usage: purchasely products create [options]

Create a product for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a product for an application.

Required scope: setup.products:create
Request:
  POST /management/v1/applications/{application_id}/products

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  default_screen_id (string) Public ID of the screen (pres_…) to use as this product's default.
  description (string) Free-text description of the product.
  icon_url (string) URL of the product's icon.
  name (string, required) Display name shown in console.
  vendor_id (string, required) Unique vendor identifier for this product, scoped to the mobile application.

Example:
  purchasely products create --app app_xxx --data @body.json --env prod
```

### `purchasely products delete`

```text
Usage: purchasely products delete [options]

Permanently delete a product

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Permanently delete a product. REST-only, requires confirmation.

Required scope: setup.products:delete
Request:
  DELETE /management/v1/applications/{application_id}/products/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Confirmation flow: a risky change first returns the impact and a confirm_token (query).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely products delete --app app_xxx --id <id> --env prod
```

### `purchasely products get`

```text
Usage: purchasely products get [options]

Show a product

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a product.

Required scope: setup.products:view
Request:
  GET /management/v1/applications/{application_id}/products/{id}

Example:
  purchasely products get --app app_xxx --id <id>
```

### `purchasely products list`

```text
Usage: purchasely products list [options]

List products for an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List products for an application.

Required scope: setup.products:view
Request:
  GET /management/v1/applications/{application_id}/products  (with --app)
  GET /management/v1/products  (without --app)

Example:
  purchasely products list --app app_xxx
```

### `purchasely products update`

```text
Usage: purchasely products update [options]

Update a product's name, description, icon, and/or default screen

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a product's name, description, icon, and/or default screen.

Required scope: setup.products:edit
Request:
  PATCH /management/v1/applications/{application_id}/products/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  default_screen_id (string) Public ID of the screen (pres_…) to use as this product's default.
  description (string) Free-text description of the product.
  icon_url (string) URL of the product's icon.
  name (string) Display name shown in console.

Example:
  purchasely products update --app app_xxx --id <id> --data @body.json --env prod
```

## purge

Delete the cached client data

### `purchasely purge`

```text
Usage: purchasely purge [options]

Delete the cached client data

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  -h, --help         display help for command

Deletes the cached outputs over 200 KB. Without --env it deletes the cache of every environment.
```

## reference

Print the full command reference of this version (Markdown, man, HTML or JSON)

### `purchasely reference`

```text
Usage: purchasely reference [options]

Print the full command reference of this version (Markdown, man, HTML or JSON)

Options:
  --format <format>  markdown (default), man, html or json (default: "markdown")
  -h, --help         display help for command

--format json prints the tree of commands, arguments and options as JSON, for agents. Hidden commands are left out.
```

## screen-versions

Management API: screen-versions

### `purchasely screen-versions`

```text
Usage: purchasely screen-versions [options] [command]

Management API: screen-versions

Options:
  -h, --help      display help for command

Commands:
  list [options]  List a screen's versions (version metadata; the raw store content blob is not
                  included)
  get [options]   Show a screen version (version metadata; the raw store content blob is not
                  included)
  help [command]  display help for command
```

### `purchasely screen-versions get`

```text
Usage: purchasely screen-versions get [options]

Show a screen version (version metadata; the raw store content blob is not included)

Options:
  --output <format>    json (default) or table
  --env <env>          prod or staging. Mandatory for writes
  --dry-run            print the request without sending it
  --yes                accept the confirmation prompt of a risky change
  --reason <text>      why you run this command (sent in the User-Agent)
  --rate <n>           requests per second, lower than the default
  --i-asked-backend    allow a rate above the hard cap
  --out <file>         write the JSON output to a file, or - for stdout
  --app <app_id>       application public id (required)
  --screen-id <value>  Public ID of the resource. [string, required]
  --id <value>         Public ID of the resource. [string, required]
  --fields <a,b>       Field selection for token economy: an explicit list of field names to include
                       in the response, or ["*"] for every field. Omit for this endpoint's own
                       default field set. [array of string] comma separated or repeated
  -h, --help           display help for command

Show a screen version (version metadata; the raw store content blob is not included).

Required scope: experiences.screens:view
Request:
  GET /management/v1/applications/{application_id}/screens/{screen_id}/screen_versions/{id}

Example:
  purchasely screen-versions get --app app_xxx --screen-id <screen-id> --id <id>
```

### `purchasely screen-versions list`

```text
Usage: purchasely screen-versions list [options]

List a screen's versions (version metadata; the raw store content blob is not included)

Options:
  --output <format>    json (default) or table
  --env <env>          prod or staging. Mandatory for writes
  --dry-run            print the request without sending it
  --yes                accept the confirmation prompt of a risky change
  --reason <text>      why you run this command (sent in the User-Agent)
  --rate <n>           requests per second, lower than the default
  --i-asked-backend    allow a rate above the hard cap
  --out <file>         write the JSON output to a file, or - for stdout
  --app <app_id>       application public id (required)
  --screen-id <value>  Public ID of the resource. [string, required]
  --fields <a,b>       Field selection for token economy: an explicit list of field names to include
                       in the response, or ["*"] for every field. Omit for this endpoint's own
                       default field set. [array of string] comma separated or repeated
  -h, --help           display help for command

List a screen's versions (version metadata; the raw store content blob is not included).

Required scope: experiences.screens:view
Request:
  GET /management/v1/applications/{application_id}/screens/{screen_id}/screen_versions

Example:
  purchasely screen-versions list --app app_xxx --screen-id <screen-id>
```

## screens

Management API: screens

### `purchasely screens`

```text
Usage: purchasely screens [options] [command]

Management API: screens

Options:
  -h, --help              display help for command

Commands:
  list [options]          List screens (paywalls) for an application, metadata only, no
                          content/draft_content
  get [options]           Show a screen (paywall), metadata only, no content/draft_content
  delete [options]        Discard (soft-delete) a screen
  make-default [options]  Set a screen as the application's default screen
  help [command]          display help for command
```

### `purchasely screens delete`

```text
Usage: purchasely screens delete [options]

Discard (soft-delete) a screen

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --id <value>             Public ID of the resource. [string, required]
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Discard (soft-delete) a screen. Requires confirm_token on retry if the screen is still associated with placements/audiences. Blocked outright (non-overridable) if the screen has draft/running A/B tests, is the app's default screen, or is referenced by other screens/flows.

Required scope: experiences.screens:delete
Request:
  DELETE /management/v1/applications/{application_id}/screens/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Confirmation flow: a risky change first returns the impact and a confirm_token (query).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely screens delete --app app_xxx --id <id> --env prod
```

### `purchasely screens get`

```text
Usage: purchasely screens get [options]

Show a screen (paywall), metadata only, no content/draft_content

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a screen (paywall), metadata only, no content/draft_content.

Required scope: experiences.screens:view
Request:
  GET /management/v1/applications/{application_id}/screens/{id}

Example:
  purchasely screens get --app app_xxx --id <id>
```

### `purchasely screens list`

```text
Usage: purchasely screens list [options]

List screens (paywalls) for an application, metadata only, no content/draft_content

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List screens (paywalls) for an application, metadata only, no content/draft_content.

Required scope: experiences.screens:view
Request:
  GET /management/v1/applications/{application_id}/screens  (with --app)
  GET /management/v1/screens  (without --app)

Example:
  purchasely screens list --app app_xxx
```

### `purchasely screens make-default`

```text
Usage: purchasely screens make-default [options]

Set a screen as the application's default screen

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Set a screen as the application's default screen.

Required scope: experiences.screens:publish
Request:
  PUT /management/v1/applications/{application_id}/screens/{id}/make_default

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely screens make-default --app app_xxx --id <id> --env prod
```

## search

Find a command for a task

### `purchasely search`

```text
Usage: purchasely search [options] <task>

Find a command for a task

Arguments:
  task               what you want to do

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --limit <n>        number of results (default: "10")
  -h, --help         display help for command

Prints a JSON array, best match first. It needs no credential.
Example:
  purchasely search "list the placements of an app"
```

## subscriptions

Management API: subscriptions

### `purchasely subscriptions`

```text
Usage: purchasely subscriptions [options] [command]

Management API: subscriptions

Options:
  -h, --help      display help for command

Commands:
  list [options]  List purchasely subscriptions for an application (or across every application you
                  can access)
  get [options]   Show a purchasely subscription, including its webhook delivery history
  help [command]  display help for command
```

### `purchasely subscriptions get`

```text
Usage: purchasely subscriptions get [options]

Show a purchasely subscription, including its webhook delivery history

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a purchasely subscription, including its webhook delivery history.

Required scope: lookups.subscriptions:view
Request:
  GET /management/v1/applications/{application_id}/purchasely_subscriptions/{id}

Example:
  purchasely subscriptions get --app app_xxx --id <id>
```

### `purchasely subscriptions list`

```text
Usage: purchasely subscriptions list [options]

List purchasely subscriptions for an application (or across every application you can access)

Options:
  --output <format>          json (default) or table
  --env <env>                prod or staging. Mandatory for writes
  --dry-run                  print the request without sending it
  --yes                      accept the confirmation prompt of a risky change
  --reason <text>            why you run this command (sent in the User-Agent)
  --rate <n>                 requests per second, lower than the default
  --i-asked-backend          allow a rate above the hard cap
  --out <file>               write the JSON output to a file, or - for stdout
  --app <app_id>             application public id (optional: omit for all apps)
  --countries <a,b>          Filter by store country ISO codes. [array of string] comma separated or
                             repeated
  --period-types <a,b>       Filter by plan period type (e.g. WEEK, MONTH, YEAR). [array of string]
                             comma separated or repeated
  --plans <a,b>              Filter by plan public_id. [array of string] comma separated or repeated
  --platform <a,b>           Filter by platform name, case-insensitive (IOS, ANDROID, AMAZON,
                             HUAWEI, STRIPE). [array of string] comma separated or repeated
  --status <a,b>             Filter by subscription status. [array of string] comma separated or
                             repeated
  --ab-test-variant <value>  Filter by ab_test_variant public_id. [string]
  --audience <value>         Filter by audience public_id. [string]
  --placement <value>        Filter by placement public_id. [string]
  --fields <a,b>             Field selection for token economy: an explicit list of field names to
                             include in the response, or ["*"] for every field. Omit for this
                             endpoint's own default field set. [array of string] comma separated or
                             repeated
  --page <value>             Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>         Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help                 display help for command

List purchasely subscriptions for an application (or across every application you can access).

Required scope: lookups.subscriptions:view
Request:
  GET /management/v1/applications/{application_id}/purchasely_subscriptions  (with --app)
  GET /management/v1/purchasely_subscriptions  (without --app)

Example:
  purchasely subscriptions list --app app_xxx
```

## surveys

Management API: surveys

### `purchasely surveys`

```text
Usage: purchasely surveys [options] [command]

Management API: surveys

Options:
  -h, --help         display help for command

Commands:
  list [options]     List surveys for an application
  get [options]      Show a single survey for an application
  results [options]  Aggregated results for a survey: answer distribution and total presentation
                     views
  help [command]     display help for command
```

### `purchasely surveys get`

```text
Usage: purchasely surveys get [options]

Show a single survey for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a single survey for an application.

Required scope: setup.surveys:view
Request:
  GET /management/v1/applications/{application_id}/surveys/{id}

Example:
  purchasely surveys get --app app_xxx --id <id>
```

### `purchasely surveys list`

```text
Usage: purchasely surveys list [options]

List surveys for an application

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --search <value>    Case-insensitive search on the survey's name. [string]
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List surveys for an application.

Required scope: setup.surveys:view
Request:
  GET /management/v1/applications/{application_id}/surveys  (with --app)
  GET /management/v1/surveys  (without --app)

Example:
  purchasely surveys list --app app_xxx
```

### `purchasely surveys results`

```text
Usage: purchasely surveys results [options]

Aggregated results for a survey: answer distribution and total presentation views

Options:
  --output <format>           json (default) or table
  --env <env>                 prod or staging. Mandatory for writes
  --dry-run                   print the request without sending it
  --yes                       accept the confirmation prompt of a risky change
  --reason <text>             why you run this command (sent in the User-Agent)
  --rate <n>                  requests per second, lower than the default
  --i-asked-backend           allow a rate above the hard cap
  --out <file>                write the JSON output to a file, or - for stdout
  --app <app_id>              application public id (required)
  --id <value>                Public ID of the resource. [string, required]
  --date-range <a,b>          Two-element [start_date, end_date] (ISO8601) window. [array of string,
                              required] comma separated or repeated
  --periodicity <value>       Bucket granularity: DAY, WEEK, MONTH or YEAR (default MONTH). [enum:
                              DAY|WEEK|MONTH|YEAR]
  --countries <a,b>           Filter by store country ISO codes (or "Unknown"). [array of string]
                              comma separated or repeated
  --platform-names <a,b>      Filter by platform name. [array of enum:
                              AMAZON|ANDROID|HUAWEI|IOS|STRIPE|SWG] comma separated or repeated
  --period-types <a,b>        Filter by plan billing period, as an ISO 8601 duration (e.g. P1W, P1M,
                              P1Y). [array of string] comma separated or repeated
  --plans <a,b>               Filter by subscription plan public_id. [array of string] comma
                              separated or repeated
  --family-sharing <a,b>      Filter by family-sharing flag ("true"/"false" strings). [array of
                              string] comma separated or repeated
  --offer-types <a,b>         Filter by offer type. [array of enum:
                              NONE|FREE_TRIAL|INTRO_OFFER|PROMOTIONAL_OFFER|PROMO_CODE] comma
                              separated or repeated
  --promotional-offers <a,b>  Filter by promotional offer public_id. [array of string] comma
                              separated or repeated
  --audiences <a,b>           Filter by audience public_id. [array of string] comma separated or
                              repeated
  --campaigns <a,b>           Filter by campaign public_id. [array of string] comma separated or
                              repeated
  --placements <a,b>          Filter by placement public_id. [array of string] comma separated or
                              repeated
  --screens <a,b>             Filter by screen (paywall) public_id. [array of string] comma
                              separated or repeated
  --ab-test-variants <a,b>    Filter by A/B test variant public_id. [array of string] comma
                              separated or repeated
  --unique-users <value>      Restrict to unique users ("true"/"false"). [string]
  --fields <a,b>              Field selection for token economy: an explicit list of field names to
                              include in the response, or ["*"] for every field. Omit for this
                              endpoint's own default field set. [array of string] comma separated or
                              repeated
  -h, --help                  display help for command

Aggregated results for a survey: answer distribution and total presentation views.

Required scope: setup.surveys:view
Request:
  GET /management/v1/applications/{application_id}/surveys/{id}/results

Example:
  purchasely surveys results --app app_xxx --id <id> --date-range <date-range>
```

## translations

Management API: translations

### `purchasely translations`

```text
Usage: purchasely translations [options] [command]

Management API: translations

Options:
  -h, --help          display help for command

Commands:
  list [options]      List translation state (languages, key count, timestamps) for an application,
                      or across every application you can reach
  create [options]    Replace ALL translations for an application from a CSV upload
                      (key,<lang1>,<lang2>,.
  download [options]  Download translation content (structured key/value map + raw CSV) for an
                      application, or across every application you can reach
  help [command]      display help for command
```

### `purchasely translations create`

```text
Usage: purchasely translations create [options]

Replace ALL translations for an application from a CSV upload (key,<lang1>,<lang2>,.

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --data <json>            request body: JSON, @file.json or - for stdin
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Replace ALL translations for an application from a CSV upload (key,<lang1>,<lang2>,... header). Destructive, requires confirmation.

Required scope: setup.translations:edit
Request:
  POST /management/v1/applications/{application_id}/translations

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  confirm_token (string) Confirmation token from a prior confirmation_required response; required to actually replace translations.
  csv (string, required) Raw CSV content, header row `key,<lang1>,<lang2>,...` (e.g. "key,en,fr\ntitle,Title,Titre").

Confirmation flow: a risky change first returns the impact and a confirm_token (body).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely translations create --app app_xxx --data @body.json --env prod
```

### `purchasely translations download`

```text
Usage: purchasely translations download [options]

Download translation content (structured key/value map + raw CSV) for an application, or across
every application you can reach

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

Download translation content (structured key/value map + raw CSV) for an application, or across every application you can reach.

Required scope: setup.translations:view
Request:
  GET /management/v1/applications/{application_id}/translations/download  (with --app)
  GET /management/v1/translations/download  (without --app)

Example:
  purchasely translations download --app app_xxx
```

### `purchasely translations list`

```text
Usage: purchasely translations list [options]

List translation state (languages, key count, timestamps) for an application, or across every
application you can reach

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (optional: omit for all apps)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List translation state (languages, key count, timestamps) for an application, or across every application you can reach.

Required scope: setup.translations:view
Request:
  GET /management/v1/applications/{application_id}/translations  (with --app)
  GET /management/v1/translations  (without --app)

Example:
  purchasely translations list --app app_xxx
```

## update

Check for a new version and install it (managed install) or print the upgrade command

### `purchasely update`

```text
Usage: purchasely update [options]

Check for a new version and install it (managed install) or print the upgrade command

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  -h, --help         display help for command

A managed install installs the new version. Another install prints the upgrade command. See docs/install.md.
```

## usage

Requests of the last hour, rejections, limiter and breaker state

### `purchasely usage`

```text
Usage: purchasely usage [options]

Requests of the last hour, rejections, limiter and breaker state

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  -h, --help         display help for command

Management API only.
```

## users

Management API: users

### `purchasely users`

```text
Usage: purchasely users [options] [command]

Management API: users

Options:
  -h, --help      display help for command

Commands:
  list [options]  Search for an end user by anonymous_id or vendor_id (returns 0 or 1 result)
  get [options]   Show a full end-user profile, including transaction/event history and devices
  help [command]  display help for command
```

### `purchasely users get`

```text
Usage: purchasely users get [options]

Show a full end-user profile, including transaction/event history and devices

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a full end-user profile, including transaction/event history and devices.

Required scope: lookups.users:view
Request:
  GET /management/v1/applications/{application_id}/users/{id}

Example:
  purchasely users get --app app_xxx --id <id>
```

### `purchasely users list`

```text
Usage: purchasely users list [options]

Search for an end user by anonymous_id or vendor_id (returns 0 or 1 result)

Options:
  --output <format>     json (default) or table
  --env <env>           prod or staging. Mandatory for writes
  --dry-run             print the request without sending it
  --yes                 accept the confirmation prompt of a risky change
  --reason <text>       why you run this command (sent in the User-Agent)
  --rate <n>            requests per second, lower than the default
  --i-asked-backend     allow a rate above the hard cap
  --out <file>          write the JSON output to a file, or - for stdout
  --app <app_id>        application public id (optional: omit for all apps)
  --identifier <value>  The user's anonymous_id or vendor_id to search for. [string, required]
  --fields <a,b>        Field selection for token economy: an explicit list of field names to
                        include in the response, or ["*"] for every field. Omit for this endpoint's
                        own default field set. [array of string] comma separated or repeated
  -h, --help            display help for command

Search for an end user by anonymous_id or vendor_id (returns 0 or 1 result).

Required scope: lookups.users:view
Request:
  GET /management/v1/applications/{application_id}/users  (with --app)
  GET /management/v1/users  (without --app)

Example:
  purchasely users list --app app_xxx --identifier <identifier>
```

## web-activations

Management API: web-activations

### `purchasely web-activations`

```text
Usage: purchasely web-activations [options] [command]

Management API: web-activations

Options:
  -h, --help                   display help for command

Commands:
  flow [options]               Show the web-activation status (slug, public/sandbox URL) for a flow
  screen [options]             Show the web-activation status (slug, public/sandbox URL) for a
                               screen (paywall)
  activate-web-flow [options]  Activate a web2app flow: creates or enables its web slug and returns
                               the public and sandbox URLs
  help [command]               display help for command
```

### `purchasely web-activations activate-web-flow`

```text
Usage: purchasely web-activations activate-web-flow [options]

Activate a web2app flow: creates or enables its web slug and returns the public and sandbox URLs

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Activate a web2app flow: creates or enables its web slug and returns the public and sandbox URLs. Idempotent.

Required scope: experiences.web2app:publish
Request:
  POST /management/v1/applications/{application_id}/web_flows/{id}/activate

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely web-activations activate-web-flow --app app_xxx --id <id> --env prod
```

### `purchasely web-activations flow`

```text
Usage: purchasely web-activations flow [options]

Show the web-activation status (slug, public/sandbox URL) for a flow

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show the web-activation status (slug, public/sandbox URL) for a flow. Read-only, does not toggle activation.

Required scope: experiences.web2app:view
Request:
  GET /management/v1/applications/{application_id}/flows/{id}/web_activation

Example:
  purchasely web-activations flow --app app_xxx --id <id>
```

### `purchasely web-activations screen`

```text
Usage: purchasely web-activations screen [options]

Show the web-activation status (slug, public/sandbox URL) for a screen (paywall)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show the web-activation status (slug, public/sandbox URL) for a screen (paywall). Read-only, does not toggle activation.

Required scope: experiences.web2app:view
Request:
  GET /management/v1/applications/{application_id}/screens/{id}/web_activation

Example:
  purchasely web-activations screen --app app_xxx --id <id>
```

## web-flows

Management API: web-flows

### `purchasely web-flows`

```text
Usage: purchasely web-flows [options] [command]

Management API: web-flows

Options:
  -h, --help        display help for command

Commands:
  list [options]    List the web2app flows (funnels opened in the browser) of an application, newest
                    first
  create [options]  Create a web2app flow: a graph of steps, each displaying a screen, served as a
                    funnel in the browser once activated
  get [options]     Show a web2app flow with its published and draft content
  update [options]  Update a web2app flow (partial: omitted fields are left untouched)
  delete [options]  Discard (soft-delete) a web2app flow
  help [command]    display help for command
```

### `purchasely web-flows create`

```text
Usage: purchasely web-flows create [options]

Create a web2app flow: a graph of steps, each displaying a screen, served as a funnel in the browser
once activated

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Create a web2app flow: a graph of steps, each displaying a screen, served as a funnel in the browser once activated. Send the same content as draft_content to publish it right away.

Required scope: experiences.web2app:create
Request:
  POST /management/v1/applications/{application_id}/web_flows

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  app_flow (boolean) Fixed to false; any other value is refused.
  content (object) Published content. Send the same value as draft_content to publish on creation.
  default_presentation_store_public_id (string) Screen public id (pres_…) shown when a step cannot be resolved.
  draft_content (object) Draft content edited in the console before publishing.
  name (string, required) Display name shown in console.
  vendor_id (string, required) Stable machine identifier used by the SDK deeplink ply/flows/<vendor_id>; unique within the app.
  web_flow (boolean) Fixed to true; any other value is refused.

Example:
  purchasely web-flows create --app app_xxx --data @body.json --env prod
```

### `purchasely web-flows delete`

```text
Usage: purchasely web-flows delete [options]

Discard (soft-delete) a web2app flow

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Discard (soft-delete) a web2app flow. Refused while a draft or running A/B test uses it.

Required scope: experiences.web2app:delete
Request:
  DELETE /management/v1/applications/{application_id}/web_flows/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Example:
  purchasely web-flows delete --app app_xxx --id <id> --env prod
```

### `purchasely web-flows get`

```text
Usage: purchasely web-flows get [options]

Show a web2app flow with its published and draft content

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show a web2app flow with its published and draft content.

Required scope: experiences.web2app:view
Request:
  GET /management/v1/applications/{application_id}/web_flows/{id}

Example:
  purchasely web-flows get --app app_xxx --id <id>
```

### `purchasely web-flows list`

```text
Usage: purchasely web-flows list [options]

List the web2app flows (funnels opened in the browser) of an application, newest first

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id (required)
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List the web2app flows (funnels opened in the browser) of an application, newest first.

Required scope: experiences.web2app:view
Request:
  GET /management/v1/applications/{application_id}/web_flows

Example:
  purchasely web-flows list --app app_xxx
```

### `purchasely web-flows update`

```text
Usage: purchasely web-flows update [options]

Update a web2app flow (partial: omitted fields are left untouched)

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  --data <json>      request body: JSON, @file.json or - for stdin
  -h, --help         display help for command

Update a web2app flow (partial: omitted fields are left untouched). Sending content publishes it; draft_content only changes the draft edited in the console.

Required scope: experiences.web2app:edit
Request:
  PATCH /management/v1/applications/{application_id}/web_flows/{id}

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  app_flow (boolean) Fixed to false; any other value is refused.
  content (object) Published content. Send the same value as draft_content to publish.
  default_presentation_store_public_id (string) Screen public id (pres_…) shown when a step cannot be resolved.
  draft_content (object) Draft content edited in the console before publishing.
  name (string) Display name shown in console.
  vendor_id (string) Stable machine identifier used by the SDK deeplink ply/flows/<vendor_id>; unique within the app.
  web_flow (boolean) Fixed to true; any other value is refused.

Example:
  purchasely web-flows update --app app_xxx --id <id> --data @body.json --env prod
```

## webhook-deliveries

Management API: webhook-deliveries

### `purchasely webhook-deliveries`

```text
Usage: purchasely webhook-deliveries [options] [command]

Management API: webhook-deliveries

Options:
  -h, --help      display help for command

Commands:
  get [options]   Show the webhook delivery history/log for a single event
  list [options]  List webhook delivery events for a user
  help [command]  display help for command
```

### `purchasely webhook-deliveries get`

```text
Usage: purchasely webhook-deliveries get [options]

Show the webhook delivery history/log for a single event

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --id <value>       Public ID of the resource. [string, required]
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show the webhook delivery history/log for a single event.

Required scope: integrations.webhook_deliveries:view
Request:
  GET /management/v1/applications/{application_id}/webhook_deliveries/{id}

Example:
  purchasely webhook-deliveries get --app app_xxx --id <id>
```

### `purchasely webhook-deliveries list`

```text
Usage: purchasely webhook-deliveries list [options]

List webhook delivery events for a user

Options:
  --output <format>   json (default) or table
  --env <env>         prod or staging. Mandatory for writes
  --dry-run           print the request without sending it
  --yes               accept the confirmation prompt of a risky change
  --reason <text>     why you run this command (sent in the User-Agent)
  --rate <n>          requests per second, lower than the default
  --i-asked-backend   allow a rate above the hard cap
  --out <file>        write the JSON output to a file, or - for stdout
  --app <app_id>      application public id
  --user-id <value>   Public ID of the resource. [string, required]
  --fields <a,b>      Field selection for token economy: an explicit list of field names to include
                      in the response, or ["*"] for every field. Omit for this endpoint's own
                      default field set. [array of string] comma separated or repeated
  --page <value>      Page number (1-indexed). Defaults to 1. [integer]
  --per-page <value>  Results per page. Defaults to 25, capped at 100. [integer]
  -h, --help          display help for command

List webhook delivery events for a user.

Required scope: integrations.webhook_deliveries:view
Request:
  GET /management/v1/users/{user_id}/webhook_deliveries

Example:
  purchasely webhook-deliveries list --user-id <user-id>
```

## webhooks

Management API: webhooks

### `purchasely webhooks`

```text
Usage: purchasely webhooks [options] [command]

Management API: webhooks

Options:
  -h, --help        display help for command

Commands:
  list [options]    Show the per-event webhook delivery toggle config for an application
  update [options]  Update per-event webhook delivery toggles for an application
  help [command]    display help for command
```

### `purchasely webhooks list`

```text
Usage: purchasely webhooks list [options]

Show the per-event webhook delivery toggle config for an application

Options:
  --output <format>  json (default) or table
  --env <env>        prod or staging. Mandatory for writes
  --dry-run          print the request without sending it
  --yes              accept the confirmation prompt of a risky change
  --reason <text>    why you run this command (sent in the User-Agent)
  --rate <n>         requests per second, lower than the default
  --i-asked-backend  allow a rate above the hard cap
  --out <file>       write the JSON output to a file, or - for stdout
  --app <app_id>     application public id (required)
  --fields <a,b>     Field selection for token economy: an explicit list of field names to include
                     in the response, or ["*"] for every field. Omit for this endpoint's own default
                     field set. [array of string] comma separated or repeated
  -h, --help         display help for command

Show the per-event webhook delivery toggle config for an application.

Required scope: integrations.webhooks:view
Request:
  GET /management/v1/applications/{application_id}/webhooks

Example:
  purchasely webhooks list --app app_xxx
```

### `purchasely webhooks update`

```text
Usage: purchasely webhooks update [options]

Update per-event webhook delivery toggles for an application

Options:
  --output <format>        json (default) or table
  --env <env>              prod or staging. Mandatory for writes
  --dry-run                print the request without sending it
  --yes                    accept the confirmation prompt of a risky change
  --reason <text>          why you run this command (sent in the User-Agent)
  --rate <n>               requests per second, lower than the default
  --i-asked-backend        allow a rate above the hard cap
  --out <file>             write the JSON output to a file, or - for stdout
  --app <app_id>           application public id (required)
  --fields <a,b>           Field selection for token economy: an explicit list of field names to
                           include in the response, or ["*"] for every field. Omit for this
                           endpoint's own default field set. [array of string] comma separated or
                           repeated
  --data <json>            request body: JSON, @file.json or - for stdin
  --confirm-token <token>  token from a CONFIRMATION_REQUIRED response
  -h, --help               display help for command

Update per-event webhook delivery toggles for an application. Always requires confirmation.

Required scope: integrations.webhooks:edit
Request:
  PUT /management/v1/applications/{application_id}/webhooks

A write needs --env prod or --env staging. Run it with --dry-run first.

Body (required): --data '<json>' or --data @file.json or --data -
  confirm_token (string) Confirmation token from a prior confirmation_required response; required to actually apply the update.
  events (object) Event-name -> {enabled: true/false} map. Events omitted or set to false are disabled, except always-on default events, which stay enabled regardless.

Confirmation flow: a risky change first returns the impact and a confirm_token (body).
  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.

Example:
  purchasely webhooks update --app app_xxx --data @body.json --env prod
```
